pub mod adapter;
pub mod connect4;

use crate::game::adapter::{GenericGameMove, GenericGameState};
use crate::game::ValidationError::ParseIdError;
use actix_web::http::StatusCode;
use actix_web::{Error, ResponseError, Result};
use adapter::GameAdapter;
use dashmap::mapref::entry::Entry;
use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use game::adapter::GameAdapter;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::ops::DerefMut;
use std::sync::Mutex;
use tokio::sync::broadcast;

/// ValidationError
#[derive(Debug, Clone)]
pub enum ValidationError {
    ParseIdError(String),
    NoSuchGameError(String),
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::ParseIdError(id) => write!(f, "ID {} is Invalid", id),
            ValidationError::NoSuchGameError(game) => write!(f, "Game {} not found", game),
        }
    }
}

impl ResponseError for ValidationError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct GameId(u128);

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct SessionId(u128);

impl GameId {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let id: u128 = rng.gen();
        GameId(id)
    }
    // Added for API to create a GameId object to input to the GameManager
    pub fn from(id: &String) -> Result<Self> {
        GameId::validate_id(id).and_then(|id| Ok(GameId(id)))
    }

    /// validate_id
    fn validate_id(game_id: &String) -> Result<u128, Error> {
        if !game_id.starts_with("game_") {
            return Err(actix_web::Error::from(ParseIdError(game_id.clone())));
        }
        let game_id_int = u128::from_str_radix(&game_id[5..], 10);
        match game_id_int {
            Ok(value) => Ok(value),
            Err(_) => {
                return Err(actix_web::Error::from(ValidationError::ParseIdError(
                    game_id.clone(),
                )))
            } // TODO: need custom error
        }
    }
}

impl fmt::Display for GameId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "game_{}", self.0)
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
        SessionId::validate_id(id).and_then(|id| Ok(Self(id)))
    }

    /// validate_id
    fn validate_id(session_id: &String) -> Result<u128, Error> {
        if !session_id.starts_with("session_") {
            return Err(actix_web::Error::from(ParseIdError(session_id.clone())));
        }
        let session_id_int = u128::from_str_radix(&session_id[8..], 10);
        match session_id_int {
            Ok(value) => Ok(value),
            Err(_) => {
                return Err(actix_web::Error::from(ValidationError::ParseIdError(
                    session_id.clone(),
                )))
            } // TODO: need custom error
        }
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "session_{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub enum GameManagerError {
    GameIdDoesNotExist(GameId),
    SessionIdDoesNotExist(SessionId),
    DuplicateUsername { username: String, game_id: GameId },
}

impl fmt::Display for GameManagerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GameManagerError::GameIdDoesNotExist(game_id) => {
                write!(f, "Game corresponding to {} does not exist", game_id)
            }
            GameManagerError::SessionIdDoesNotExist(session_id) => {
                write!(f, "Session corresponding to {} does not exist", session_id)
            }
            GameManagerError::DuplicateUsername { username, game_id } => {
                write!(
                    f,
                    "Player with username {} already in game corresponding to {}",
                    username, game_id
                )
            }
        }
    }
}

impl ResponseError for GameManagerError {
    fn status_code(&self) -> StatusCode {
        match self {
            GameManagerError::GameIdDoesNotExist(_) => StatusCode::NOT_FOUND,
            GameManagerError::SessionIdDoesNotExist(_) => StatusCode::NOT_FOUND,
            GameManagerError::DuplicateUsername { .. } => StatusCode::CONFLICT,
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
    pub fn create_game(
        &mut self,
        game: impl FnOnce(GameId) -> Box<dyn GameAdapter>,
    ) -> Result<GameId> {
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

    pub fn receive_move(&self, session_id: SessionId, encoded_move: Value) -> Result<()> {
        let session = GameManager::get_session(&self.sessions, session_id)?;
        let game_adapter_mutex = GameManager::get_game_adapter_mutex(&self.games, session.game_id)?;
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

    pub fn wait_for_update(&self, game_id: GameId) -> Result<broadcast::Receiver<()>> {
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
