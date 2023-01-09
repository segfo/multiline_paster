use std::{fs::OpenOptions, io::Write};

use windows::{Win32::Foundation::*, Win32::UI::WindowsAndMessaging::*};

mod kbdhook;
use clap::{Parser, *};
use kbdhook::*;

#[clap(group(
    ArgGroup::new("run_mode")
        .required(false)
        .args(&["clipboard", "burst"]),
))]
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CommandLineArgs {
    /// 動作モードがクリップボード経由でペーストされます（デフォルト：キーボードエミュレーションでのペースト）
    /// 本モードはバーストモードと排他です。
    #[arg(long, default_value_t = false)]
    clipboard: bool,
    /// バーストモード（フォームに対する連続入力モード）にするか選択できます。
    #[arg(long, default_value_t = false)]
    burst: bool,
}
impl CommandLineArgs {
    fn configure(&self, mut run_mode: RunMode) -> RunMode {
        run_mode.set_burst_mode(self.burst);
        run_mode.set_input_mode(if self.clipboard {
            InputMode::Clipboard
        } else {
            InputMode::DirectKeyInput
        });
        run_mode
    }
}

use serde_derive::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Config {
    tabindex_key: Vec<char>,
}
impl Default for Config {
    fn default() -> Self {
        Config {
            tabindex_key: vec!['\t'],
        }
    }
}
mod loadconfig;
use loadconfig::*;
impl Config {
    // カレントディレクトリに新規作成を試みる
    // もし新規作成できなければ、ホームディレクトリに作成する
    fn load_file(path: &str) -> Self {
        let config = TomlConfigDeserializer::<Config>::from_file(path);
        match config {
            Ok(file) => file,
            Err(_e) => {
                let conf = Config::default();
                match OpenOptions::new()
                    .create_new(true)
                    .truncate(true)
                    .write(true)
                    .read(false)
                    .open(path)
                {
                    Ok(mut file) => {
                        let _r = file.write(toml::to_string(&conf).unwrap().as_bytes());
                    }
                    Err(_e) => {
                        let mut pathbuf = std::fs::canonicalize(&home_dir().unwrap()).unwrap();
                        pathbuf.push(path);
                        match OpenOptions::new()
                            .truncate(true)
                            .create_new(true)
                            .write(true)
                            .read(false)
                            .open(pathbuf)
                        {
                            Ok(mut file) => {
                                let _r = file.write(toml::to_string(&conf).unwrap().as_bytes());
                            }
                            Err(_e) => {}
                        }
                    }
                }
                conf
            }
        }
    }
}

use dirs::home_dir;
#[async_std::main]
async fn main() {
    sethook();
    let mut msg = MSG::default();
    let args = CommandLineArgs::parse();
    let mut mode = args.configure(RunMode::default());
    let config = Config::load_file("config.toml").tabindex_key;
    mode.next_key = config;
    set_mode(mode);
    unsafe {
        while GetMessageW(&mut msg, HWND::default(), 0, 0).into() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
