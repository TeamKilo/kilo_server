pub mod adapter;
pub mod connect4;

use crate::game::adapter::{
    GameAdapter, GameAdapterError, GameAdapterErrorType, GenericGameMove, GenericGameState, Stage,
};
use crate::notify::Subscription;
use actix_web::http::StatusCode;
use actix_web::{Error, ResponseError, Result};
use dashmap::mapref::entry::Entry;
use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use derive_more::Display;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ops::DerefMut;
use std::sync::Mutex;

fn encode_id(bytes: &[u8]) -> String {
    base32::encode(base32::Alphabet::RFC4648 { padding: false }, bytes)
}

fn decode_id(data: &str) -> Option<Vec<u8>> {
    base32::decode(base32::Alphabet::RFC4648 { padding: false }, data)
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Display)]
#[display(fmt = "game_{}", "encode_id(_0)")]
pub struct GameId([u8; 4]);

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Display)]
#[display(fmt = "session_{}", "encode_id(_0)")]
pub struct SessionId([u8; 16]);

fn new_parse_id_error(id: &String) -> Error {
    actix_web::Error::from(GameManagerError::InvalidId(id.clone()))
}

fn validate_id(id: &String, prefix: &str) -> Result<Vec<u8>, Error> {
    if !id.starts_with(prefix) {
        return Err(new_parse_id_error(id));
    }

    decode_id(&id[prefix.len()..]).ok_or(new_parse_id_error(id))
}

impl GameId {
    pub fn new() -> Self {
        GameId(rand::thread_rng().gen())
    }

    // Added for API to create a GameId object to input to the GameManager
    pub fn from(id: &String) -> Result<Self> {
        let vec = validate_id(id, "game_")?;
        let bytes = TryInto::<[u8; 4]>::try_into(vec).or(Err(new_parse_id_error(id)))?;
        Ok(GameId(bytes))
    }
}

impl SessionId {
    pub fn new() -> Self {
        SessionId(rand::thread_rng().gen())
    }

    // Added for API to create a SessionId object
    pub fn from(id: &String) -> Result<Self> {
        let vec = validate_id(id, "session_")?;
        let bytes = TryInto::<[u8; 16]>::try_into(vec).or(Err(new_parse_id_error(id)))?;
        Ok(SessionId(bytes))
    }
}

const MAX_USERNAME_LENGTH: usize = 12;

#[derive(Debug, Clone, Display)]
pub enum InvalidUsernameReason {
    #[display(fmt = "already in game {}", _0)]
    AlreadyInGame(GameId),
    #[display(fmt = "too short")]
    TooShort,
    #[display(fmt = "longer than {} characters", MAX_USERNAME_LENGTH)]
    TooLong,
}

#[derive(Debug, Clone, Display)]
pub enum GameManagerError {
    #[display(fmt = "invalid id: {}", _0)]
    InvalidId(String),
    #[display(fmt = "game type {} does not exist", _0)]
    NoSuchGameType(String),
    #[display(fmt = "no game with id {}", _0)]
    GameNotFound(GameId),
    #[display(fmt = "no session with id {}", _0)]
    SessionNotFound(SessionId),
    #[display(fmt = "invalid username ({}): {}", reason, username)]
    InvalidUsername {
        username: String,
        reason: InvalidUsernameReason,
    },
    #[display(fmt = "session {} does not match game id {}", session_id, game_id)]
    GameIdDoesNotMatch {
        game_id: GameId,
        session_id: SessionId,
    },
}

impl ResponseError for GameManagerError {
    fn status_code(&self) -> StatusCode {
        match self {
            GameManagerError::GameNotFound(_) => StatusCode::NOT_FOUND,
            GameManagerError::SessionNotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::BAD_REQUEST,
        }
    }
}

pub struct Session {
    username: String,
    game_id: GameId,
}

impl Session {
    pub fn new(username: String, game_id: GameId) -> Self {
        Session { username, game_id }
    }
}

type GameAdapterMutex = Mutex<Box<dyn GameAdapter>>;

#[derive(Serialize)]
pub struct GameSummary {
    pub game_id: String,
    pub game_type: String,
    pub players: Vec<String>,
    pub stage: String,
}

pub struct GameManager {
    games: DashMap<GameId, Mutex<Box<dyn GameAdapter>>>,
    sessions: DashMap<SessionId, Session>,
}

impl GameManager {
    pub fn new() -> Self {
        GameManager {
            games: DashMap::new(),
            sessions: DashMap::new(),
        }
    }

