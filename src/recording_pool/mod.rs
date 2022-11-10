use std::sync::RwLock;

use log::info;
use mirakurun_client::models::Program;
use once_cell::sync::Lazy;
use serde_derive::{Deserialize, Serialize};
use tokio::sync::mpsc::Receiver;

use crate::recording_pool::pool::RecTaskQueue;

pub(crate) mod pool;
mod recording_task;

pub(crate) static REC_POOL: Lazy<RwLock<RecTaskQueue>> =
    Lazy::new(|| RwLock::new(RecTaskQueue::new()));

#[derive(Debug)]
pub enum RecordControlMessage {
    CreateOrUpdate(RecordingTaskDescription),
    TryCreate(RecordingTaskDescription),
    Remove(i64),
}

// Recomposed from Program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingTaskDescription {
    pub program: Program,
    pub save_location: String,
}

pub(crate) async fn recording_pool_startup(mut rx: Receiver<RecordControlMessage>) {
    // Process messages one after another
    loop {
        let received = rx.recv().await;

        if let Some(received) = received.as_ref() {
            info!("Incoming RecordControlMessage:\n {:?}", received);
        }

        match received {
            Some(RecordControlMessage::CreateOrUpdate(info)) => REC_POOL.write().unwrap().add(info),
            Some(RecordControlMessage::Remove(id)) => REC_POOL.write().unwrap().try_remove(id),
            Some(RecordControlMessage::TryCreate(info)) => REC_POOL.write().unwrap().try_add(info),
            None => continue,
        };
    }
}
