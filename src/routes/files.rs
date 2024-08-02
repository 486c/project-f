use axum::{extract::{Path, Request}, response::IntoResponse, routing::get, Router};
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::state::AxumState;

async fn get_file(
  Path(id): Path<String>,
  request: Request
) -> impl IntoResponse {
  let filepath = format!("./files/{}", id);
  let serve_file = ServeFile::new(filepath);
  serve_file.oneshot(request).await
}

pub fn router() -> Router<AxumState> {
  Router::new()
    .route("/:id", get(get_file))
}