    pub fn create_game(&self, game: impl FnOnce(GameId) -> Box<dyn GameAdapter>) -> Result<GameId> {
        loop {
            let game_id = GameId::new();
            if let entry @ Entry::Vacant(_) = self.games.entry(game_id) {
                entry.or_insert(Mutex::new(game(game_id)));
                break Ok(game_id);
            }
        }
    }

    pub fn receive_join(&self, game_id: GameId, username: String) -> Result<SessionId> {
        let game_adapter_mutex = GameManager::get_game_adapter_mutex(&self.games, game_id)?;
        let mut mutex_guard = game_adapter_mutex.lock().unwrap();
        let game_adapter = mutex_guard.deref_mut();

        if game_adapter.get_stage() != Stage::Waiting {
            return Err(GameAdapterError::actix_err(
                game_id,
                GameAdapterErrorType::InvalidGameStage(game_adapter.get_stage()),
            ));
        }

        if username.is_empty() {
            return Err(actix_web::Error::from(GameManagerError::InvalidUsername {
                username,
                reason: InvalidUsernameReason::TooShort,
            }));
        }

        if username.len() > MAX_USERNAME_LENGTH {
            return Err(actix_web::Error::from(GameManagerError::InvalidUsername {
                username,
                reason: InvalidUsernameReason::TooLong,
            }));
        }

        if game_adapter.has_player(&username) {
            return Err(actix_web::Error::from(GameManagerError::InvalidUsername {
                username,
                reason: InvalidUsernameReason::AlreadyInGame(game_id),
            }));
        }

        game_adapter.add_player(username.clone())?;

        loop {
            let session_id = SessionId::new();
            if let entry @ Entry::Vacant(_) = self.sessions.entry(session_id) {
                let session = Session::new(username, game_id);
                entry.or_insert(session);
                break Ok(session_id);
            }
        }
    }

    pub fn receive_move(
        &self,
        game_id: GameId,
        session_id: SessionId,
        encoded_move: Value,
    ) -> Result<()> {
        let session = GameManager::get_session(&self.sessions, session_id)?;
        if session.game_id != game_id {
            return Err(actix_web::Error::from(
                GameManagerError::GameIdDoesNotMatch {
                    game_id,
                    session_id,
                },
            ));
        }

        let game_adapter_mutex = GameManager::get_game_adapter_mutex(&self.games, game_id)?;
        let mut game_adapter = game_adapter_mutex.lock().unwrap();

        game_adapter.play_move(GenericGameMove {
            player: session.username.clone(),
            payload: encoded_move,
        })
    }

    pub fn get_state(&self, game_id: GameId) -> Result<GenericGameState> {
        let game_adapter_mutex = GameManager::get_game_adapter_mutex(&self.games, game_id)?;
        let game_adapter = game_adapter_mutex.lock().unwrap();

        let mut state = game_adapter.get_encoded_state()?;

        if let serde_json::Value::Object(ref mut map) = state.payload {
            map.insert(
                String::from("game_type"),
                serde_json::to_value(game_adapter.get_type()).unwrap(),
            );

            return Ok(state);
        }

        panic!("State payload must be a Serde object")
    }

    pub fn list_games(&self) -> Vec<GameSummary> {
        self.games
            .iter()
            .map(|x| {
                let game_adapter = x.value().lock().unwrap();
                let state = game_adapter.get_encoded_state().unwrap();
                GameSummary {
                    game_id: x.key().to_string(),
                    game_type: String::from(game_adapter.get_type()),
                    players: state.players,
                    stage: state.stage.to_string(),
                }
            })
            .collect()
    }

    pub fn subscribe(&self, game_id: GameId) -> Result<Subscription> {
        Ok(GameManager::get_game_adapter_mutex(&self.games, game_id)?
            .lock()
            .unwrap()
            .get_notifier()
            .subscribe())
    }

    fn get_game_adapter_mutex<'a>(
        games: &DashMap<GameId, GameAdapterMutex>,
        game_id: GameId,
    ) -> Result<Ref<GameId, GameAdapterMutex>> {
        match games.get(&game_id) {
            Some(x) => Ok(x),
            None => Err(actix_web::Error::from(GameManagerError::GameNotFound(
                game_id,
            ))),
        }
    }

    fn get_session<'a>(
        sessions: &DashMap<SessionId, Session>,
        session_id: SessionId,
    ) -> Result<Ref<SessionId, Session>> {
        match sessions.get(&session_id) {
            Some(x) => Ok(x),
            None => Err(actix_web::Error::from(GameManagerError::SessionNotFound(
                session_id,
            ))),
        }
    }
}
