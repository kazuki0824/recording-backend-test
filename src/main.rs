use crate::{
    epg_syncer::epg_sync_startup,
    sched_trigger::scheduler_startup,
    recording_pool::recording_pool_startup,
    api::api_startup
};

mod api;
mod epg_syncer;
mod mirakurun_client;
mod recording_pool;
mod sched_trigger;
mod recording_planner;

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    env_logger::init();

    //Create Recording Queue Notifier
    let (rqn_tx, rqn_rx) = tokio::sync::mpsc::channel(100);

    // Spawn epg_syncer
    tokio::select! {
        _ = epg_sync_startup() => {  },
        _ = scheduler_startup(rqn_tx.clone()) => {  },
        _ = recording_pool_startup(rqn_tx, rqn_rx) => {  },

        _ = api_startup() => {  },

        _ = tokio::signal::ctrl_c() => { println!("First signal: gracefully exitting...") }
    }

}
