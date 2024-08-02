use std::{net::SocketAddr, sync::Arc};

use axum::{http::{header, StatusCode}, response::IntoResponse, Router};
use dotenvy::dotenv;
use manager::Manager;
use sqlx::SqlitePool;
use tokio::{net::TcpListener, sync::Mutex};
use eyre::Result;
use tower_http::{cors::{Any, CorsLayer}, services::ServeDir};

mod manager;
mod routes;
mod state;

async fn error_404() -> impl IntoResponse {
  (StatusCode::NOT_FOUND, "404 Not Found")
}

async fn shutdown_signal() {
  tokio::signal::ctrl_c()
    .await
    .expect("failed to install CTRL+C signal handler");
}

#[tokio::main]
async fn main() -> Result<()> {
  dotenv()?;

  let listener = {
    let addr = SocketAddr::from(([127, 0, 0, 1], 9999));
    TcpListener::bind(addr).await?
  };

  let pool = SqlitePool::connect("sqlite:db.db").await?;

  let manager = Manager::new(pool);
  let state = Arc::new(Mutex::new(manager));

  let app = Router::new()
    .nest("/files", routes::files::router())
    .nest("/manage", routes::manage::router())
    .nest_service("/", ServeDir::new("static"))
    // TODO: disable in production?
    .layer(
      CorsLayer::new()
        .allow_methods(Any)
        .allow_origin(Any)
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::CONTENT_RANGE])
    )
    .fallback(error_404)
    .with_state(state);

  axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();

  Ok(())
}
