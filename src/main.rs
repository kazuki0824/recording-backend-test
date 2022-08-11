use crate::epg_syncer::epg_sync_startup;

// mod es;
mod api;
mod epg_syncer;
mod mirakurun_client;
mod recording_pool;
mod time_trigger;

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    // TODO: https://tokio.rs/tokio/topics/shutdown
    // match signal::ctrl_c().await {
    //     Ok(()) => {},
    //     Err(err) => {
    //         eprintln!("Unable to listen for shutdown signal: {}", err);
    //         // we also shut down in case of error
    //     },
    // }

    // Spawn epg_syncer
    epg_sync_startup().await;



}
