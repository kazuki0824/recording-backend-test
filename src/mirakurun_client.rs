use mirakurun_client::apis::channels_api::GetChannelsError;
use mirakurun_client::apis::configuration::Configuration;
use mirakurun_client::apis::programs_api::{get_programs, GetProgramsError};
use mirakurun_client::apis::Error;
use mirakurun_client::apis::services_api::{get_services, GetServicesError};
use mirakurun_client::models::{Channel, Program, Service};

pub type ChannelsReturnType = Result<Vec<Channel>, Error<GetChannelsError>>;
pub type ServicesReturnType = Result<Vec<Service>, Error<GetServicesError>>;
pub type ProgramsReturnType = Result<Vec<Program>, Error<GetProgramsError>>;

pub(crate) async fn fetch_services(c: &Configuration) -> ServicesReturnType {
    get_services(c, None, None, None, None, None, None).await
}

pub(crate) async fn fetch_programmes(c: &Configuration) -> ProgramsReturnType {
    get_programs(c, None, None, None).await
}
