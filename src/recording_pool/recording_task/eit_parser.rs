use chrono::{DateTime, Local};

struct EitParserInner {}

pub(super) struct EitParser {
    state: EitParserInner,
    buf: [u8; 8192],
}

pub(super) struct Eit {
    pub(super) start: DateTime<Local>,
    pub(super) end: DateTime<Local>,
}

impl EitParser {
    pub fn new() -> Self {
        EitParser {
            state: EitParserInner {},
            buf: [0; 8192],
        }
    }
    pub(super) fn push(&self, buf: &[u8]) -> Option<Eit> {
        None
    }
}
