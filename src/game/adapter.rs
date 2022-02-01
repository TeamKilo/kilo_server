use serde_json::Value;
use serde::{Serialize, Deserialize};
use std::vec::Vec;
use crate::game::{GameId, SessionId};
use actix_web::{ResponseError, Result};
use std::fmt;
use std::fmt::{Display, Formatter};
use actix_web::http::StatusCode;

#[derive(Debug, Clone)]
pub enum GameAdapterError {
    PlayerLimitExceeded(usize),
    InvalidGameState(State)
}

impl fmt::Display for GameAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GameAdapterError::PlayerLimitExceeded(size) =>
                write!(f, "limit of {} players exceeded", size),
            GameAdapterError::InvalidGameState(state) =>
                write!(f, "invalid operation for state {}", state)
        }
    }
}

impl ResponseError for GameAdapterError {
    fn status_code(&self) -> StatusCode {
        StatusCode::CONFLICT
    }
}

#[derive(Serialize, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Waiting,
    InProgress,
    Ended,
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            State::Waiting => write!(f, "waiting"),
            State::InProgress => write!(f, "in_progress"),
            State::Ended => write!(f, "ended")
        }
    }
}

#[derive(Serialize)]
pub struct GenericGameState {
    pub players: Vec<String>,
    pub state: State,
    pub can_move: Vec<String>,
    pub payload: Value,
}

#[derive(Deserialize)]
pub struct GenericGameMove {
    player: String,
    payload: Value,
}

pub trait GameAdapter: Send {
    fn new(game_id: GameId) -> Self where Self: Sized;
    fn add_player(&mut self, username: String) -> Result<()>;
    fn play_move(&mut self, game_move: GenericGameMove) -> Result<()>;
    fn get_encoded_state(&self) -> Result<GenericGameState>;
}
