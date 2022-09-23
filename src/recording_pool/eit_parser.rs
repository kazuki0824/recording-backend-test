use chrono::{DateTime, Local};

pub(super) struct EitParser {


}

pub(super) struct Eit {
    pub(super) start: DateTime<Local>,
    pub(super) end: DateTime<Local>

}

impl EitParser {
    pub(super) fn push(&self, buf: &[u8]) -> Option<Eit> {
        None
    }
}