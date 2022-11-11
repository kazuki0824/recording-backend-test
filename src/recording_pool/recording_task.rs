use std::io::Error;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use chrono::{DateTime, Duration, Local};
use log::info;
use pin_project_lite::pin_project;
use tokio::io::AsyncWrite;

use crate::recording_pool::recording_task::{eit_parser::EitParser, io_object::IoObject};
use crate::recording_pool::{RecordingTaskDescription, REC_POOL};

mod eit_parser;
mod io_object;

machine!(
    pub(crate) enum RecordingState {
        A { since: DateTime<Local> },
        B1 { since: DateTime<Local> },
        B2 { since: DateTime<Local> },
        Rec { since: DateTime<Local> },
        Lost { graceful: bool },
    }
);
impl IntoB2 for A {}
impl IntoB2 for B1 {}
impl IntoB2 for B2 {}

impl IntoRec for A {}
impl IntoRec for B1 {}
impl IntoRec for B2 {}

impl A {
    fn on_wait_for_premiere(self, WaitForPremiere { start_at }: WaitForPremiere) -> RecordingState {
        if start_at < Local::now() {
            RecordingState::B1(B1 {
                since: Local::now(),
            })
        } else if self.since + Duration::hours(1) < Local::now() {
            RecordingState::Lost(Lost { graceful: false })
        } else {
            RecordingState::A(A { since: self.since })
        }
    }
}

impl B1 {
    fn on_wait_for_premiere(self, WaitForPremiere { start_at }: WaitForPremiere) -> RecordingState {
        if self.since + Duration::hours(3) < Local::now() {
            RecordingState::Lost(Lost { graceful: false })
        } else {
            RecordingState::B1(B1 { since: self.since })
        }
    }
}

impl B2 {
    fn on_present_program_lost(self, _: PresentProgramLost) -> Lost {
        Lost { graceful: true }
    }
}

transitions!(RecordingState,
    [
        (A, FoundInFollowing) => B2,
        (B1, FoundInFollowing) => B2,
        (B2, FoundInFollowing) => B2,
        (A, FoundInPresent) => Rec,
        (B1, FoundInPresent) => Rec,
        (B2, FoundInPresent) => Rec,
        (B2, PresentProgramLost) => Lost,
        (A, WaitForPremiere) => [A, B1, Lost],
        (B1, WaitForPremiere) => [B1, Lost]
    ]
);
trait IntoB2 {
    fn on_found_in_following(self, _: FoundInFollowing) -> B2
    where
        Self: Sized,
    {
        B2 {
            since: Local::now(),
        }
    }
}

trait IntoRec {
    fn on_found_in_present(self, _: FoundInPresent) -> Rec
    where
        Self: Sized,
    {
        Rec {
            since: Local::now(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FoundInFollowing;

#[derive(Clone, Debug, PartialEq)]
pub struct FoundInPresent;

#[derive(Clone, Debug, PartialEq)]
pub struct PresentProgramLost;

#[derive(Clone, Debug, PartialEq)]
pub struct WaitForPremiere {
    start_at: DateTime<Local>,
}

pin_project! {
    pub(crate) struct RecordingTask {
        #[pin]
        target: IoObject,
        eit: EitParser,
        pub(crate) state: RecordingState,
        pub(crate) id: i64
    }
}

impl RecordingTask {
    pub(crate) async fn new(info: &RecordingTaskDescription) -> Result<Self, Error> {
        let info = info.clone();
        let mut target = info.save_location;
        target.push(format!(
            "{}_{}.m2ts",
            info.program.id,
            info.program
                .name
                .as_ref()
                .unwrap_or(&"untitled".to_string())
        ));
        let target = IoObject::new(target.as_path()).await?;
        Ok(Self {
            target,
            eit: EitParser::new(),
            state: RecordingState::A(A {
                since: Local::now(),
            }),
            id: info.program.id,
        })
    }
}

impl AsyncWrite for RecordingTask {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let me = self.project();

        // Get RecordingDescription. If not exist, return error.
        if let Some(item) = REC_POOL.read().unwrap().at(me.id) {
            //TODO: Evaluate states and control IoObject
            match me.eit.push(buf) {
                None => (),
                Some(eit) => unimplemented!(),
            } //TODO: Eit Parser's result
            me.target.poll_write(cx, buf)
        } else {
            me.target.poll_shutdown(cx).map_ok(|_| 0)
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let me = self.project();
        me.target.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let me = self.project();
        REC_POOL.write().unwrap().try_remove(me.id);
        info!("id: {} is shutting down...", me.id);
        me.target.poll_shutdown(cx)
    }
}
