/// Ser/des for recording_pool. Contents are serialized on drop automatically.

use std::collections::BTreeMap;
use std::io::Read;
use tokio::sync::mpsc::Sender;
use ulid::Ulid;
use crate::recording_pool::recording_task::RecordingTask;
use crate::recording_pool::{RecordControlMessage, RecordingTaskDescription};

pub struct RecTaskQueue
{
    inner: BTreeMap<Ulid, RecordingTask>,
}

impl RecTaskQueue
{
    pub(crate) fn new(tx: Sender<RecordControlMessage>) -> Result<RecTaskQueue, std::io::Error> {
        // Import tasks left behind in the previous session
        let mut inner = {
            let mut str: String = "".to_string();
            std::fs::File::create("q_recording.json")?.read_to_string(&mut str)?;
            let items: Vec<RecordingTaskDescription> = serde_json::from_str(&str).unwrap();

            BTreeMap::<Ulid, RecordingTask>::from_iter(items.into_iter().map(|info| {
                let task_id = Ulid::new();
                (task_id, RecordingTask::new(task_id, info, tx.clone()))
            }))
        };

        Ok(RecTaskQueue { inner })
    }
    pub(crate) fn try_add(&mut self, info: RecordingTaskDescription, tx: Sender<RecordControlMessage>) -> bool {
        if self.inner
            .iter()
            .all(|item| item.1.info.mirakurun_id != info.mirakurun_id)
        {
            let task_id = Ulid::new();
            self.inner.insert(task_id, RecordingTask::new(task_id, info, tx.clone()));
            true
        }
        else { false }
    }
    pub(crate) fn try_remove(&mut self, id: Ulid) -> bool {
        if self.inner.contains_key(&id) {
            self.inner.remove(&id);
            true
        }
        else { false }
    }
    pub(crate) fn at(&self, id: Ulid) -> Option<&RecordingTaskDescription> {
        self.inner.get(&id).map(|f| &f.info)
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = &RecordingTaskDescription> {
        self.inner.iter().map(|f| &f.1.info)
    }
    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut RecordingTaskDescription> {
        self.inner.iter_mut().map(|f| &mut f.1.info)
    }
}


impl Drop for RecTaskQueue
{
    fn drop(&mut self) {
        //Export remaining tasks
        let queue_item_exported: Vec<RecordingTaskDescription> =
            self.iter().map(|f| f.clone()).collect();
        let _result = match serde_json::to_string(&queue_item_exported) {
            Ok(str) => std::fs::write("q_recording.json", str),
            Err(e) => panic!("Serialization failed."),
        };
    }
}
