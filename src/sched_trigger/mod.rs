use std::cell::Cell;
use std::ops::DerefMut;
use std::sync::Mutex;
use chrono::{Local, DateTime};
use tokio::sync::mpsc::Sender;
use uuid::Uuid;
use crate::recording_pool::RecordControlMessage;

pub(crate) static Q_RESERVED: Mutex<Vec<Reservation>> = Mutex::new(vec![]);

#[derive(Clone)]
pub(crate) struct Reservation {
    start: Cell<DateTime<Local>>,
    end: Cell<DateTime<Local>>,
    auto: Option<Uuid> // If it is added auto-planned reservation (e.g. Record all of the items in the series)
}

pub(crate) async fn scheduler_startup(tx: Sender<RecordControlMessage>) -> ! {

    loop {
        let mut lock = Q_RESERVED.lock().unwrap();

        let mut v = lock.clone();
        for item in lock.iter() {
            if is_in_the_recording_range(item.start.get(), item.end.get(), Local::now()) {
                tx.send(RecordControlMessage::Add).await.unwrap();
            }
        }
        // Drop expired item TODO: Repetitive recording rule
        lock.retain(|item| item.end.get() < Local::now());

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

#[inline]
fn is_in_the_recording_range(left: DateTime<Local>, right: DateTime<Local>, value: DateTime<Local>) -> bool {
    (left - chrono::Duration::seconds(10) < value) && (value < right)
}
