use std::collections::HashMap;

use log::info;
use meilisearch_sdk::errors::Error;
use mirakurun_client::models::related_item::Type;
use mirakurun_client::models::Program;

use crate::db_utils::{push_programs_ranges, push_services_ranges};
use crate::epg_syncer::EpgSyncManager;
use crate::mirakurun_client::{
    fetch_programmes, fetch_services, ProgramsReturnType, ServicesReturnType,
};

impl EpgSyncManager {
    async fn fetch_epg(&self) -> (ServicesReturnType, ProgramsReturnType) {
        let p = fetch_programmes(&self.m_conf).await;
        let s = fetch_services(&self.m_conf).await;
        (s, p)
    }
    async fn get_reverse_event_relay(p: &Vec<Program>) -> HashMap<i32, i32> {
        let mut table = HashMap::new();
        for elem in p {
            if let Some(ref rels) = elem.related_items {
                for r in rels {
                    if let Some(Type::Relay) = r.r#type {
                        table.insert(r.event_id.unwrap(), elem.event_id);
                    }
                }
            }
        }
        table
    }
    pub(crate) async fn refresh_db(&self) -> Result<(), Error> {
        // Periodically updates the list of currently available channels, future programs.
        // This is triggered every 10 minutes.
        let initial_epg = self.fetch_epg().await;
        info!(
            "{:?}",
            push_programs_ranges(&self.index_programs, &initial_epg.1.unwrap()).await?
        );
        info!(
            "{:?}",
            push_services_ranges(&self.index_programs, &initial_epg.0.unwrap()).await?
        );
        Ok(())
    }
}
