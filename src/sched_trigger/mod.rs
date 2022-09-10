use std::cell::Cell;
use std::sync::Mutex;
use chrono::{Local, DateTime};
use tokio::sync::mpsc::Sender;
use crate::recording_pool::RecordControlMessage;

pub(crate) static Q_RESERVED: Mutex<Vec<Reservation>> = Mutex::new(vec![]);

pub(crate) struct Reservation {
    start: Cell<DateTime<Local>>,
    end: Cell<DateTime<Local>>

}

pub(crate) async fn scheduler_startup(tx: Sender<RecordControlMessage>) -> ! {

    loop {
        let lock = Q_RESERVED.lock().unwrap();

        for item in lock.iter() {
            if is_in_the_recording_range(item.start.get(), item.end.get(), Local::now()) {
                tx.send(RecordControlMessage::Add).await.unwrap();
            } else if item.end.get() < Local::now() {
                tx.send(RecordControlMessage::Remove).await.unwrap()
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

#[inline]
fn is_in_the_recording_range(left: DateTime<Local>, right: DateTime<Local>, value: DateTime<Local>) -> bool {
    (left - chrono::Duration::seconds(10) < value) && (value < right)
}
