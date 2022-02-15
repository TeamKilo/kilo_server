use crate::game::GameId;
use crate::notify::Notifier;
use actix_web::http::StatusCode;
use actix_web::{ResponseError, Result};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::vec::Vec;

#[derive(Debug, Clone, Display)]
pub enum GameAdapterError {
    #[display(fmt = "limit of {} players exceeded", _0)]
    PlayerLimitExceeded(usize),
    #[display(fmt = "invalid operation for state {}", _0)]
    InvalidGameState(State),
    #[display(fmt = "user {} cannot move at the moment", _0)]
    InvalidPlayer(String),
    #[display(fmt = "invalid move: {}", _0)]
    InvalidMove(String),
    #[display(fmt = "game has already ended")]
    GameEnded,
}

impl ResponseError for GameAdapterError {
    fn status_code(&self) -> StatusCode {
        StatusCode::CONFLICT
    }
}

#[derive(Serialize, Debug, Copy, Clone, Eq, PartialEq, Display)]
#[serde(rename_all = "snake_case")]
pub enum State {
    #[display(fmt = "waiting")]
    Waiting,
    #[display(fmt = "in_progress")]
    InProgress,
    #[display(fmt = "ended")]
    Ended,
}

#[derive(Serialize)]
pub struct GenericGameState {
    pub game: String,
    pub players: Vec<String>,
    pub can_move: Vec<String>,
    pub winners: Vec<String>,
    pub state: State,
    pub payload: Value,
}

#[derive(Deserialize)]
pub struct GenericGameMove {
    pub player: String,
    pub payload: Value,
}

pub trait GameAdapter: Send {
    fn new(game_id: GameId) -> Self
    where
        Self: Sized;
    fn get_notifier(&self) -> &Notifier;
    fn add_player(&mut self, username: String) -> Result<()>;
    fn has_player(&self, username: &str) -> bool;
    fn play_move(&mut self, game_move: GenericGameMove) -> Result<()>;
    fn get_encoded_state(&self) -> Result<GenericGameState>;
    fn get_user_from_token(&self) -> String;
}
