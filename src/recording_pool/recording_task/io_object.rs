use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Error, IoSlice};
use std::path::Path;
use std::pin::Pin;
use std::process::{Child, Command};
use std::task::{Context, Poll};
use log::warn;
use tokio::io::AsyncWrite;

pub(crate) struct OutputObject {
    pub(crate) child: Result<Child, Error>,
    pub(crate) raw_out: BufWriter<File>,
}

impl OutputObject {
    pub(crate) fn new(output: &Path) -> Result<Self, Error> {
        let mut raw_out = BufWriter::new(OpenOptions::new().create(true).append(true).write(true).open(output)?);

        let child = Command::new(
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
            .spawn();
        if let Err(ref e) = child { warn!("{}", e) }
        Ok(Self {
            raw_out,
            child
        })
    }
}


