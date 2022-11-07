use std::sync::Arc;

use log::info;
use mirakurun_client::models::Program;
use serde_derive::{Deserialize, Serialize};
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;

use crate::recording_pool::pool::RecTaskQueue;

pub(crate) mod pool;
mod recording_task;

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

pub(crate) async fn recording_pool_startup(
    q_recording: Arc<Mutex<RecTaskQueue>>,
    mut rx: Receiver<RecordControlMessage>,
) {
    // Process messages one after another
    loop {
        let received = rx.recv().await;

        if let Some(received) = received.as_ref() {
            info!("Incoming RecordControlMessage:\n {:?}", received);
        }
        let res = match received {
            Some(RecordControlMessage::CreateOrUpdate(info)) => q_recording.lock().await.add(info),
            Some(RecordControlMessage::Remove(id)) => {
                q_recording.lock().await.try_remove(id);
                false
            }
            Some(RecordControlMessage::TryCreate(info)) => q_recording.lock().await.try_add(info),
            None => continue,
        };
    }
}
