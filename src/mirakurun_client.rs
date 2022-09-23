use mirakurun_client::apis::channels_api::{
    get_channels, get_services_by_channel, GetChannelsError,
};
use mirakurun_client::apis::configuration::Configuration;
use mirakurun_client::apis::programs_api::{get_programs, GetProgramsError};
use mirakurun_client::apis::Error;
use mirakurun_client::models::{Channel, Program};

pub type ChannelsReturnType = Result<Vec<Channel>, Error<GetChannelsError>>;
pub type ProgramsReturnType = Result<Vec<Program>, Error<GetProgramsError>>;

pub(crate) async fn fetch_channels(c: &Configuration) -> ChannelsReturnType {
    let channels = get_channels(c, None, None, None)?;
    Ok(channels
        .into_iter()
        .map(|channel| {
            let services = get_services_by_channel(c, &*channel.r#type.to_string(), &*channel.channel);
            Channel {
                r#type: channel.r#type,
                channel: channel.channel,
                space: channel.space,
                name: channel.name,
                satellite: channel.satellite,
                services: services.ok(),
                freq: channel.freq,
                polarity: channel.polarity,
                tsmf_rel_ts: channel.tsmf_rel_ts,
            }
        })
        .collect::<Vec<_>>())
}

pub(crate) async fn fetch_programmes(c: &Configuration) -> ProgramsReturnType {
    get_programs(c, None, None, None)
}
