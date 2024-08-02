use std::sync::Arc;
use tokio::sync::Mutex;
use crate::manager::Manager;

pub type AxumState = Arc<Mutex<Manager>>;