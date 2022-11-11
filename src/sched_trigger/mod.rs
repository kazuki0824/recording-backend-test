use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Duration, Local};
use log::{error, info, warn};
use mirakurun_client::models::Program;
use serde_derive::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

use crate::recording_planner::PlanId;
use crate::recording_pool::{RecordControlMessage, RecordingTaskDescription};

pub(crate) struct SchedQueue {
    pub(crate) items: Vec<Schedule>,
}

impl Drop for SchedQueue {
    fn drop(&mut self) {
        //Export remaining tasks
        let path = Path::new("./q_schedules.json")
            .canonicalize()
            .unwrap_or(PathBuf::from("./q_schedules.json"));
        let result = match serde_json::to_string(&self.items) {
            Ok(str) => std::fs::write(&path, str),
            Err(e) => panic!("Serialization failed. {}", e),
        };
        if result.is_ok() {
            println!("q_schedules is saved in {}.", path.display())
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Schedule {
    pub(crate) program: Program,
    pub(crate) plan_id: PlanId,
    // If it is added through a plan (e.g. Record all of the items in the series), its uuid is stored here.
    pub(crate) is_active: bool,
}

pub(crate) async fn scheduler_startup(
    q_schedules: Arc<Mutex<SchedQueue>>,
    tx: Sender<RecordControlMessage>,
) -> Result<(), std::io::Error> {
    //Import all the previously stored schedules
    {
        q_schedules.lock().await.items.append(&mut {
            let path = Path::new("./q_schedules.json");
            let schedules =
                if path.exists() {
                    let str = std::fs::read(path.canonicalize()?)?;
                    match serde_json::from_slice::<Vec<Schedule>>(&str)
                    {
                        Ok(items) => Some(items),
                        Err(e) => {
                            warn!("{}", e);
                            None
                        }
                    }
                } else {
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

            // Drop expired item
            q_schedules.items.retain(|item| {
                let start_at = item.program.start_at;

                match item {
                    Schedule { program: Program{ duration: Some(length_msec), .. }, .. }  => {
                        //長さ有限かつ現在放送終了してたらドロップ
                        Local::now() < start_at + Duration::milliseconds(*length_msec as i64)
                    }
                    Schedule { program: Program{ duration: None, .. }, .. }  => {
                        //長さ未定のときは、開始時刻から１時間経過したらドロップ
                        Local::now() < start_at + Duration::hours(1)
                    }
                }
            });
            remainder = q_schedules.items.len();

            info!(
                "{} schedule units remains. {} of unit(s) dropped.",
                remainder,
                found - remainder
            );


            for item in q_schedules.items.iter() {
                match item {
                    // 有効かつ長さ有限
                    Schedule {is_active: true, program: Program{ duration: Some(length_msec),  .. }, ..} => {
                        //保存場所の決定
                        let save_location = {
                            let candidate = match item.plan_id {
                                PlanId::Word(id) => format!("./word_{}/", id),
                                PlanId::Series(id) => format!("./series_{}/", id),
                                PlanId::None => "./common/".to_string(),
                            };
                            if let Err(e) = std::fs::create_dir_all(&candidate) {
                                error!("Failed to create dir at {}.\n{}", &candidate, e);
                                continue
                            }
                            std::fs::canonicalize(candidate).unwrap()
                        };

                        let task = RecordingTaskDescription {
                            program: item.program.clone(),
                            save_location,
                        };

                        if is_in_the_recording_range(
                            // 放送開始10分以内前
                            (item.program.start_at - Duration::minutes(10)).into(),
                            item.program.start_at.into(),
                            Local::now(),
                        ) {
                            // Mirakurun側の更新を取り入れる
                            tx.send(RecordControlMessage::CreateOrUpdate(task))
                                .await
                                .unwrap();
                        } else if is_in_the_recording_range(
                            // 放送中
                            item.program.start_at.into(),
                            (item.program.start_at + Duration::milliseconds(*length_msec as i64))
                                .into(),
                            Local::now(),
                        ) {
                            // Mirakurun側の更新を取り入れず、タスク側の状態遷移に一任する
                            // 存在しない場合は追加する
                            // 放送されているのにDLが始まっていない（ex. 起動時にすでに放送がはじまっていた場合）
                            tx.send(RecordControlMessage::TryCreate(task))
                                .await
                                .unwrap();
                        }
                    }
                    Schedule{is_active: true, program: Program { duration: None, .. }, ..} => {
                        error!("未実装：録画開始時点でduration不明")
                    }
                    _ => continue,
                }
            }
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
    assert!(left < right);
    (left < value) && (value < right)
}
