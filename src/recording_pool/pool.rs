/// Ser/des for recording_pool. Contents are serialized on drop automatically.
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use log::{info, warn};

use crate::recording_pool::{RecordControlMessage, RecordingTaskDescription};

pub(crate) struct RecTaskQueue {
    inner: HashMap<i64, RecordingTaskDescription>,
}

impl RecTaskQueue {
    pub(crate) fn new() -> Result<RecTaskQueue, std::io::Error> {
        // Import tasks left behind in the previous session
        let inner = {
            let path = Path::new("./q_recording.json");
            if path.exists() {
                let mut str: String = "".to_string();
                std::fs::File::open(path.canonicalize()?)?.read_to_string(&mut str)?;

                match serde_json::from_str::<HashMap<i64, RecordingTaskDescription>>(&str) {
                    Ok(items) => Some(items),
                    Err(e) => {
                        warn!("{}", e);
                        None
                    }
                }
            } else {
                None
            }
        };

        let inner = inner.unwrap_or_else(|| {
            info!("No valid q_recording.json is found. It'll be created or overwritten just before exiting.");
            HashMap::new()
        });

        info!("q_recording successfully loaded. {} items.", inner.len());
        Ok(RecTaskQueue { inner })
    }
    pub(crate) fn add(&mut self, info: RecordingTaskDescription) -> bool
    {
        self.inner.insert(info.program.id, info);
        true
    }
    pub(crate) fn try_add(
        &mut self,
        info: RecordingTaskDescription,
    ) -> bool {
        if self.inner.contains_key(&info.program.id)
        {
            return false;
        }
        self.inner.insert(info.program.id, info).is_none()
    }
    pub(crate) fn try_remove(&mut self, id: i64) -> bool {
        if self.inner.contains_key(&id) {
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
