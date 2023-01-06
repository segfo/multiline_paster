use windows::{Win32::Foundation::*, Win32::UI::WindowsAndMessaging::*};

mod kbdhook;
use clap::{Parser, *};
use kbdhook::*;
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CommandLineArgs {
    /// 動作モードがクリップボード経由でペーストされます（デフォルト：キーボードエミュレーションでのペースト）
    #[arg(long, default_value_t = false)]
    clipboard: bool,
    /// バーストモード（フォームに対する連続入力モード）にするか選択できます。
    #[arg(long,default_value_t=false)]
    burst:bool
}

#[async_std::main]
async fn main() {
    sethook();
    let mut msg = MSG::default();
    let args = CommandLineArgs::parse();
    let mut run_mode = RunMode::default();
    if args.clipboard {
        run_mode.set_input_mode(InputMode::Clipboard)
    }
    if args.burst{
        run_mode.set_burst_mode(true)
    }
    set_mode(run_mode);
    unsafe {
        while GetMessageW(&mut msg, HWND::default(), 0, 0).into() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
