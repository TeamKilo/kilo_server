// API endpoints

use crate::game::adapter::GameAdapter;
use crate::game::ValidationError::NoSuchGameError;
use crate::game::{adapter, connect4, GameId, GameManager};
use actix_web::web::Json;
use actix_web::{get, post, web, Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/* Helpers */

/* Structs for the different endpoints*/

/// Struct for req to `/api/create-game`
#[derive(Deserialize)]
pub struct CreateGameReq {
    name: String,
}

/// Struct for res from `/api/create-game`
#[derive(Serialize)]
pub struct CreateGameRes {
    game_id: String,
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
    username: String,
}

/// Struct for res from `/api/join-game`
#[derive(serde::Serialize)]
pub struct JoinGameRes {
    session_id: String,
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
    updated: bool,
}

#[post("/api/create-game")]
pub(crate) async fn create_game(
    mut payload: web::Json<CreateGameReq>,
    gm_wrapped: web::Data<RwLock<GameManager>>,
) -> Result<Json<CreateGameRes>> {
    // Validate & Pass it on
    let mut gm = gm_wrapped.write().unwrap();
    let game_id = match payload.name.as_str() {
        "connect_4" => gm.create_game(|id| Box::new(connect4::Connect4Adapter::new(id))),
        _ => return Err(Error::from(NoSuchGameError(payload.name.clone()))),
    };
    game_id.and_then(|id| {
        Ok(Json(CreateGameRes {
            game_id: id.to_string(),
        }))
    })
}

#[get("/api/list-games")]
pub(crate) async fn list_games() -> Result<Json<ListGamesRes>> {
    todo!()
}

#[post("/api/join-game")]
pub(crate) async fn join_game(
    mut payload: web::Json<JoinGameReq>,
    gm_wrapped: web::Data<RwLock<GameManager>>,
) -> Result<Json<JoinGameRes>> {
    // Validate input
    let game_id = GameId::from(&payload.game_id)?;
    // Join a game
    let mut gm = gm_wrapped.write().unwrap();
    gm.receive_join(game_id, payload.username.clone())
        .and_then(|session_id| {
            Ok(Json(JoinGameRes {
                session_id: session_id.to_string(),
            }))
        })
}

#[get("/api/{game_id}/get-state")]
pub(crate) async fn get_state(
    web::Path((game_id)): web::Path<String>,
    gm_wrapped: web::Data<RwLock<GameManager>>,
) -> Result<Json<adapter::GenericGameState>> {
    // Validation & get_state
    let game_id = GameId::from(&game_id)?;
    let gm = gm_wrapped.read().unwrap();
    // Get state and return
    Ok(Json(gm.get_state(game_id)?))
}

#[post("/api/{game_id}/submit-move")]
pub(crate) async fn submit_move(
    web::Path((game_id)): web::Path<String>,
    mut payload: web::Json<adapter::GenericGameMove>,
    gm_wrapped: web::Data<RwLock<GameManager>>,
) -> Result<Json<SubmitMoveRes>> {
    let game_id = GameId::from(&game_id)?;
    // Submit that to game manager
    let mut gm = gm_wrapped.read().unwrap();
    // Return success or failure
    gm.receive_move(payload.into_inner())
        .and_then(|()| Ok(Json(SubmitMoveRes { success: true })))
}

#[get("/api/{game_id}/wait-for-update")]
pub(crate) async fn wait_for_move(
    web::Path((game_id)): web::Path<String>,
) -> Result<Json<WaitForUpdateRes>> {
    todo!()
}
