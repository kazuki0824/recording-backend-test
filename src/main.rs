use crate::{
    epg_syncer::epg_sync_startup,
    sched_trigger::scheduler_startup,
    recording_pool::recording_pool_startup
};

// mod es;
mod api;
mod epg_syncer;
mod mirakurun_client;
mod recording_pool;
mod sched_trigger;

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    //Create Recording Queue Notifier
    let (rqn_tx, mut rqn_rx) = tokio::sync::mpsc::channel(100);

    // Spawn epg_syncer
    tokio::select! {
        _ = epg_sync_startup() => {  },
        _ = scheduler_startup(rqn_tx.clone()) => {  },
        _ = recording_pool_startup(rqn_tx, rqn_rx) => {  }

        _ = tokio::signal::ctrl_c() => {  }
    }

}
