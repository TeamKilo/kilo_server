// API endpoints

use std::fmt::{Display, Formatter, write};
use std::ops::DerefMut;
use std::str::FromStr;
use std::sync::RwLock;
use actix_web::{post, get, Responder, web, Result, ResponseError, Error};
use actix_web::http::StatusCode;
use actix_web::web::Json;
use serde::{Serialize, Deserialize};
use serde_json::{json};
use crate::api::ValidationError::NoSuchGameError;
use crate::game::{GameManager, adapter, connect4, GameId};
use crate::game::adapter::GameAdapter;

/* Helpers */

/// ValidationError
#[derive(Debug, Clone)]
pub enum ValidationError {
    ParseIntError(String),
    NoSuchGameError(String)
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::ParseIntError(id) => write!(f, "ID {} is Invalid", id),
            ValidationError::NoSuchGameError(game) => write!(f, "Game {} not found", game)
        }
    }
}

impl ResponseError for ValidationError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

/// validate_id
fn validate_id(game_id: &String) -> Result<u128, Error> {
    let game_id_int = u128::from_str(&game_id);
    match game_id_int {
        Ok(value) => Ok(value),
        Err(_) => return Err(actix_web::Error::from(ValidationError::ParseIntError(game_id.clone()))) // TODO: need custom error
    }
}

/* Structs for the different endpoints*/

/// Struct for req to `/api/create-game`
#[derive(Deserialize)]
pub struct CreateGameReq {
    name: String
}

/// Struct for res from `/api/create-game`
#[derive(Serialize)]
pub struct CreateGameRes {
    game_id: String
}

/// Struct for res from `/api/list-games`
#[derive(serde::Serialize)]
pub struct ListGamesRes {
    // TODO
}

/// Struct for req to `/api/join-game`
#[derive(Deserialize)]
pub struct JoinGameReq {
    game_id: String,
    username: String
}

/// Struct for res from `/api/join-game`
#[derive(serde::Serialize)]
pub struct JoinGameRes {
    session_id: String
}

// Struct for res from `/api/{game_id}/get-state`
// <br> It's basically the game::adaptor::GenericGameState Object

// Struct for req to `/api/{game_id}/submit-move`
// This is basically the game::adaptor::GenericGameMove struct

/// Struct for res from `/api/{game_id}/submit-move`
#[derive(Serialize)]
pub struct SubmitMoveRes {
    success: bool,
}

/// Struct for res from `/api/{game_id}/submit-move`
#[derive(Serialize)]
pub struct WaitForUpdateRes {
    updated: bool
}

#[post("/api/create-game")]
pub(crate) async fn create_game(mut payload: web::Json<CreateGameReq>,
                                gm_wrapped: web::Data<RwLock<GameManager>>) -> Result<Json<CreateGameRes>> {
    // Validate & Pass it on
    let mut gm = gm_wrapped.write().unwrap();
    let game_id = match payload.name.as_str() {
        "connect_4" => gm.create_game(|id| Box::new(connect4::Connect4Adapter::new(id))),
        _ => return Err(Error::from(NoSuchGameError(payload.name.clone()))) // TODO: Should I return an empty string or an error
    };
    game_id.and_then(|id| Ok(Json(CreateGameRes { game_id: id.to_string()[4..].parse()? })))
}

#[get("/api/list-games")]
pub(crate) async fn list_games() -> Result<Json<ListGamesRes>> {
    todo!()
}

#[post("/api/join-game")]
pub(crate) async fn join_game(mut payload: web::Json<JoinGameReq>,
                              gm_wrapped: web::Data<RwLock<GameManager>>) -> Result<Json<JoinGameRes>> {
    // Validate input
    let game_id = validate_id(&payload.game_id)?;
    // Join a game
    let mut gm = gm_wrapped.write().unwrap();
    let session_id = gm.receive_join(GameId::from(game_id), payload.username.clone());
    todo!()
}

#[get("/api/{game_id}/get-state")]
pub(crate) async fn get_state(web::Path((game_id)): web::Path<String>, gm_wrapped: web::Data<RwLock<GameManager>>) -> Result<Json<adapter::GenericGameState>> {
    // Validation & get_state
    let game_id = validate_id(&game_id)?;
    let gm = gm_wrapped.read().unwrap();
    // Get state and return
    Ok(Json(gm.get_state(GameId::from(game_id))?))
}

#[post("/api/{game_id}/submit-move")]
pub(crate) async fn submit_move(web::Path((game_id)): web::Path<String>,
                                mut payload: web::Json<adapter::GenericGameMove>,
                                gm_wrapped: web::Data<RwLock<GameManager>>) -> Result<Json<SubmitMoveRes>> {
    let game_id = validate_id(&game_id)?;
    // Submit that to game manager
    let mut gm = gm_wrapped.write().unwrap();
    // Return success or failure
    gm.receive_move(payload.into_inner()).and_then(|()|Ok(Json(SubmitMoveRes { success: true })))
}

#[get("/api/{game_id}/wait-for-update")]
pub(crate) async fn wait_for_move(web::Path((game_id)): web::Path<String>) -> Result<Json<WaitForUpdateRes>> {
    todo!()
}
