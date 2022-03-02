// API endpoints

use crate::game::adapter::{GameAdapter, Stage};
use crate::game::search::{GameSummary, SearchOptions, SortKey, SortOrder};
use crate::game::{adapter, connect4, GameId, GameManager, GameType, SessionId};
use actix_web::web::Json;
use actix_web::{get, post, web, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize)]
pub struct CreateGameRequest {
    game_type: GameType,
}

#[derive(Serialize)]
pub struct CreateGameResponse {
    game_id: GameId,
}

#[post("/api/create-game")]
pub(crate) async fn create_game(
    payload: web::Json<CreateGameRequest>,
    gm_wrapped: web::Data<GameManager>,
) -> Result<Json<CreateGameResponse>> {
    let game_id = match payload.game_type {
        GameType::Connect4 => {
            gm_wrapped.create_game(|id| Box::new(connect4::Connect4Adapter::new(id)))
        }
    }?;

    Ok(Json(CreateGameResponse { game_id }))
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

#[derive(Serialize)]
pub struct ListGamesResponse {
    game_summaries: Vec<GameSummary>,
    number_of_games: usize,
}

#[get("/api/list-games")]
pub(crate) async fn list_games(
    query: web::Query<ListGamesQuery>,
    gm_wrapped: web::Data<GameManager>,
) -> Result<Json<ListGamesResponse>> {
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

    let game_summaries = gm_wrapped.list_games(options)?;

    Ok(Json(ListGamesResponse {
        game_summaries,
        number_of_games: gm_wrapped.get_number_of_games(),
    }))
}

#[derive(Deserialize)]
pub struct JoinGameRequest {
    username: String,
}

#[derive(Serialize)]
pub struct JoinGameResponse {
    session_id: SessionId,
}

#[post("/api/{game_id}/join-game")]
pub(crate) async fn join_game(
    web::Path(game_id): web::Path<GameId>,
    payload: web::Json<JoinGameRequest>,
    gm_wrapped: web::Data<GameManager>,
) -> Result<Json<JoinGameResponse>> {
    gm_wrapped
        .receive_join(game_id, payload.username.clone())
        .and_then(|session_id| Ok(Json(JoinGameResponse { session_id })))
}

#[get("/api/{game_id}/get-state")]
pub(crate) async fn get_state(
    web::Path(game_id): web::Path<GameId>,
    gm_wrapped: web::Data<GameManager>,
) -> Result<Json<adapter::GenericGameState>> {
    Ok(Json(gm_wrapped.get_state(game_id)?))
}

#[derive(Deserialize)]
pub struct SubmitMoveRequest {
    session_id: SessionId,
    payload: Value,
}

#[derive(Serialize)]
pub struct SubmitMoveResponse {
    success: bool,
}

#[post("/api/{game_id}/submit-move")]
pub(crate) async fn submit_move(
    web::Path(game_id): web::Path<GameId>,
    payload: web::Json<SubmitMoveRequest>,
    gm_wrapped: web::Data<GameManager>,
) -> Result<Json<SubmitMoveResponse>> {
    gm_wrapped
        .receive_move(game_id, payload.session_id, payload.payload.clone())
        .and_then(|()| Ok(Json(SubmitMoveResponse { success: true })))
}

#[derive(Deserialize)]
pub struct WaitForUpdateQuery {
    since: Option<usize>,
}

#[derive(Serialize)]
pub struct WaitForUpdateResponse {
    clock: usize,
}

#[get("/api/{game_id}/wait-for-update")]
pub(crate) async fn wait_for_update(
    web::Path(game_id): web::Path<GameId>,
    query: web::Query<WaitForUpdateQuery>,
    gm_wrapped: web::Data<GameManager>,
) -> Result<Json<WaitForUpdateResponse>> {
    Ok(Json(WaitForUpdateResponse {
        clock: gm_wrapped.subscribe(game_id)?.wait(query.since).await?,
    }))
}
