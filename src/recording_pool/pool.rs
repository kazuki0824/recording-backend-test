/// Ser/des for recording_pool. Contents are serialized on drop automatically.
use std::collections::HashMap;
use std::io::{Error, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use futures_util::TryStreamExt;
use log::{debug, info, warn};
use mirakurun_client::apis::configuration::Configuration;
use mirakurun_client::apis::programs_api::get_program_stream;
use structopt::StructOpt;
use tokio::select;
use tokio::sync::oneshot::{Receiver, Sender};
use tokio_util::io::StreamReader;

use crate::recording_pool::recording_task::RecordingTask;
use crate::recording_pool::{REC_POOL, RecordingTaskDescription};
use crate::Opt;

#[derive(Default)]
pub(crate) struct RecTaskQueue {
    inner: HashMap<i64, RecordingTaskDescription>,
    inner_abort_handle: HashMap<i64, Sender<()>>,
}

impl RecTaskQueue {
    pub(crate) fn new() -> Self {
        Self::default()
    }
    pub(crate) fn at(&self, id: &i64) -> Option<&RecordingTaskDescription> {
        self.inner.get(&id)
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = &RecordingTaskDescription> {
        self.inner.values()
    }
    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut RecordingTaskDescription> {
        self.inner.values_mut()
    }
}

async fn generate_task(id: i64, rx: Receiver<()>) -> Result<(), Error> {
    let (mut src, mut rec) = {
        let target = REC_POOL.lock().await.inner.get(&id).expect("A new task cannot be spawned because the RecordingTaskDescription is not found.").clone();

        // Create a new task
        let rec = RecordingTask::new(&target).await?;

        let args = Opt::from_args();
        let m_url = args.mirakurun_base_uri;
        let mut c = Configuration::new();
        c.base_path = m_url;
        // Get Ts Stream
        let src = match get_program_stream(&c, target.program.id, None, None).await {
            Ok(value) => StreamReader::new(
                value
                    .bytes_stream()
                    .map_err(|e: mirakurun_client::Error| Error::new(std::io::ErrorKind::Other, e)),
            ),
            Err(e) => return Err(Error::new(std::io::ErrorKind::Other, e)),
        };
        (src, rec)
    };

    select! {
        // If value is removed, abort the transmission.
        //_ = || async{ while let Some(_) = REC_POOL.lock().await.inner.get(&id) {} }=> {},
        _ = rx => {},
        // Stream connection
        Err(e) = tokio::io::copy(&mut src, &mut rec) => warn!("{}", e)
    }

    Ok(())
}
