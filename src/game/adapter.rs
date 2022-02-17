use crate::game::GameId;
use crate::notify::Notifier;
use actix_web::http::StatusCode;
use actix_web::{ResponseError, Result};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::vec::Vec;

fn format_invalid_game_stage(stage: &Stage) -> &'static str {
    match stage {
        Stage::Waiting => "game has not started yet",
        Stage::InProgress => "game is in progress",
        Stage::Ended => "game has ended",
    }
}

#[derive(Debug, Clone, Display)]
pub enum GameAdapterErrorType {
    #[display(fmt = "player {} cannot move at the moment", _0)]
    InvalidPlayer(String),
    #[display(fmt = "invalid move: {}", _0)]
    InvalidMove(String),
    #[display(fmt = "{}", "format_invalid_game_stage(_0)")]
    InvalidGameStage(Stage),
}

#[derive(Debug, Clone, Display)]
#[display(fmt = "{} ({})", error_type, game_id)]
pub struct GameAdapterError {
    pub game_id: GameId,
    pub error_type: GameAdapterErrorType,
}

impl GameAdapterError {
    pub fn actix_err(game_id: GameId, error_type: GameAdapterErrorType) -> actix_web::Error {
        actix_web::Error::from(GameAdapterError {
            game_id,
            error_type,
        })
    }
}

impl ResponseError for GameAdapterError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

#[derive(Serialize, Debug, Copy, Clone, Eq, PartialEq, Display)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    #[display(fmt = "waiting")]
    Waiting,
    #[display(fmt = "in_progress")]
    InProgress,
    #[display(fmt = "ended")]
    Ended,
}

#[derive(Serialize)]
pub struct GenericGameState {
    pub players: Vec<String>,
    pub can_move: Vec<String>,
    pub winners: Vec<String>,
    pub stage: Stage,
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
    fn get_stage(&self) -> Stage;
    fn get_encoded_state(&self) -> Result<GenericGameState>;
    fn get_user_from_token(&self) -> String;
    fn get_type(&self) -> &str;
}
