use tokio::sync::mpsc::Receiver;

pub(crate) enum RecordControlMessage {
    Add,
    Update,
    Remove
}

pub(crate) async fn recording_pool_startup(rx: Receiver<RecordControlMessage>) {

}
