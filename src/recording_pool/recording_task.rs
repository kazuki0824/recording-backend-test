use std::io::{Error, IoSlice};
use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::process::{Child, Command};
use tokio::sync::mpsc::Sender;
use ulid::Ulid;

use crate::recording_pool::{EitParser, RecordControlMessage, RecordingTaskDescription};

pin_project! {
    pub(super) struct RecordingTask {
        #[pin]
        eit_update_request_sender: Sender<RecordControlMessage>,
        pub(super) info: RecordingTaskDescription,
        #[pin]
        child: Child,
        eit: EitParser,
        pub(super) task_id: Ulid
    }
}

impl RecordingTask {
    pub(crate) fn new(
        task_id: Ulid,
        info: RecordingTaskDescription,
        sender: Sender<RecordControlMessage>,
    ) -> RecordingTask {
        RecordingTask {
            eit_update_request_sender: sender,
            info,
            child: Command::new(
                "/home/maleicacid/CLionProjects/recorder-backend-rs/target/debug/tsreadex",
            )
            .args(vec![
                // 取り除く TS パケットの10進数の PID
                // EIT の PID を指定
                "-x", "18/38/39",
                // 特定サービスのみを選択して出力するフィルタを有効にする
                // 有効にすると、特定のストリームのみ PID を固定して出力される
                "-n", "-1",
                // 主音声ストリームが常に存在する状態にする
                // ストリームが存在しない場合、無音の AAC ストリームが出力される
                // 音声がモノラルであればステレオにする
                // デュアルモノを2つのモノラル音声に分離し、右チャンネルを副音声として扱う
                "-a", "13",
                // 副音声ストリームが常に存在する状態にする
                // ストリームが存在しない場合、無音の AAC ストリームが出力される
                // 音声がモノラルであればステレオにする
                "-b", "5",
                // 字幕ストリームが常に存在する状態にする
                // ストリームが存在しない場合、PMT の項目が補われて出力される
                "-c", "1",
                // 文字スーパーストリームが常に存在する状態にする
                // ストリームが存在しない場合、PMT の項目が補われて出力される
                "-u", "1",
                // 字幕と文字スーパーを aribb24.js が解釈できる ID3 timed-metadata に変換する
                // +4: FFmpeg のバグを打ち消すため、変換後のストリームに規格外の5バイトのデータを追加する
                // +8: FFmpeg のエラーを防ぐため、変換後のストリームの PTS が単調増加となるように調整する
                "-d", "13",
            ])
            .kill_on_drop(true)
            .spawn()
            .expect("Spawning subprocess `tsreadex` failed."),
            eit: EitParser::new(),
            task_id,
        }
    }
}

impl AsyncRead for RecordingTask {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let mut me = self.project();
        Pin::new(
            me.child
                .stdout
                .as_mut()
                .expect("The child process has no stdout."),
        )
        .poll_read(cx, buf)
    }
}

impl AsyncWrite for RecordingTask {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        // Update if EIT[schedule] is received
        if let Some(eit) = self.eit.push(buf) {
            let message = RecordControlMessage::Update((self.task_id, eit.start, eit.end));

            self.eit_update_request_sender
                .blocking_send(message)
                .unwrap();
        }
        let mut me = self.project();
        Pin::new(
            &mut me
                .child
                .stdin
                .as_mut()
                .expect("The child process has no stdin."),
        )
        .poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let mut me = self.project();
        Pin::new(
            &mut me
                .child
                .stdin
                .as_mut()
                .expect("The child process has no stdin."),
        )
        .poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let mut me = self.project();
        Pin::new(
            &mut me
                .child
                .stdin
                .as_mut()
                .expect("The child process has no stdin."),
        )
        .poll_shutdown(cx)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<Result<usize, Error>> {
        let mut me = self.project();
        Pin::new(&mut me.child.stdin.as_mut().expect("")).poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.child.stdin.as_ref().expect("").is_write_vectored()
    }
}
