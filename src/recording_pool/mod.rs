use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub(crate) enum RecordControlMessage {
    Add,
    Update,
    Remove
}

struct RecordingTask {

}

pub(crate) async fn recording_pool_startup(tx: Sender<RecordControlMessage>, mut rx: Receiver<RecordControlMessage>) {
    let q_recording: Vec<RecordingTask> = vec![];

    loop {
        match rx.recv().await
        {
            Some(RecordControlMessage::Add) => {

            },
            Some(RecordControlMessage::Remove) => {

            },
            Some(RecordControlMessage::Update) => {

            },
            None => break
        }
    }
}
