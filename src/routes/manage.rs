use std::env;

use axum::{extract::{DefaultBodyLimit, Multipart, Path, Query, Request, State}, http::{header, HeaderMap, StatusCode}, middleware::{self, Next}, response::{IntoResponse, Response}, routing::{delete, get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::limit::RequestBodyLimitLayer;

use crate::{manager::{DatabaseFile, ManagerError}, state::AxumState};

#[derive(Clone)]
struct ManageState {
  token: String,
}

#[derive(Deserialize)]
struct Pagination {
  page: Option<usize>,
}

#[derive(Serialize)]
struct ListFilesResponse {
  files: Vec<DatabaseFile>,
  total: i64,
}

async fn list_files(
  State(manager): State<AxumState>,
  pagination: Query<Pagination>,
) -> Response {
  let page = pagination.page.unwrap_or(1);

  let manager = manager.lock().await;

  let (total, files) = match manager.get_files(page).await {
    Ok((total, files)) => (total, files),
    Err(e) => return e.into_response(),
  };

  (StatusCode::OK, Json(ListFilesResponse { files, total })).into_response()
}

#[derive(Serialize)]
struct UploadResult {
  id: String,
  existed: bool,
}

async fn upload_file(
  State(manager): State<AxumState>,
  mut multipart: Multipart,
) -> Response {
  if let Ok(Some(field)) = multipart.next_field().await {
    if field.name().unwrap() != "file" {
      return (StatusCode::BAD_REQUEST, "Bad Request (invalid multipart)").into_response();
    }

    let filename = field.file_name().unwrap().to_string();
    let bytes = field.bytes().await.unwrap();

    let manager = manager.lock().await;

    match manager.upload_file(&filename, &bytes).await {
      Ok(id) => return (StatusCode::OK, Json(UploadResult { id, existed: false })).into_response(),
      Err(e) => match e {
        ManagerError::FileExists(uid) => return (StatusCode::OK, Json(UploadResult { id: uid, existed: true })).into_response(),
        e => return e.into_response(),
      }
    }
  }

  (StatusCode::BAD_REQUEST, "Bad Request (invalid multipart)").into_response()
}

#[derive(Deserialize)]
struct BeginChunksParams {
  filename: String,
}

#[derive(Serialize)]
struct BeginChunksResult {
  id: String,
}

async fn begin_chunks(
  State(manager): State<AxumState>,
  headers: HeaderMap,
  Json(BeginChunksParams { filename }): Json<BeginChunksParams>,
) -> Response {
  let mut manager = manager.lock().await;

  let size = match headers.get(header::CONTENT_RANGE) {
    Some(value) => match value.to_str() {
      Ok(value) => value.parse::<usize>().unwrap_or(0),
      _ => 0
    },
    _ => 0
  };

  let processor_id = match manager.begin_chunked_upload(&filename, size).await {
    Ok(id) => id,
    Err(e) => return e.into_response()
  };

  (StatusCode::OK, Json(BeginChunksResult { id: processor_id })).into_response()
}

#[derive(Deserialize)]
struct ChunkParams {
  id: String,
}

async fn upload_chunk(
  State(manager): State<AxumState>,
  Path(id): Path<String>,
  headers: HeaderMap,
  mut multipart: Multipart,
) -> Response {
  if let Ok(Some(field)) = multipart.next_field().await {
    if field.name().unwrap() != "chunk" {
      return (StatusCode::BAD_REQUEST, "Bad Request (invalid multipart)").into_response();
    }

    let bytes = field.bytes().await.unwrap();

    let start = match headers.get(header::CONTENT_RANGE) {
      Some(value) => match value.to_str() {
        Ok(value) => value.parse::<usize>().unwrap_or(0),
        _ => 0
      },
      _ => 0
    };

    let mut manager = manager.lock().await;

    match manager.process_chunk(&id, &bytes, start).await {
      Ok(_) => return (StatusCode::OK, "OK").into_response(),
      Err(e) => return e.into_response()
    }
  }

  (StatusCode::BAD_REQUEST, "Bad Request (invalid multipart)").into_response()
}

async fn end_chunks(
  State(manager): State<AxumState>,
  Json(ChunkParams { id }): Json<ChunkParams>,
) -> Response {
  let mut manager = manager.lock().await;

  match manager.finish_chunked_upload(&id).await {
    Ok(id) => (StatusCode::OK, Json(UploadResult { id, existed: false })).into_response(),
    Err(e) => match e {
      ManagerError::FileExists(id) => (StatusCode::OK, Json(UploadResult { id, existed: true })).into_response(),
      e => e.into_response()
    },
  }
}

async fn discard_upload(
  State(manager): State<AxumState>,
  Path(id): Path<String>,
) -> Response {
  let mut manager = manager.lock().await;

  manager.discard_upload(&id).await;

  (StatusCode::OK, "OK").into_response()
}

async fn delete_file(
  State(manager): State<AxumState>,
  Path(id): Path<String>,
) -> Response {
  let manager = manager.lock().await;

  match manager.delete_file(&id).await {
    Ok(_) => (StatusCode::OK, "OK").into_response(),
    Err(e) => e.into_response()
  }
}

async fn auth_middleware(
  State(ManageState { token }): State<ManageState>,
  headers: HeaderMap,
  request: Request,
  next: Next
) -> Response {
  match headers.get(header::AUTHORIZATION) {
    Some(value) => {
      match value.to_str() {
        Ok(value) if value == token => next.run(request).await,
        _ => (StatusCode::FORBIDDEN, "Forbidden").into_response()
      }
    },
    _ => (StatusCode::FORBIDDEN, "Forbidden").into_response()
  }
}

pub fn router() -> Router<AxumState> {
  let token = env::vars()
    .find(|(k, _)| k == "TOKEN")
    .map(|(_, v)| v)
    .unwrap_or_default();

  Router::new()
    .route("/files", get(list_files))
    .nest("/upload", Router::new()
      .route("/file", post(upload_file))

      .route("/begin_chunks", post(begin_chunks))
      .route("/chunk/:id", post(upload_chunk))
      .route("/end_chunks", post(end_chunks))
      .route("/discard/:id", post(discard_upload))
    )
    .route("/files/:id", delete(delete_file))
    .layer(DefaultBodyLimit::disable())
    .layer(RequestBodyLimitLayer::new(
      1024 * 1024 * 90
    ))
    .layer(middleware::from_fn_with_state(ManageState { token }, auth_middleware))
}