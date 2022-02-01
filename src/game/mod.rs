mod adapter;

use adapter::GameAdapter;
use std::collections::HashMap;
use std::sync::Mutex;
use actix_web::Result;
use serde::{Serialize, Deserialize};

#[derive(Copy, Clone, Serialize, Deserialize)] pub struct GameId(u128);
#[derive(Copy, Clone, Serialize, Deserialize)] pub struct SessionId(u128);

impl GameId {
    pub fn new() {
        todo!()
    }
}

impl SessionId {
    pub fn new() {
        todo!()
    }
}

pub struct GameManager {
    games: HashMap<GameId, Mutex<Box<dyn GameAdapter>>>,
    sessions: HashMap<SessionId, GameId>,
}

impl GameManager {
    pub fn new() -> Self {
        GameManager { games: HashMap::new(), sessions: HashMap::new() }
    }

    // pub fn create_game(&mut self, game: impl FnOnce(GameId) -> dyn GameAdapter) -> Result<GameId> {
    //     todo!()
    // }
    //
    // pub fn receive_join(&self, game_id: GameId) -> Result<SessionId> {
    //     todo!()
    // }
    //
    // pub fn receive_move(&self, session_id: SessionId, encoded_move: ?) -> Result<()> {
    //     todo!()
    // }
    //
    // pub fn get_state(&self, game_id: GameId) -> Result<?> {
    //     todo!()
    // }
}
