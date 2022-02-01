mod adapter;
mod connect4;

use adapter::GameAdapter;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::sync::Mutex;
use serde::{Serialize, Deserialize};
use actix_web::{ResponseError, Result};
use actix_web::http::StatusCode;
use rand::Rng;

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)] pub struct GameId(u128);
#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)] pub struct SessionId(u128);

impl GameId {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let id: u128 = rng.gen();
        GameId(id)
    }
}

impl fmt::Display for GameId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Game{}", self.0)
    }
}

impl SessionId {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let id: u128 = rng.gen();
        SessionId(id)
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Session{}", self.0)
    }
}

#[derive(Debug, Clone)] pub struct GameIdDoesNotExistError(GameId);

impl fmt::Display for GameIdDoesNotExistError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "game corresponding to {} does not exist", self.0)
    }
}

impl ResponseError for GameIdDoesNotExistError {
    fn status_code(&self) -> StatusCode {
        StatusCode::NOT_FOUND
    }
}

pub struct Session {
    username: String,
    game_id: GameId,
}

impl Session {
    pub fn new(username: String, game_id: GameId) -> Self {
        Session {
            username,
            game_id
        }
    }
}

pub struct GameManager {
    games: HashMap<GameId, Mutex<Box<dyn GameAdapter>>>,
    sessions: HashMap<SessionId, Session>,
}

impl GameManager {
    pub fn new() -> Self {
        GameManager { games: HashMap::new(), sessions: HashMap::new() }
    }

    pub fn create_game(&mut self, game: impl FnOnce(GameId) -> Box<dyn GameAdapter>) -> Result<GameId> {
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
        let _ = self.get_game_adapter(game_id);

        let mut session_id;
        loop {
            session_id = SessionId::new();
            if !self.sessions.contains_key(&session_id) {
                break;
            }
        }

        let session = Session::new(username, game_id);
        self.sessions.insert(session_id, session);

        Ok(session_id)
    }

    // pub fn receive_move(&self, session_id: SessionId, encoded_move: ?) -> Result<()> {
    //     todo!()
    // }
    //
    // pub fn get_state(&self, game_id: GameId) -> Result<?> {
    //     todo!()
    // }

    fn get_game_adapter(&self, game_id: GameId) -> Result<&Mutex<Box<dyn GameAdapter>>> {
        match self.games.get(&game_id) {
            Some(x) => Ok(x),
            None => Err(actix_web::Error::from(GameIdDoesNotExistError(game_id)))
        }
    }
}
