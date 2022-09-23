use std::cell::Cell;
use std::sync::Mutex;

use chrono::{DateTime, Duration, Local};
use log::info;
use mirakurun_client::models::Program;
use tokio::sync::mpsc::Sender;
use ulid::Ulid;

use crate::recording_pool::{RecordControlMessage, RecordingTaskDescription};

pub(crate) static Q_RESERVED: Mutex<Vec<Reservation>> = Mutex::new(vec![]);

#[derive(Clone)]
pub(crate) struct Reservation {
    program: Program,
    pub(crate) plan_id: Option<Ulid>, // If it is added through a plan (e.g. Record all of the items in the series), its uuid is stored here.
    is_active: bool
}

pub(crate) async fn scheduler_startup(tx: Sender<RecordControlMessage>) -> ! {

    loop {
        info!("Now locking Q_RESERVED.");
        {
            let mut lock = Q_RESERVED.lock().unwrap();

            let (found, mut remainder) = (lock.len(), 0usize);

            info!("{} scheduling units found.", found);

            for item in lock.iter() {
                let (start, end) = (item.program.start_at, item.program.start_at + Duration::milliseconds(item.program.duration as i64));
                if is_in_the_recording_range(start.into(), end.into(), Local::now()) && item.is_active {
                    let task = RecordingTaskDescription {
                        title: "".to_string(),
                        mirakurun_id: item.program.id,
                        start: Cell::new(Default::default()),
                        end: Cell::new(Default::default())
                    };
                    tx.send(RecordControlMessage::Create(task)).await.unwrap();
                }
            }
            // Drop expired item
            lock.retain(|item| {
                let time = item.program.start_at + Duration::milliseconds(item.program.duration as i64);
                time < Local::now()
            });
            remainder = lock.len();

            info!("{} scheduling units remains. {} of unit(s) dropped.", remainder, found - remainder);
        }
        info!("Scanning schedules completed. Now releasing Q_RESERVED.");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

#[inline]
fn is_in_the_recording_range(left: DateTime<Local>, right: DateTime<Local>, value: DateTime<Local>) -> bool {
    (left - chrono::Duration::seconds(10) < value) && (value < right)
}
