// API endpoints

use actix_web::{post, get, Responder, web::Payload};

#[post("/api/create-game")]
pub(crate) async fn create_game(mut payload: Payload) -> impl Responder {
    todo!(); "not yet implemented"
}

#[get("/api/list-games")]
pub(crate) async fn list_games(mut payload: Payload) -> impl Responder {
    todo!(); "not yet implemented"
}

#[post("/api/join-game")]
pub(crate) async fn join_game(mut payload: Payload) -> impl Responder {
    todo!(); "not yet implemented"
}

#[get("/api/{game_id}/get-state")]
pub(crate) async fn get_state(mut payload: Payload) -> impl Responder {
    todo!(); "not yet implemented"
}

#[post("/api/{game_id}/submit-move")]
pub(crate) async fn submit_move(mut payload: Payload) -> impl Responder {
    todo!(); "not yet implemented"
}

#[get("/api/{game_id}/wait-for-update")]
pub(crate) async fn wait_for_move(mut payload: Payload) -> impl Responder {
    todo!(); "not yet implemented"
}
