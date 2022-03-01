pub mod adapter;
pub mod connect4;
pub mod search;

use crate::game::adapter::{
    GameAdapter, GameAdapterError, GameAdapterErrorType, GenericGameMove, GenericGameState, Stage,
};
use crate::game::search::{GameSummary, SearchEngine, SearchOptions};
use crate::notify::Subscription;
use actix_web::http::StatusCode;
use actix_web::{ResponseError, Result};
use chrono::{DateTime, Duration, Utc};
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use derive_more::Display;
use rand::Rng;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::ops::DerefMut;
use std::sync::Mutex;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum GameType {
    #[serde(rename = "connect_4")]
    Connect4,
}

fn encode_id(bytes: &[u8]) -> String {
    base32::encode(base32::Alphabet::RFC4648 { padding: false }, bytes)
}

fn decode_id(data: &str) -> Option<Vec<u8>> {
    base32::decode(base32::Alphabet::RFC4648 { padding: false }, data)
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Display)]
#[display(fmt = "game_{}", "encode_id(_0)")]
pub struct GameId([u8; 4]);

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Display)]
#[display(fmt = "session_{}", "encode_id(_0)")]
pub struct SessionId([u8; 16]);

fn new_parse_id_error<E>(id: &str) -> E
where
    E: de::Error,
{
    de::Error::custom(format!("invalid id: {}", id))
}

fn validate_id<E>(id: &str, prefix: &str) -> Result<Vec<u8>, E>
where
    E: de::Error,
{
    if !id.starts_with(prefix) {
        return Err(new_parse_id_error(id));
    }

    decode_id(&id[prefix.len()..]).ok_or(new_parse_id_error(id))
}

impl GameId {
    pub fn new() -> Self {
        GameId(rand::thread_rng().gen())
    }
}

impl Serialize for GameId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for GameId {
    fn deserialize<D>(deserializer: D) -> Result<GameId, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct GameIdVisitor;

        impl<'de> de::Visitor<'de> for GameIdVisitor {
            type Value = GameId;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("game id")
            }

            fn visit_str<E>(self, v: &str) -> Result<GameId, E>
            where
                E: de::Error,
            {
                let vec = validate_id(v, "game_")?;
                let bytes = TryInto::<[u8; 4]>::try_into(vec).map_err(|_| new_parse_id_error(v))?;
                Ok(GameId(bytes))
            }
        }

        deserializer.deserialize_string(GameIdVisitor)
    }
}

impl SessionId {
    pub fn new() -> Self {
        SessionId(rand::thread_rng().gen())
    }
}

impl Serialize for SessionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SessionId {
    fn deserialize<D>(deserializer: D) -> Result<SessionId, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SessionIdVisitor;

        impl<'de> de::Visitor<'de> for SessionIdVisitor {
            type Value = SessionId;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("session id")
            }

            fn visit_str<E>(self, v: &str) -> Result<SessionId, E>
            where
                E: de::Error,
            {
                let vec = validate_id(v, "session_")?;
                let bytes =
                    TryInto::<[u8; 16]>::try_into(vec).map_err(|_| new_parse_id_error(v))?;
                Ok(SessionId(bytes))
            }
        }

        deserializer.deserialize_string(SessionIdVisitor)
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
    #[display(fmt = "no game with id {}", _0)]
    GameNotFound(GameId),
    #[display(fmt = "no session with id {}", _0)]
    SessionNotFound(SessionId),
    #[display(fmt = "invalid username ({}): {}", reason, username)]
    InvalidUsername {
        username: String,
        reason: InvalidUsernameReason,
    },
    #[display(fmt = "page must be at least one")]
    InvalidPage,
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
}

impl Session {
    pub fn new(username: String) -> Self {
        Session { username }
    }
}

pub struct Game {
    adapter: Box<dyn GameAdapter>,
    sessions: HashMap<SessionId, Session>,
    last_update: DateTime<Utc>,
}

