use std::io::{Error, IoSlice};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc::Sender;

use crate::recording_pool::{RecordControlMessage, RecordingTaskDescription};
use crate::recording_pool::recording_task::{eit_parser::EitParser, io_object::OutputObject};

mod eit_parser;
mod io_object;

enum RecordingState {
    A,
    B1,
    B2,
    Rec
}

pin_project! {
    pub(super) struct RecordingTask {
        pub(super) info: RecordingTaskDescription,
        target: OutputObject,
        eit: EitParser,
        state: RecordingState,
        pub(super) task_id: i64
    }
}

impl RecordingTask {
    pub(crate) fn new(
        info: RecordingTaskDescription,
    ) -> Result<Self, Error> {
        let mut target = PathBuf::from(&info.save_location);
        target.push(
            format!("{}_{}.m2ts",
                    info.program.id,
                    info.program.name.as_ref().unwrap_or(&"untitled".to_string()))
        );
        let target = OutputObject::new(target.as_path())?;
        Ok(Self {
            target,
            eit: EitParser::new(),
            state: RecordingState::A,
            task_id: info.program.id,
            info,
        })
    }
}
//
// impl AsyncWrite for RecordingTask {
//     fn poll_write(
//         self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         buf: &[u8],
//     ) -> Poll<Result<usize, Error>> {
//         // Update if EIT[schedule] is received
//         if let Some(eit) = self.eit.push(buf) {
//             //TODO: Update start & duration
//         }
//         if let Ok(ref input) = self.target.child {
//             input
//         }
//         .poll_write(cx, buf)
//     }
//
//     fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
//         let mut me = self.project();
//         Pin::new(
//             &mut me
//                 .child
//                 .stdin
//                 .as_mut()
//                 .expect("The child process has no stdin."),
//         )
//         .poll_flush(cx)
//     }
//
//     fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
//         let mut me = self.project();
//         Pin::new(
//             &mut me
//                 .child
//                 .stdin
//                 .as_mut()
//                 .expect("The child process has no stdin."),
//         )
//         .poll_shutdown(cx)
//     }
//
//     fn poll_write_vectored(
//         self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         bufs: &[IoSlice<'_>],
//     ) -> Poll<Result<usize, Error>> {
//         let mut me = self.project();
//         Pin::new(&mut me.child.stdin.as_mut().expect("")).poll_write_vectored(cx, bufs)
//     }
//
//     fn is_write_vectored(&self) -> bool {
//         self.child.stdin.as_ref().expect("").is_write_vectored()
//     }
// }
