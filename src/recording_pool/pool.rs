/// Ser/des for recording_pool. Contents are serialized on drop automatically.
use std::collections::HashMap;
use std::io::Error;

use futures_util::TryStreamExt;
use log::warn;
use mirakurun_client::apis::configuration::Configuration;
use mirakurun_client::apis::programs_api::get_program_stream;
use structopt::StructOpt;
use tokio::select;
use tokio::sync::oneshot::{Receiver, Sender};
use tokio_util::io::StreamReader;

use crate::recording_pool::recording_task::RecordingTask;
use crate::recording_pool::{RecordingTaskDescription, REC_POOL};
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
    pub(crate) fn add(&mut self, info: RecordingTaskDescription) {
        // 1. Insert RecordingTaskDescription regardless of its existence.
        // 2. Create new task only if there's no abort_handle that has the same id in inner_abort_handle.
        //    In this situation, RecordingTaskDescription should be overwritten.
        let id = info.program.id;

        self.inner.insert(id, info);

        if !self.inner_abort_handle.contains_key(&id) {
            let (tx, rx) = tokio::sync::oneshot::channel();
            tokio::spawn(generate_task(id, rx));

            self.inner_abort_handle.insert(id, tx);
        }
    }
    pub(crate) fn try_add(&mut self, info: RecordingTaskDescription) {
        // 1. Create new task only if there's no abort_handle that has the same id in inner_abort_handle.
        //    In this situation, RecordingTaskDescription should be overwritten.
        // 2. Otherwise, create RecordingTaskDescription if it isn't exist.
        let id = info.program.id;

        let insertion_result = {
            if !self.inner.contains_key(&id) {
                self.inner.insert(id, info);

                if !self.inner_abort_handle.contains_key(&id) {
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    tokio::spawn(generate_task(id, rx));

                    self.inner_abort_handle.insert(id, tx);
                }
                true
            } else {
                false
            }
        };
    }
    pub(crate) fn try_remove(&mut self, id: i64) -> bool {
        let info_removal = self.inner.remove(&id);
        let handle_removal = self
            .inner_abort_handle
            .remove(&id)
            .and_then(|abort| abort.send(()).ok());
        info_removal.is_some() || handle_removal.is_some()
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
        let target = REC_POOL
            .read()
            .unwrap()
            .inner
            .get(&id)
            .expect(
                "A new task cannot be spawned because the RecordingTaskDescription is not found.",
            )
            .clone();

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
