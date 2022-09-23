use meilisearch_sdk::errors::Error;

use crate::epg_syncer::ProgramsIndexManager;
use crate::mirakurun_client::{
    ChannelsReturnType, fetch_channels, fetch_programmes, ProgramsReturnType,
};

impl ProgramsIndexManager {
    async fn fetch_epg(&self) -> (ChannelsReturnType, ProgramsReturnType) {
        let p = fetch_programmes(&self.m_conf).await;
        let c = fetch_channels(&self.m_conf).await;
        (c, p)
    }
    pub(crate) async fn refresh_db(&self) -> Result<(), Error> {
        // Periodically updates the list of currently available channels, future programs.
        // This is triggered every 10 minutes.
        let initial_epg = self.fetch_epg().await;
        self.update_programs(initial_epg.1.unwrap()).await
    }
}
