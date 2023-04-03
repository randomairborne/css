#![warn(clippy::all, clippy::nursery, clippy::pedantic)]
mod auth;
mod error;
mod oauth;
mod routes;
mod state;

extern crate google_classroom1 as classroom;

use axum::routing::get;
pub use error::Error;
pub use state::AppState;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let config_string =
        std::fs::read_to_string("./config.toml").expect("Failed to read config file");
    let config: Config = toml::from_str(&config_string).expect("Invalid TOML");
    let app = axum::Router::new()
        .route("/", get(routes::about))
        .route("/privacy", get(routes::privacy))
        .route("/privacy/", get(routes::privacy))
        .route("/terms", get(routes::terms))
        .route("/terms/", get(routes::terms))
        .route("/classes", get(routes::classes))
        .route("/classes/", get(routes::classes))
        .route("/class/:classid", get(routes::class))
        .route("/todo", get(routes::todos_all))
        .route("/todo/", get(routes::todos_all))
        .route("/todo/:class", get(routes::todos_for_class))
        .route("/oauth", get(oauth::redirect))
        .route("/oauth/callback", get(oauth::set_tokens))
        .layer(tower_cookies::CookieManagerLayer::new())
        .nest_service("/assets", ServeDir::new("assets"))
        .with_state(AppState::new(config));
    println!("Listening on http://localhost:8080");
    axum::Server::bind(&([0, 0, 0, 0], 8080).into())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(serde::Deserialize)]
pub struct Config {
    key: String,
    root_url: String,
    client_id: String,
    client_secret: String,
}
