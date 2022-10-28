use std::net::SocketAddr;
use std::sync::Arc;

use axum::{Router, routing::get};
use axum::http::StatusCode;
use log::info;
use meilisearch_sdk::client::Client;
use mirakurun_client::models::Program;
use structopt::StructOpt;
use tokio::sync::Mutex;

use crate::{Opt, RecTaskQueue, SchedQueue};
use crate::recording_pool::RecordingTaskDescription;

mod db;

pub(crate) async fn api_startup(
    q_schedules: Arc<Mutex<SchedQueue>>,
    q_recording: Arc<Mutex<RecTaskQueue>>,
) {
    let q_schedules1 = q_schedules.clone();
    let q_schedules2 = q_schedules.clone();
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
                    let search_client = Client::new(args.meilisearch_base_uri, "masterKey");
                    let res = match search_client.get_index("_programs").await {
                        Ok(f) => {
                            f.get_documents::<Program>().await
                        },
                        Err(e) => Err(e)
                    };

                    match res {
                        Ok(res) => (StatusCode::OK, serde_json::to_string(&res.results).unwrap()),
                        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
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
            );

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("listening on {}", addr);
    let e = axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
