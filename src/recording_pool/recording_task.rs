use std::io::Error;
use std::path::PathBuf;

use pin_project_lite::pin_project;

use crate::recording_pool::recording_task::{eit_parser::EitParser, io_object::OutputObject};
use crate::recording_pool::RecordingTaskDescription;

mod eit_parser;
mod io_object;

enum RecordingState {
    A,
    B1,
    B2,
    Rec,
}

pin_project! {
    pub(super) struct RecordingTask {
        pub(super) info: RecordingTaskDescription,
        pub(super) target: OutputObject,
        eit: EitParser,
        state: RecordingState,
        pub(super) task_id: i64
    }
}

impl RecordingTask {
    pub(crate) async fn new(info: RecordingTaskDescription) -> Result<Self, Error> {
        let mut target = PathBuf::from(&info.save_location);
        target.push(format!(
            "{}_{}.m2ts",
            info.program.id,
            info.program
                .name
                .as_ref()
                .unwrap_or(&"untitled".to_string())
        ));
        let target = OutputObject::new(target.as_path()).await?;
        Ok(Self {
            target,
            eit: EitParser::new(),
            state: RecordingState::A,
            task_id: info.program.id,
            info,
        })
    }
}
