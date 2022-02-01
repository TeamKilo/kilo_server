mod api;
mod game;

use actix_web::{web, App, HttpRequest, HttpServer, Responder};
use std::sync::RwLock;
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let host = env::var("HOST").unwrap_or("127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or("8080".to_string());

    let game_manager = web::Data::new(
        RwLock::new(game::GameManager::new())
    );
    HttpServer::new(move || {
        App::new()
            .app_data(game_manager.clone())
            .service(api::create_game)
            .service(api::list_games)
            .service(api::join_game)
            .service(api::get_state)
            .service(api::submit_move)
            .service(api::wait_for_move)
    }).bind(format!("{}:{}", host, port))?.run().await
}
