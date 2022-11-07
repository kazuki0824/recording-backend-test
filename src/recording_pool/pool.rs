/// Ser/des for recording_pool. Contents are serialized on drop automatically.
use std::collections::HashMap;
use std::io::{Error, Read};
use std::path::{Path, PathBuf};

use futures_util::TryStreamExt;
use log::{info, warn};
use mirakurun_client::apis::configuration::Configuration;
use mirakurun_client::apis::programs_api::get_program_stream;
use structopt::StructOpt;
use tokio::select;
use tokio::sync::oneshot::{Receiver, Sender};
use tokio_util::io::StreamReader;

use crate::recording_pool::recording_task::RecordingTask;
use crate::recording_pool::RecordingTaskDescription;
use crate::Opt;

pub(crate) struct RecTaskQueue {
    inner: HashMap<i64, RecordingTaskDescription>,
    inner_abort_handle: HashMap<i64, Sender<()>>,
}

impl RecTaskQueue {
    pub(crate) fn new() -> Result<RecTaskQueue, Error> {
        // Import tasks left behind in the previous session
        let inner = {
            let path = Path::new("./q_recording.json");
            if path.exists() {
                let mut str: String = "".to_string();
                std::fs::File::open(path.canonicalize()?)?.read_to_string(&mut str)?;

                match serde_json::from_str::<HashMap<i64, RecordingTaskDescription>>(&str) {
                    Ok(items) => {
                        let mut handles = HashMap::new();
                        for x in items.iter() {
                            let (tx, rx) = tokio::sync::oneshot::channel::<()>();
                            tokio::task::spawn(generate_task(x.1.clone(), rx));
                            handles.insert(*x.0, tx);
                        }
                        Some((items, handles))
                    }
                    Err(e) => {
                        warn!("{}", e);
                        None
                    }
                }
            } else {
                None
            }
        };

        if let Some((inner, inner_abort_handle)) = inner {
            info!("q_recording successfully loaded. {} items.", inner.len());
            Ok(RecTaskQueue {
                inner,
                inner_abort_handle,
            })
        } else {
            info!("No valid q_recording.json is found. It'll be created or overwritten just before exiting.");
            Ok(RecTaskQueue {
                inner: HashMap::new(),
                inner_abort_handle: HashMap::new(),
            })
        }
    }
    pub(crate) fn add(&mut self, info: RecordingTaskDescription) -> bool {
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        tokio::task::spawn(generate_task(info.clone(), rx));
        self.inner_abort_handle.insert(info.program.id, tx);
        self.inner.insert(info.program.id, info);
        true
    }
    pub(crate) fn try_add(&mut self, info: RecordingTaskDescription) -> bool {
        if self.inner.contains_key(&info.program.id) {
            return false;
        }
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        tokio::task::spawn(generate_task(info.clone(), rx));
        self.inner_abort_handle.insert(info.program.id, tx);
        self.inner.insert(info.program.id, info).is_none()
    }
    pub(crate) fn try_remove(&mut self, id: i64) -> bool {
        if self.inner.contains_key(&id) {
            self.inner_abort_handle
                .remove(&id)
                .and_then(|h| h.send(()).ok());
            self.inner.remove(&id);
            info!("{} is removed from q_recording", id);
            true
        } else {
            false
        }
    }
    pub(crate) fn at(&self, id: i64) -> Option<&RecordingTaskDescription> {
        self.inner.get(&id)
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = &RecordingTaskDescription> {
        self.inner.values()
    }
    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut RecordingTaskDescription> {
        self.inner.values_mut()
    }
}

impl Drop for RecTaskQueue {
    fn drop(&mut self) {
        //Export remaining tasks
        let queue_item_exported: HashMap<i64, RecordingTaskDescription> =
            self.iter().map(|f| (f.program.id, f.clone())).collect();
        let path = Path::new("./q_recording.json")
            .canonicalize()
            .unwrap_or(PathBuf::from("./q_recording.json"));
        let result = match serde_json::to_string(&queue_item_exported) {
            Ok(str) => std::fs::write(&path, str),
            Err(e) => panic!("Serialization failed. {}", e),
        };
        if result.is_ok() {
            println!("q_recording is saved in {}.", path.display())
        }
    }
}

async fn generate_task(r: RecordingTaskDescription, rx: Receiver<()>) -> Result<(), Error> {
    // m_url
    let args = Opt::from_args();
    let m_url = args.mirakurun_base_uri;

    let mut c = Configuration::new();
    c.base_path = m_url;
    let mut src = match get_program_stream(&c, r.program.id, None, None).await {
        Ok(value) => StreamReader::new(
            value
                .bytes_stream()
                .map_err(|e: mirakurun_client::Error| Error::new(std::io::ErrorKind::Other, e)),
        ),
        Err(e) => return Err(Error::new(std::io::ErrorKind::Other, e)),
    };
    let mut task = RecordingTask::new(r).await?;

    if let Ok(ref mut p) = task.target.child {
        select! {
            Err(e) = tokio::io::copy(p.stdout.as_mut().expect(""), &mut task.target.raw_out) => warn!("{}", e),
            Err(e) = tokio::io::copy(&mut src, p.stdin.as_mut().expect("")) => warn!("{}", e),
            _ = rx => {}
        }
    } else {
        select! {
            _ = tokio::io::copy(&mut src, &mut task.target.raw_out) => {  },
            _ = rx => {  }
        }
    };
    Ok(())
}
