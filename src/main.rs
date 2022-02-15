mod api;
mod game;

use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{web, App, Error, HttpServer};
use std::env;

const MAX_JSON_PAYLOAD_SIZE: usize = 4096;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let host = env::var("HOST").unwrap_or("127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or("8080".to_string());

    let game_manager = web::Data::new(game::GameManager::new());
    let json_config = web::JsonConfig::default()
        .limit(MAX_JSON_PAYLOAD_SIZE)
        .error_handler(|err, _req| Error::from(err));

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(Cors::permissive())
            .app_data(game_manager.clone())
            .app_data(json_config.clone())
            .service(api::create_game)
            .service(api::list_games)
            .service(api::join_game)
            .service(api::get_state)
            .service(api::submit_move)
            .service(api::wait_for_update)
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}
