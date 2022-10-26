use std::cell::Cell;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, Duration, Local};
use log::{info, warn};
use mirakurun_client::models::Program;
use serde_derive::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use ulid::Ulid;

use crate::recording_pool::{RecordControlMessage, RecordingTaskDescription};

pub(crate) struct SchedQueue {
    pub(crate) items: Vec<Schedule>,
}

impl Drop for SchedQueue {
    fn drop(&mut self) {
        //Export remaining tasks
        let path = Path::new("q_recording.json").canonicalize().unwrap();
        let result = match serde_json::to_string(&self.items) {
            Ok(str) => std::fs::write(&path, str),
            Err(e) => panic!("Serialization failed. {}", e),
        };
        if result.is_ok() {
            println!("q_recording is saved in {}.", path.display())
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Schedule {
    pub(crate) program: Program,
    pub(crate) plan_id: Option<u128>, // If it is added through a plan (e.g. Record all of the items in the series), its uuid is stored here.
    is_active: bool,
}

pub(crate) async fn scheduler_startup(
    q_schedules: Arc<Mutex<SchedQueue>>,
    tx: Sender<RecordControlMessage>,
) -> Result<(), std::io::Error> {
    //Import all the previously stored schedules
    {
        q_schedules.lock().await.items.append(&mut {
            let path = Path::new("q_schedules.json");
            let schedules=
                if path.exists() {
                    let str = std::fs::read("q_schedules.json")?;
                    match serde_json::from_slice::<Vec<Schedule>>(&str)
                    {
                        Ok(items) => Some(items),
                        Err(e) => {
                            warn!("{}", e);
                            None
                        }
                    }
                }
                else {
                    None
                };
            schedules.unwrap_or_else(|| {
                info!("No valid q_schedules.json is found. It'll be created or overwritten just before exiting.");
                Vec::new()
            })
        });
    }

    loop {
        info!("Now locking q_schedules.");
        {
            let q_schedules = &mut q_schedules.lock().await;

            let (found, mut remainder) = (q_schedules.items.len(), 0usize);

            info!("{} scheduling units found.", found);

            for item in q_schedules.items.iter() {
                let (start, end) = (
                    item.program.start_at,
                    item.program.start_at + Duration::milliseconds(item.program.duration as i64),
                );
                if is_in_the_recording_range(start.into(), end.into(), Local::now())
                    && item.is_active
                {
                    let task = RecordingTaskDescription {
                        title: "".to_string(),
                        mirakurun_id: item.program.id,
                        start: Cell::new(Default::default()),
                        end: Cell::new(Default::default()),
                    };
                    tx.send(RecordControlMessage::Create(task)).await.unwrap();
                }
            }
            // Drop expired item
            q_schedules.items.retain(|item| {
                let time =
                    item.program.start_at + Duration::milliseconds(item.program.duration as i64);
                time < Local::now()
            });
            remainder = q_schedules.items.len();

            info!(
                "{} schedule units remains. {} of unit(s) dropped.",
                remainder,
                found - remainder
            );
        }
        info!("Scanning schedules completed. Now releasing q_schedules.");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

#[inline]
fn is_in_the_recording_range(
    left: DateTime<Local>,
    right: DateTime<Local>,
    value: DateTime<Local>,
) -> bool {
    (left - Duration::seconds(10) < value) && (value < right)
}
