use chrono::{DateTime, Local};
use crate::recording_pool::RecordingTaskDescription;

struct EitParserInner {}

pub(super) struct EitParser {
    state: EitParserInner,
    buf: [u8; 8192],
}

pub(super) enum EitDetected {
    FoundInP,
    FoundInF,
    NotFound
}

impl EitParser {
    pub fn new() -> Self {
        EitParser {
            state: EitParserInner {},
            buf: [0; 8192],
        }
    }
    pub(super) fn push(&self, buf: &[u8], program: &RecordingTaskDescription) -> EitDetected {
        todo!()
    }
}
