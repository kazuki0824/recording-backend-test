use std::sync::Mutex;
use tokio::sync::mpsc::Sender;
use crate::recording_pool::RecordControlMessage;

pub(crate) static Q_RESERVED: Mutex<Vec<Reservation>> = Mutex::new(vec![]);

pub(crate) struct Reservation {


}


pub(crate) async fn scheduler_startup(tx: Sender<RecordControlMessage>) {

}