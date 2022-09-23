use std::cell::Cell;
use std::collections::BTreeMap;

use chrono::{DateTime, Local};
use log::info;
use tokio::sync::mpsc::{Receiver, Sender};
use ulid::Ulid;

use crate::recording_pool::eit_parser::EitParser;
use crate::recording_pool::recording_task::RecordingTask;

mod eit_parser;
mod recording_task;

#[derive(Debug)]
pub(crate) enum RecordControlMessage {
    Create(RecordingTaskDescription),
    Update((Ulid, DateTime<Local>, DateTime<Local>)),
    Remove(Ulid),
}

// Recomposed from Program.
#[derive(Debug)]
pub struct RecordingTaskDescription {
    pub title: String,
    pub mirakurun_id: i64,
    pub(crate) start: Cell<DateTime<Local>>,
    pub(crate) end: Cell<DateTime<Local>>,
}

pub(crate) async fn recording_pool_startup(
    tx: Sender<RecordControlMessage>,
    mut rx: Receiver<RecordControlMessage>,
) -> std::io::Result<()> {
    // Import tasks left behind
    let mut q_recording = {
        let str = std::fs::read("q_recording.json")?;
        BTreeMap::<Ulid, RecordingTask>::from_iter(
            mirakurun_client::from_str(str.into())
        )
    };

    loop {
        let received = rx.recv().await;

        if let Some(received) = received.as_ref() {
            info!("Incoming RecordControlMessage:\n {:?}", received);
        }
        match received {
            Some(RecordControlMessage::Create(info)) => {
                if q_recording
                    .iter()
                    .all(|item| item.1.info.mirakurun_id != info.mirakurun_id)
                {
                    let task_id = Ulid::new();
                    q_recording.insert(
                        task_id,
                        RecordingTask::new(task_id, info, tx.clone())
                    );
                }
            }
            Some(RecordControlMessage::Remove(id)) => {
                if q_recording.contains_key(&id) {
                    q_recording.remove(&id);
                }
            }
            Some(RecordControlMessage::Update((id, start, end))) => {
                if let Some(selected_item) = q_recording.get(&id) {
                    selected_item.info.start.set(start);
                    selected_item.info.end.set(end);
                }
            }
            None => break,
        }
    }

    //Export remaining tasks
    let queue_item_exported = q_recording.into_values().map(|item| item.info).collect();
    match mirakurun_client::to_string(queue_item_exported) {
        Ok(str) => std::fs::write("q_recording.json", str),
        Err(e) => e
    }

}
