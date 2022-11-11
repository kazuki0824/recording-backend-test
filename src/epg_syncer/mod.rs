use std::sync::Arc;
use std::time::Duration;

use log::{debug, error, info};
use meilisearch_sdk;
use meilisearch_sdk::client::Client;
use meilisearch_sdk::errors::Error;
use meilisearch_sdk::indexes::Index;
use mirakurun_client::apis::configuration::Configuration;
use mirakurun_client::models::event::EventContent::{Program, Service, Tuner};
use structopt::StructOpt;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use crate::db_utils::{push_programs_ranges, push_services_ranges};
use crate::{Opt, SchedQueue};

mod events_stream;
mod periodic_tasks;

pub(crate) async fn epg_sync_startup(sched_ptr: Arc<Mutex<SchedQueue>>) {
    let args = Opt::from_args();
    EpgSyncManager::new(
        args.mirakurun_base_uri,
        args.meilisearch_base_uri,
        Some(sched_ptr),
    )
    .await
    .unwrap();
}

///
pub(crate) struct EpgSyncManager {
    m_conf: Configuration,
    search_client: Client,
    index_programs: Index,
    index_services: Index,
    sched_ptr: Option<Arc<Mutex<SchedQueue>>>,
}

impl EpgSyncManager {
    pub(crate) async fn new<S: Into<String> + Sized, T: Into<String> + Sized>(
        m_url: S,
        db_url: T,
        sched_ptr: Option<Arc<Mutex<SchedQueue>>>,
    ) -> Result<(), Error> {
        // Initialize Mirakurun
        let mut m_conf = Configuration::new();
        m_conf.base_path = m_url.into();

        // Initialize Meilisearch
        let search_client = Client::new(db_url, "masterKey");

        // Try to get the inner index if the task succeeded
        let index_programs = match search_client.get_index("_programs").await {
            Ok(index) => index,
            Err(_) => {
                let task = search_client.create_index("_programs", Some("id")).await?;
                let task = task.wait_for_completion(&search_client, None, None).await?;
                task.try_make_index(&search_client).unwrap()
            }
        };
        let index_services = match search_client.get_index("_services").await {
            Ok(index) => index,
            Err(_) => {
                let task = search_client.create_index("_services", Some("id")).await?;
                let task = task.wait_for_completion(&search_client, None, None).await?;
                task.try_make_index(&search_client).unwrap()
            }
        };

        let tracker = Self {
            m_conf,
            search_client,
            index_programs,
            index_services,
            sched_ptr,
        };

        let periodic = async {
            let sec = 600;
            info!("Periodic EPG update is running every {} seconds.", sec);
            loop {
                tracker.refresh_db().await.expect("TODO: panic message");
                info!("refresh_db() succeeded.");

                tokio::time::sleep(Duration::from_secs(sec)).await;
            }
        };

        let event = async {
            'outer: loop {
                // Subscribe NDJSON here.
                // Store programs data into DB, and keep track of them using Mirakurun's Events API.
                let mut stream = match tracker.update_db_from_stream().await {
                    Ok(value) => value,
                    Err(e) => {
                        error!("{:#?}", e);
                        error!("Reconecting to Events API.");
                        continue;
                    }
                };

                // filter
                'inner: loop {
                    let next_str = match stream.next().await {
                        Some(Ok(line)) => {
                            if line.trim().eq_ignore_ascii_case("[") {
                                continue;
                            };
                            debug!("{}", line);
                            info!("length = {}", line.len());
                            line
                        }
                        _ => continue,
                    };

                    match serde_json::from_str(&next_str) {
                        Ok(Service(value)) => {
                            info!("Updating the service: {:#?}", value);
                            match push_services_ranges(&tracker.index_services, &vec![value]).await
                            {
                                Ok(_) => info!("Updates have been successfully applied."),
                                Err(e) => error!("{}", e),
                            }
                            continue;
                        }
                        Ok(Program(value)) => {
                            info!("EIT[p/f] from Mirakurun. \n{:?}", &value);
                            match push_programs_ranges(
                                &tracker.index_programs,
                                &vec![value.clone()],
                            )
                            .await
                            {
                                Ok(_) => {
                                    info!("Updates have been successfully applied.");
                                    // Update schedules
                                    if let Some(sched_ptr) = &tracker.sched_ptr {
                                        sched_ptr.lock().await.items.iter_mut().for_each(
                                            |mut f| {
                                                if value.id == f.program.id {
                                                    //TODO: Print details
                                                    f.program.start_at = value.start_at;
                                                    f.program.duration = value.duration;
                                                }
                                            },
                                        );
                                    }
                                }
                                Err(e) => error!("{}", e),
                            }
                            continue;
                        }
                        Ok(Tuner(value)) => {
                            info!("Tuner configuration has been changed. {:?}", value)
                        }
                        Err(e) => {
                            error!("In /events, {}", e);
                            break 'inner;
                        }
                    }
                }
                info!("Reconnecting to /events")
            }
        };

        tokio::select! {
            _ = periodic => {  },
            _ = event => {  },
        };

        Ok(())
    }
}
