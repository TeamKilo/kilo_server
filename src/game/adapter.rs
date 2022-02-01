use serde_json::Value;
use serde::{Serialize, Deserialize};
use std::vec::Vec;
use crate::game::{GameId, SessionId};
use actix_web::Result;


#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Waiting,
    InProgress,
    Ended,
}

#[derive(Serialize)]
pub struct GenericGameState {
    players: Vec<String>,
    state: State,
    can_move: Vec<String>,
    payload: Value,
}

#[derive(Deserialize)]
pub struct GenericGameMove {
    session_id: SessionId,
    payload: Value,
}

pub trait GameAdapter: Send {
    fn new(game_id: GameId) -> Self where Self: Sized;
    fn add_player(&mut self, session_id: SessionId) -> Result<()>;
    fn play_move(&mut self, session_id: SessionId, encoded_move: &str) -> Result<()>;
    fn get_encoded_state(&self) -> String;
}
