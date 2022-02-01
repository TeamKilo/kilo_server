use std::collections::HashMap;
use std::sync::Mutex;
use actix_web::Result;

#[derive(Copy, Clone)] pub struct GameId(u128);
#[derive(Copy, Clone)] pub struct SessionId(u128);

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

pub trait GameAdapter: Send {
    fn new(game_id: GameId) -> Self where Self: Sized;
    fn add_player(&mut self, session_id: SessionId) -> Result<()>;
    fn play_move(&mut self, session_id: SessionId, encoded_move: &str) -> Result<()>;
    fn get_encoded_state(&self) -> String;
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
