// API endpoints

use crate::game::adapter::GameAdapter;
use crate::game::{adapter, connect4, GameId, GameManager, GameManagerError, SessionId};
use actix_web::web::Json;
use actix_web::{get, post, web, Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize)]
pub struct CreateGameReq {
    name: String,
}

#[derive(Serialize)]
pub struct CreateGameRes {
    game_id: String,
}

#[derive(Deserialize)]
pub struct JoinGameReq {
    username: String,
}

#[derive(Serialize)]
pub struct JoinGameRes {
    session_id: String,
}

#[derive(Deserialize)]
pub struct SubmitMoveReq {
    session_id: String,
    payload: Value,
}

#[derive(Serialize)]
pub struct SubmitMoveRes {
    success: bool,
}

#[derive(Deserialize)]
pub struct WaitForUpdateQuery {
    since: Option<usize>,
}

#[derive(Serialize)]
pub struct WaitForUpdateRes {
    clock: usize,
}

#[post("/api/create-game")]
pub(crate) async fn create_game(
    payload: web::Json<CreateGameReq>,
    gm_wrapped: web::Data<GameManager>,
) -> Result<Json<CreateGameRes>> {
    // Validate & Pass it on
    let game_id = match payload.name.as_str() {
        "connect_4" => gm_wrapped.create_game(|id| Box::new(connect4::Connect4Adapter::new(id))),
        _ => {
            return Err(Error::from(GameManagerError::NoSuchGameType(
                payload.name.clone(),
            )))
        }
    };
    game_id.and_then(|id| {
        Ok(Json(CreateGameRes {
            game_id: id.to_string(),
        }))
    })
}

#[get("/api/list-games")]
pub(crate) async fn list_games(gm_wrapped: web::Data<GameManager>) -> Result<Json<Vec<String>>> {
    Ok(Json(gm_wrapped.list_games()?))
}

#[post("/api/{game_id}/join-game")]
pub(crate) async fn join_game(
    web::Path(game_id): web::Path<String>,
    payload: web::Json<JoinGameReq>,
    gm_wrapped: web::Data<GameManager>,
) -> Result<Json<JoinGameRes>> {
    // Validation
    let game_id = GameId::from(&game_id)?;
    // Join a game
    gm_wrapped
        .receive_join(game_id, payload.username.clone())
        .and_then(|session_id| {
            Ok(Json(JoinGameRes {
                session_id: session_id.to_string(),
            }))
        })
}

#[get("/api/{game_id}/get-state")]
pub(crate) async fn get_state(
    web::Path(game_id): web::Path<String>,
    gm_wrapped: web::Data<GameManager>,
) -> Result<Json<adapter::GenericGameState>> {
    // Validation
    let game_id = GameId::from(&game_id)?;
    // Get state and return
    Ok(Json(gm_wrapped.get_state(game_id)?))
}

#[post("/api/{game_id}/submit-move")]
pub(crate) async fn submit_move(
    web::Path(game_id): web::Path<String>,
    payload: web::Json<SubmitMoveReq>,
    gm_wrapped: web::Data<GameManager>,
) -> Result<Json<SubmitMoveRes>> {
    // Validation
    let game_id = GameId::from(&game_id)?;
    let session_id = SessionId::from(&payload.session_id)?;

    // Submit that to game manager
    // Return success or failure

    gm_wrapped
        .receive_move(game_id, session_id, payload.payload.clone())
        .and_then(|()| Ok(Json(SubmitMoveRes { success: true })))
}

#[get("/api/{game_id}/wait-for-update")]
pub(crate) async fn wait_for_update(
    web::Path(game_id): web::Path<String>,
    query: web::Query<WaitForUpdateQuery>,
    gm_wrapped: web::Data<GameManager>,
) -> Result<Json<WaitForUpdateRes>> {
    let game_id = GameId::from(&game_id)?;
    Ok(Json(WaitForUpdateRes {
        clock: gm_wrapped.subscribe(game_id)?.wait(query.since).await?,
    }))
}
