// API endpoints

use crate::game::adapter::{GameAdapter, Stage};
use crate::game::search::{GameSummary, SearchOptions, SortKey, SortOrder};
use crate::game::{adapter, connect4, GameId, GameManager, GameType, SessionId};
use actix_web::web::Json;
use actix_web::{get, post, web, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize)]
pub struct CreateGameReq {
    game_type: GameType,
}

#[derive(Serialize)]
pub struct CreateGameRes {
    game_id: GameId,
}

#[post("/api/create-game")]
pub(crate) async fn create_game(
    payload: web::Json<CreateGameReq>,
    gm_wrapped: web::Data<GameManager>,
) -> Result<Json<CreateGameRes>> {
    let game_id = match payload.game_type {
        GameType::Connect4 => {
            gm_wrapped.create_game(|id| Box::new(connect4::Connect4Adapter::new(id)))
        }
    }?;

    Ok(Json(CreateGameRes { game_id }))
}

#[derive(Deserialize)]
pub struct ListGamesQuery {
    page: Option<usize>,
    sort_order: Option<SortOrder>,
    sort_key: Option<SortKey>,
    game_type: Option<GameType>,
    players: Option<usize>,
    stage: Option<Stage>,
}

#[get("/api/list-games")]
pub(crate) async fn list_games(
    query: web::Query<ListGamesQuery>,
    gm_wrapped: web::Data<GameManager>,
) -> Result<Json<Vec<GameSummary>>> {
    let ListGamesQuery {
        page,
        sort_order,
        sort_key,
        game_type,
        players,
        stage,
    } = query.0;

    let options = SearchOptions {
        page: page.unwrap_or(1),
        sort_order: sort_order.unwrap_or(SortOrder::Desc),
        sort_key: sort_key.unwrap_or(SortKey::LastUpdated),
        game_type,
        players,
        stage,
    };

    Ok(Json(gm_wrapped.list_games(options)?))
}

#[derive(Deserialize)]
pub struct JoinGameReq {
    username: String,
}

#[derive(Serialize)]
pub struct JoinGameRes {
    session_id: SessionId,
}

#[post("/api/{game_id}/join-game")]
pub(crate) async fn join_game(
    web::Path(game_id): web::Path<GameId>,
    payload: web::Json<JoinGameReq>,
    gm_wrapped: web::Data<GameManager>,
) -> Result<Json<JoinGameRes>> {
    gm_wrapped
        .receive_join(game_id, payload.username.clone())
        .and_then(|session_id| Ok(Json(JoinGameRes { session_id })))
}

#[get("/api/{game_id}/get-state")]
pub(crate) async fn get_state(
    web::Path(game_id): web::Path<GameId>,
    gm_wrapped: web::Data<GameManager>,
) -> Result<Json<adapter::GenericGameState>> {
    Ok(Json(gm_wrapped.get_state(game_id)?))
}

#[derive(Deserialize)]
pub struct SubmitMoveReq {
    session_id: SessionId,
    payload: Value,
}

#[derive(Serialize)]
pub struct SubmitMoveRes {
    success: bool,
}

#[post("/api/{game_id}/submit-move")]
pub(crate) async fn submit_move(
    web::Path(game_id): web::Path<GameId>,
    payload: web::Json<SubmitMoveReq>,
    gm_wrapped: web::Data<GameManager>,
) -> Result<Json<SubmitMoveRes>> {
    gm_wrapped
        .receive_move(game_id, payload.session_id, payload.payload.clone())
        .and_then(|()| Ok(Json(SubmitMoveRes { success: true })))
}

#[derive(Deserialize)]
pub struct WaitForUpdateQuery {
    since: Option<usize>,
}

#[derive(Serialize)]
pub struct WaitForUpdateRes {
    clock: usize,
}

#[get("/api/{game_id}/wait-for-update")]
pub(crate) async fn wait_for_update(
    web::Path(game_id): web::Path<GameId>,
    query: web::Query<WaitForUpdateQuery>,
    gm_wrapped: web::Data<GameManager>,
) -> Result<Json<WaitForUpdateRes>> {
    Ok(Json(WaitForUpdateRes {
        clock: gm_wrapped.subscribe(game_id)?.wait(query.since).await?,
    }))
}
