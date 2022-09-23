use std::time::Duration;

use log::info;
use meilisearch_sdk;
use meilisearch_sdk::client::Client;
use meilisearch_sdk::errors::Error;
use meilisearch_sdk::indexes::Index;
use mirakurun_client::apis::configuration::Configuration;
use mirakurun_client::apis::programs_api::get_program;
use mirakurun_client::models::Program;
use tokio_stream::StreamExt;

use crate::sched_trigger::Q_RESERVED;

mod events_stream;
mod periodic_tasks;
mod query;

pub(crate) async fn epg_sync_startup() {
    let tracker =
        ProgramsIndexManager::new("http://localhost:40772/api", "http://localhost:7700/").await.unwrap();

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
        // Subscribe NDJSON here.
        // Store programs data into DB, and keep track of them using Mirakurun's Events API.
        let stream = tracker.update_db_from_stream();
        let mut stream = tokio_stream::iter(stream.await);

        // filter
        loop {
            match stream.next().await
            {
                Some(Ok(value)) => {
                    let id = value.id;
                    let p = get_program(&tracker.m_conf, id).expect("TODO: panic message");
                    tracker.update_programs(vec![p]).await.unwrap();
                    continue
                },
                Some(Err(e)) => {
                    return Err(e)
                },
                None => return Ok(())
            }
        }
    };


    tokio::select! {
        _ = periodic => {  },
        // _ = event => {  },
    }
}


///
struct ProgramsIndexManager {
    m_conf: Configuration,
    search_client: Client,
    index: Index,
    // TODO: channels_cache
    // TODO: programs_cache
}

impl ProgramsIndexManager {
    async fn new<S: Into<String> + Sized, T: Into<String> + Sized>(
        m_url: S,
        db_url: T,
    ) -> Result<ProgramsIndexManager, Error> {
        // Initialize Mirakurun
        let mut m_conf = Configuration::new();
        m_conf.base_path = m_url.into();

        // Initialize Meilisearch
        let search_client = Client::new(db_url, "masterKey");

        // Try to get the inner index if the task succeeded
        let index = match search_client.get_index("_programs").await {
            Ok(index) => index,
            Err(e) => {
                let task = search_client
                    .create_index("_programs", Some("id"))
                    .await?;
                let task = task
                    .wait_for_completion(&search_client, None, None)
                    .await?;
                task.try_make_index(&search_client).unwrap()
            }
        };

        Ok(Self {
            m_conf,
            search_client,
            index,
        })
    }

    async fn update_programs(&self, item_delta: Vec<Program>) -> Result<(), Error> {
        // Update Meilisearch
        let task = self.index.add_or_update(&item_delta, Some("id")).await?;
        task.wait_for_completion(&self.search_client, None, Some(Duration::from_secs(60))).await.unwrap();

        // Update the queued reservation if matches
        // Q_RESERVED.lock()
        Ok(())
    }
}
