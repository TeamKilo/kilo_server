pub mod adapter;
pub mod connect4;

use crate::game::adapter::{GenericGameMove, GenericGameState};
use crate::game::ValidationError::ParseIdError;
use actix_web::http::StatusCode;
use actix_web::{Error, ResponseError, Result};
use adapter::GameAdapter;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::ops::DerefMut;
use std::sync::Mutex;
use serde_json::Value;

/// ValidationError
#[derive(Debug, Clone)]
pub enum ValidationError {
    ParseIdError(String),
    NoSuchGameError(String),
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
}

impl fmt::Display for GameManagerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GameManagerError::GameIdDoesNotExist(game_id) => {
                write!(f, "game corresponding to {} does not exist", game_id)
            }
            GameManagerError::SessionIdDoesNotExist(session_id) => {
                write!(f, "session corresponding to {} does not exist", session_id)
            }
        }
    }
}

impl ResponseError for GameManagerError {
    fn status_code(&self) -> StatusCode {
        match self {
            GameManagerError::GameIdDoesNotExist(_) => StatusCode::NOT_FOUND,
            GameManagerError::SessionIdDoesNotExist(_) => StatusCode::NOT_FOUND,
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

pub struct GameManager {
    games: HashMap<GameId, Mutex<Box<dyn GameAdapter>>>,
    sessions: HashMap<SessionId, Session>,
}

impl GameManager {
    pub fn new() -> Self {
        GameManager {
            games: HashMap::new(),
            sessions: HashMap::new(),
        }
    }

    pub fn create_game(
        &mut self,
        game: impl FnOnce(GameId) -> Box<dyn GameAdapter>,
    ) -> Result<GameId> {
        let mut game_id;
        loop {
            game_id = GameId::new();
            if !self.games.contains_key(&game_id) {
                break;
            }
        }

        let game_adapter = game(game_id);
        self.games.insert(game_id, Mutex::new(game_adapter));
        Ok(game_id)
    }

    pub fn receive_join(&mut self, game_id: GameId, username: String) -> Result<SessionId> {
        if !self.games.contains_key(&game_id) {
            return Err(actix_web::Error::from(
                GameManagerError::GameIdDoesNotExist(game_id),
            ));
        }

        let mut session_id;
        loop {
            session_id = SessionId::new();
            if !self.sessions.contains_key(&session_id) {
                break;
            }
        }

        let session = Session::new(username.clone(), game_id);
        self.sessions.insert(session_id, session);

        let game_adapter_mutex = self.get_game_adapter(game_id)?;
        let mut mutex_guard = game_adapter_mutex.lock().unwrap();
        let mut game_adapter = mutex_guard.deref_mut();
        game_adapter.add_player(username);

        Ok(session_id)
    }

    pub fn receive_move(&self, session_id: SessionId, encoded_move: Value) -> Result<()> {
        // Deleted sessionId because it corresponds to the "player" field in GenericGameMove
        todo!()
    }

    pub fn get_state(&self, game_id: GameId) -> Result<GenericGameState> {
        todo!()
    }

    fn get_game_adapter(&self, game_id: GameId) -> Result<&Mutex<Box<dyn GameAdapter>>> {
        match self.games.get(&game_id) {
            Some(x) => Ok(x),
            None => Err(actix_web::Error::from(
                GameManagerError::GameIdDoesNotExist(game_id),
            )),
        }
    }

    fn get_session(&self, session_id: SessionId) -> Result<&Session> {
        match self.sessions.get(&session_id) {
            Some(x) => Ok(x),
            None => Err(actix_web::Error::from(
                GameManagerError::SessionIdDoesNotExist(session_id),
            )),
        }
    }
}
