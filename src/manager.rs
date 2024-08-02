use std::{collections::HashMap, fmt::Display, fs, path::PathBuf};

use axum::{http::StatusCode, response::IntoResponse};
use rand::RngCore;
use serde::Serialize;
use sqlx::{Pool, Sqlite};
use tokio::{fs::File, io::AsyncWriteExt};

const SIZE_LIMIT: usize = 1024 * 1024 * 1024; // 1 gb 

pub enum ManagerError {
  QueryFailed,
  // Move this out
  FileExists(String),
  UnableToGenerateId,
  FileTooLarge,
  InvalidUploadId,
  ChunkOutOfBounds,
  FileNotFound,
  FailedToDelete,
}

impl Display for ManagerError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::QueryFailed => write!(f, "Database query failed"),
      Self::FileExists(id) => write!(f, "File already exists: {}", id),
      Self::UnableToGenerateId => write!(f, "Unable to generate a unique id"),
      Self::FileTooLarge => write!(f, "File is too large (max {} bytes)", SIZE_LIMIT),
      Self::InvalidUploadId => write!(f, "Invalid upload id"),
      Self::ChunkOutOfBounds => write!(f, "Chunk out of bounds"),
      Self::FileNotFound => write!(f, "File not found"),
      Self::FailedToDelete => write!(f, "Failed to delete file"),
    }
  }
}

impl IntoResponse for ManagerError {
  fn into_response(self) -> axum::response::Response {
    (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
  }
}

#[derive(Serialize)]
pub struct DatabaseFile {
  pub id: String,
  pub filename: String,
  pub bytes: i64,
}

struct ChunkProcessor {
  pub data: Vec<u8>,
  pub filename: String,
}

impl ChunkProcessor {
  fn new(size: usize, filename: &str) -> Self {
    Self {
      data: vec![0; size],
      filename: filename.to_string()
    }
  }

  fn chunk(&mut self, data: &[u8], start: usize) -> Result<(), ManagerError> {
    if start + data.len() > self.data.len() {
      return Err(ManagerError::ChunkOutOfBounds)
    }

    self.data[start..start + data.len()].copy_from_slice(data);

    Ok(())
  }
}

pub struct Manager {
  pool: Pool<Sqlite>,
  base: PathBuf,
  uploads: HashMap<String, ChunkProcessor>,
}

impl Manager {
  pub fn new(pool: Pool<Sqlite>) -> Self {
    let base = PathBuf::from("./files");

    if !base.exists() {
      fs::create_dir(&base).unwrap();
    }

    Self {
      pool,
      base,
      uploads: HashMap::new(),
    }
  }

  fn get_file_path(&self, id: &str) -> PathBuf {
    let mut path = self.base.clone();
    path.push(id);
    path
  }

  pub async fn get_files(&self, page: usize) -> Result<(i64, Vec<DatabaseFile>), ManagerError> {
    let offset = 10 * (page - 1) as u32;
    let total_files = sqlx::query_scalar!("SELECT COUNT(*) FROM files")
      .fetch_one(&self.pool)
      .await
      .map_err(|_| ManagerError::QueryFailed)?;
    let files = sqlx::query_as!(DatabaseFile, "
      SELECT id, filename, bytes
      FROM files
      LIMIT 10
      OFFSET ?
    ", offset)
      .fetch_all(&self.pool)
      .await
      .map_err(|_| ManagerError::QueryFailed)?;
    Ok((total_files, files))
  }

  pub async fn upload_file(&self, filename: &str, data: &[u8]) -> Result<String, ManagerError> {
    let original_filename = PathBuf::from(filename);
    let extension = original_filename.extension().map(|s| s.to_str().unwrap());

    let uid = self.generate_uid(extension).await.ok_or(ManagerError::QueryFailed)?;
    let file_path = self.get_file_path(&uid);

    if file_path.exists() {
      return Err(ManagerError::UnableToGenerateId); // TODO: change error
    }

    let bytes = data.len() as i64;
    let crc = crc32fast::hash(data);

    let hash_check = sqlx::query!("
      SELECT id
      FROM files
      WHERE bytes = ? AND crc = ?
    ", bytes, crc)
      .fetch_optional(&self.pool)
      .await
      .map_err(|_| ManagerError::QueryFailed)?;

    if let Some(hash_check) = hash_check {
      return Err(ManagerError::FileExists(hash_check.id));
    }

    let mut file = File::create(file_path)
      .await
      .map_err(|_| ManagerError::QueryFailed)?;

    file.write_all(data)
      .await
      .map_err(|_| ManagerError::QueryFailed)?;

    let _ = sqlx::query!(
      "INSERT INTO files
        (id, filename, bytes, crc)
      VALUES (?, ?, ?, ?)",
      uid,
      filename,
      bytes,
      crc
    )
      .execute(&self.pool)
      .await;

    Ok(uid)
  }

  pub async fn begin_chunked_upload(&mut self, filename: &str, size: usize) -> Result<String, ManagerError> {
    let id = Self::generate_random_id();

    if size > SIZE_LIMIT {
      Err(ManagerError::FileTooLarge)
    } else {
      let processor = ChunkProcessor::new(size, filename);
      self.uploads.insert(id.clone(), processor);

      Ok(id)
    }
  }

  pub async fn process_chunk(&mut self, id: &str, data: &[u8], start: usize) -> Result<(), ManagerError> {
    if let Some(processor) = self.uploads.get_mut(id) {
      processor.chunk(data, start)
    } else {
      Err(ManagerError::InvalidUploadId)
    }
  }

  pub async fn finish_chunked_upload(&mut self, id: &str) -> Result<String, ManagerError> {
    if let Some(processor) = self.uploads.remove(id) {
      self.upload_file(&processor.filename, &processor.data).await
    } else {
      Err(ManagerError::InvalidUploadId)
    }
  }

  pub async fn discard_upload(&mut self, id: &str) {
    self.uploads.remove(id);
  }

  pub async fn delete_file(&self, id: &str) -> Result<(), ManagerError> {
    let file_path = self.get_file_path(id);

    if !file_path.exists() {
      return Err(ManagerError::FileNotFound);
    }

    fs::remove_file(file_path)
      .map_err(|_| ManagerError::FailedToDelete)?;

    sqlx::query!("DELETE FROM files WHERE id = ?", id)
      .execute(&self.pool)
      .await
      .map_err(|_| ManagerError::QueryFailed)?;

    Ok(())
  }

  async fn generate_uid(&self, extension: Option<&str>) -> Option<String> {
    // 5 attemps to generate a unique id
    for _ in 0..5 {
      let id = Self::generate_random_id();

      let id = match extension {
        Some(extension) => format!("{}.{}", id, extension),
        None => id,
      };

      let result = sqlx::query!(
        r#"SELECT EXISTS(SELECT 1 FROM files WHERE id = ?) AS "exists";"#,
        id
      )
        .fetch_one(&self.pool)
        .await
        .unwrap();

      if result.exists == 1 {
        continue;
      }

      return Some(id);
    }

    None
  }

  fn generate_random_id() -> String {
    let mut bytes = [0; 8];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
  }
}