pub struct GameManager {
    games: DashMap<GameId, Mutex<Game>>,
}

impl GameManager {
    pub fn new() -> Self {
        GameManager { games: DashMap::new() }
    }

    pub fn create_game(&self, factory: impl FnOnce(GameId) -> Box<dyn GameAdapter>) -> Result<GameId> {
        self.gc_games();

        loop {
            let game_id = GameId::new();
            if let entry @ Entry::Vacant(_) = self.games.entry(game_id) {
                entry.or_insert(Mutex::new(Game {
                    adapter: factory(game_id),
                    sessions: HashMap::new(),
                    last_update: chrono::offset::Utc::now()
                }));
                break Ok(game_id);
            }
        }
    }

    pub fn receive_join(&self, game_id: GameId, username: String) -> Result<SessionId> {
        let mutex = self.games.get(&game_id)
            .ok_or_else(|| { GameManager::game_not_found(game_id) })?;
        let mut mutex_guard = mutex.lock().unwrap();
        let game_adapter = mutex_guard.adapter.deref_mut();

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
        mutex_guard.last_update = chrono::offset::Utc::now();

        let new_session = Session::new(username);
        loop {
            let session_id = SessionId::new();

            if !mutex_guard.sessions.contains_key(&session_id) {
                mutex_guard.sessions.insert(session_id, new_session);
                return Ok(session_id)
            }
        }
    }

    pub fn receive_move(
        &self,
        game_id: GameId,
        session_id: SessionId,
        encoded_move: Value,
    ) -> Result<()> {
        let mutex = self.games.get(&game_id)
            .ok_or_else(|| { GameManager::game_not_found(game_id) })?;
        let mut mutex_guard = mutex.lock().unwrap();
        let username = mutex_guard.sessions.get(&session_id)
            .ok_or_else(|| { GameManager::session_not_found(session_id) })?.username.clone();

        mutex_guard.adapter.deref_mut().play_move(GenericGameMove {
            player: username,
            payload: encoded_move,
        })?;
        mutex_guard.last_update = chrono::offset::Utc::now();

        Ok(())
    }

    pub fn get_state(&self, game_id: GameId) -> Result<GenericGameState> {
        let mutex = self.games.get(&game_id)
            .ok_or_else(|| { GameManager::game_not_found(game_id) })?;
        let mut mutex_guard = mutex.lock().unwrap();
        let game_adapter = mutex_guard.adapter.deref_mut();

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

    pub fn list_games(&self, options: SearchOptions) -> Result<Vec<GameSummary>> {
        SearchEngine::apply(
            self.games.iter().map(|x| {
                let mut guard = x.value().lock().unwrap();
                let game_adapter = guard.adapter.deref_mut();
                let GenericGameState { players, stage, .. } =
                    game_adapter.get_encoded_state().unwrap();
                GameSummary {
                    game_id: *x.key(),
                    game_type: game_adapter.get_type(),
                    players,
                    stage,
                    last_updated: guard.last_update,
                }
            }),
            &options,
        )
    }

    pub fn subscribe(&self, game_id: GameId) -> Result<Subscription> {
        Ok(self.games.get(&game_id)
            .ok_or_else(|| { GameManager::game_not_found(game_id) })?
            .lock()
            .unwrap()
            .adapter.deref_mut()
            .get_notifier()
            .subscribe())
    }

    fn gc_games(&self) {
        let now = chrono::offset::Utc::now();
        self.games.retain(|_, v| {
            match v.try_lock() {
                Ok(guard) => guard.last_update + Duration::minutes(5) >= now,
                Err(_) => true
            }
        })
    }

    fn game_not_found(game_id: GameId) -> actix_web::Error {
        actix_web::Error::from(GameManagerError::GameNotFound(game_id))
    }

    fn session_not_found(session_id: SessionId) -> actix_web::Error {
        actix_web::Error::from(GameManagerError::SessionNotFound(session_id))
    }
}
