#[macro_use]
extern crate machine;

use std::sync::Arc;

use structopt::StructOpt;
use tokio::sync::Mutex;

use crate::sched_trigger::SchedQueue;
use crate::{
    api::api_startup, epg_syncer::epg_sync_startup, recording_pool::recording_pool_startup,
    sched_trigger::scheduler_startup,
};

mod api;
mod db_utils;
mod epg_syncer;
mod mirakurun_client;
mod recording_planner;
mod recording_pool;
mod sched_trigger;

#[derive(Debug, StructOpt)]
#[structopt(name = "meister", about = "An example of StructOpt usage.")]
struct Opt {
    #[structopt(default_value = "http://localhost:40772/api")]
    mirakurun_base_uri: String,
    #[structopt(default_value = "http://localhost:7700/")]
    meilisearch_base_uri: String,
    #[structopt(short)]
    meilisearch_api_key: Option<String>,
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    env_logger::init();

    //Create Recording Queue Notifier
    let (rqn_tx, rqn_rx) = tokio::sync::mpsc::channel(100);

    //Deserialize
    let q_schedules = Arc::new(Mutex::new(SchedQueue { items: vec![] }));
    //let rules;

    // Spawn epg_syncer
    tokio::select! {
        _ = epg_sync_startup(q_schedules.clone()) => {  },
        _ = scheduler_startup(q_schedules.clone(), rqn_tx.clone()) => {  },
        _ = recording_pool_startup(rqn_rx) => {  },

        _ = api_startup(q_schedules.clone()) => {  },

        _ = tokio::signal::ctrl_c() => { println!("First signal: gracefully exitting...") }
    }
}
