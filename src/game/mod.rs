pub mod adapter;
pub mod connect4;

use crate::game::adapter::{GameAdapter, GenericGameMove, GenericGameState};
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
use std::fmt;
use std::fmt::Formatter;
use std::ops::DerefMut;
use std::sync::Mutex;

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct GameId(u128);

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct SessionId(u128);

fn validate_id(id: &String, prefix: &str) -> Result<u128, Error> {
    let parse_id_error = || actix_web::Error::from(GameManagerError::ParseIdError(id.clone()));

    if !id.starts_with(prefix) {
        return Err(parse_id_error());
    }

    let base64 = &id[prefix.len()..];
    let id_vec =
        base64::decode_config(base64, base64::URL_SAFE_NO_PAD).or(Err(parse_id_error()))?;

    Ok(u128::from_be_bytes(
        id_vec.as_slice().try_into().or(Err(parse_id_error()))?,
    ))
}

impl GameId {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let id: u128 = rng.gen();
        GameId(id)
    }

    // Added for API to create a GameId object to input to the GameManager
    pub fn from(id: &String) -> Result<Self> {
        validate_id(id, "game_").and_then(|id| Ok(GameId(id)))
    }
}

impl fmt::Display for GameId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "game_{}",
            base64::encode_config(self.0.to_be_bytes(), base64::URL_SAFE_NO_PAD)
        )
    }
}

impl SessionId {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let id: u128 = rng.gen();
        SessionId(id)
    }

    // Added for API to create a SessionId object
    pub fn from(id: &String) -> Result<Self> {
        validate_id(id, "session_").and_then(|id| Ok(Self(id)))
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "session_{}",
            base64::encode_config(self.0.to_be_bytes(), base64::URL_SAFE_NO_PAD)
        )
    }
}

#[derive(Debug, Clone, Display)]
pub enum GameManagerError {
    #[display(fmt = "invalid id: {}", _0)]
    ParseIdError(String),
    #[display(fmt = "game {} does not exist", _0)]
    NoSuchGameError(String),
    #[display(fmt = "no game with id {}", _0)]
    GameIdDoesNotExist(GameId),
    #[display(fmt = "no session with id {}", _0)]
    SessionIdDoesNotExist(SessionId),
    #[display(fmt = "username {} already in game with id {}", username, game_id)]
    DuplicateUsername { username: String, game_id: GameId },
    #[display(fmt = "session {} does not match game id {}", session_id, game_id)]
    GameIdDoesNotMatch {
        game_id: GameId,
        session_id: SessionId,
    },
}

impl ResponseError for GameManagerError {
    fn status_code(&self) -> StatusCode {
        match self {
            GameManagerError::ParseIdError(_) => StatusCode::BAD_REQUEST,
            GameManagerError::NoSuchGameError(_) => StatusCode::BAD_REQUEST,
            GameManagerError::GameIdDoesNotExist(_) => StatusCode::NOT_FOUND,
            GameManagerError::SessionIdDoesNotExist(_) => StatusCode::NOT_FOUND,
            GameManagerError::DuplicateUsername { .. } => StatusCode::CONFLICT,
            GameManagerError::GameIdDoesNotMatch { .. } => StatusCode::BAD_REQUEST,
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
        if game_adapter.has_player(&username) {
            return Err(actix_web::Error::from(
                GameManagerError::DuplicateUsername { username, game_id },
            ));
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
        GameManager::get_game_adapter_mutex(&self.games, game_id)?
            .lock()
            .unwrap()
            .get_encoded_state()
    }

    pub fn list_games(&self) -> Result<Vec<String>> {
        let mut games: Vec<String> = vec![];
        for x in self.games.iter() {
            games.push(x.key().to_string());
        }
        Ok(games)
    }

    pub fn wait_for_update(&self, game_id: GameId) -> Result<Subscription> {
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
            None => Err(actix_web::Error::from(
                GameManagerError::GameIdDoesNotExist(game_id),
            )),
        }
    }

    fn get_session<'a>(
        sessions: &DashMap<SessionId, Session>,
        session_id: SessionId,
    ) -> Result<Ref<SessionId, Session>> {
        match sessions.get(&session_id) {
            Some(x) => Ok(x),
            None => Err(actix_web::Error::from(
                GameManagerError::SessionIdDoesNotExist(session_id),
            )),
        }
    }
}
