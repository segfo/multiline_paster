use windows::{
    Win32::Foundation::*,
    Win32::{
        UI::WindowsAndMessaging::*,
    },
};

mod kbdhook;
use kbdhook::*;
use clap::{*,Parser};
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CommandLineArgs{
    /// 動作モードがクリップボード経由でペーストされます（デフォルト：キーボードエミュレーションでのペースト）
    #[arg( long, default_value_t = false)]
    clipboard:bool
}

fn main() {
    sethook();
    let mut msg = MSG::default();
    let args = CommandLineArgs::parse();
    if args.clipboard{
        set_mode(InputMode::Clipboard);
    }else{
        set_mode(InputMode::DirectKeyInput);
    }
    unsafe {
        while GetMessageW(&mut msg, HWND::default(), 0, 0).into() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
