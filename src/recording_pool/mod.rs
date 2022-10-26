use std::cell::Cell;
use std::sync::Arc;

use chrono::{DateTime, Local};
use log::info;
use serde_derive::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use ulid::Ulid;

use crate::recording_pool::eit_parser::EitParser;
use crate::recording_pool::pool::RecTaskQueue;

mod eit_parser;
pub(crate) mod pool;
mod recording_task;

#[derive(Debug)]
pub enum RecordControlMessage {
    Create(RecordingTaskDescription),
    Update((Ulid, DateTime<Local>, DateTime<Local>)),
    Remove(Ulid),
}

// Recomposed from Program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingTaskDescription {
    pub title: String,
    pub mirakurun_id: i64,
    pub(crate) start: Cell<DateTime<Local>>,
    pub(crate) end: Cell<DateTime<Local>>,
}

pub(crate) async fn recording_pool_startup(
    q_recording: Arc<Mutex<RecTaskQueue>>,
    tx: Sender<RecordControlMessage>,
    mut rx: Receiver<RecordControlMessage>,
) {
    // Process messages one after another
    loop {
        let received = rx.recv().await;

        if let Some(received) = received.as_ref() {
            info!("Incoming RecordControlMessage:\n {:?}", received);
        }
        match received {
            Some(RecordControlMessage::Create(info)) => {
                q_recording.lock().await.try_add(info, tx.clone());
            }
            Some(RecordControlMessage::Remove(id)) => {
                q_recording.lock().await.try_remove(id);
            }
            Some(RecordControlMessage::Update((id, start, end))) => {
                if let Some(selected_item) = q_recording.lock().await.at(id) {
                    selected_item.start.set(start);
                    selected_item.end.set(end);
                }
            }
            None => break,
        }
    }
}
