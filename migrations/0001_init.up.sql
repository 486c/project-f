-- Add up migration script here
CREATE TABLE IF NOT EXISTS files (
  id TEXT PRIMARY KEY NOT NULL,
  filename TEXT NOT NULL,
  bytes INTEGER NOT NULL,
  crc INTEGER NOT NULL
);