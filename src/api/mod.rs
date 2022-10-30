use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{response, Router, routing::{get, put}};
use axum::response::IntoResponse;
use log::info;
use structopt::StructOpt;
use tokio::sync::Mutex;

use crate::{Opt, RecTaskQueue, SchedQueue};
use crate::db_utils::{get_all_programs, get_temporary_accessor, pull_program};
use crate::recording_pool::RecordingTaskDescription;
use crate::sched_trigger::Schedule;

mod db;

pub(crate) async fn api_startup(
    q_schedules: Arc<Mutex<SchedQueue>>,
    q_recording: Arc<Mutex<RecTaskQueue>>,
) {
    let q_schedules1 = q_schedules.clone();
    let q_schedules2 = q_schedules.clone();
    let q_schedules3 = q_schedules.clone();
    let app =
        Router::new()
            .route(
                "/",
                get(move || async move {
                    serde_json::to_string(&q_schedules1.lock().await.items).unwrap()
                }),
            )
            .route(
                "/programs",
                get(|| async {
                    let args = Opt::from_args();
                    let client = get_temporary_accessor();
                    let res = get_all_programs(&client).await;
                    match res {
                        Ok(res) => Ok(response::Json(res)),
                        Err(e) => Err(e.to_string().into_response())
                    }
                }),
            )
            .route(
                "/q/sched",
                get(move || async move {
                    serde_json::to_string(&q_schedules2.lock().await.items).unwrap()
                }),
            )
            .route(
                "/q/recording",
                get(move || async move {
                    let obj = q_recording
                        .lock()
                        .await
                        .iter()
                        .cloned()
                        .collect::<Vec<RecordingTaskDescription>>();
                    serde_json::to_string(&obj).unwrap()
                }),
            )
            .route("/new/sched", put(move |p| async move { put_recording_schedule(q_schedules3, p).await }));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("listening on {}", addr);
    let e = axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}


async fn put_recording_schedule(schedules: Arc<Mutex<SchedQueue>>,
                                axum::extract::Query(params):
                                axum::extract::Query<HashMap<String, String>>) -> Result<response::Json<Schedule>, String>
{
    let program = {
        let client = get_temporary_accessor();
        // Check input
        let id = params.get("id").ok_or("invalid query string\n")?
            .parse::<i64>().map_err(|e| e.to_string())?;
        // Pull
        pull_program(&client, id).await.or_else(|e| Err(e.to_string()))?
    };
    let s = Schedule{
        program,
        plan_id: None,
        is_active: true
    };
    schedules.lock().await.items.push(s.clone());
    info!("Program {:?} (service_id={}, network_id={}, event_id={}) has been successfully added to sched_trigger.",
        &s.program.description,
        &s.program.service_id,
        &s.program.network_id,
        &s.program.event_id,
    );
    Ok(response::Json(s))
}
