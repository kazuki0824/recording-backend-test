use std::error::Error;
use std::fmt::{Display, Formatter};
use mirakurun_client::apis::events_api::get_events_stream;
use mirakurun_client::from_value;
use mirakurun_client::models::{EventResource, Service};
use crate::epg_syncer::ProgramsIndexManager;

#[derive(Debug)]
enum DeErrReason{
    ResourceKindMismatch
}

#[derive(Debug)]
struct DeError {
    reason: DeErrReason
}

impl Error for DeError {}
impl Display for DeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl ProgramsIndexManager {
    pub(crate) async fn update_db_from_stream(&self) -> impl Iterator<Item = Result<Service, Box<dyn Error>>> {
        // subscribe_to_events_api
        get_events_stream(&self.m_conf, None, Some("programs")).unwrap()
            .map(|value| {
                match value {
                    Ok(value) if value.resource == EventResource::Service => {
                        from_value::<Service>(value.data).map_err(|e| Box::new(e) as Box<dyn Error>)
                    },
                    Ok(_) => Err(Box::new( DeError { reason: DeErrReason::ResourceKindMismatch } ) as Box<dyn Error>),
                    Err(e) => Err(Box::new(e) as Box<dyn Error>)
                }
            })
    }
}
