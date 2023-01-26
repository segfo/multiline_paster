use std::{
    fs::OpenOptions,
    io::{BufReader, BufWriter, Read, Write},
    path::PathBuf,
};

use libloading:: Symbol;
use multiline_parser_pluginlib::{plugin::*, result::*};
use toolbox::config_loader::ConfigLoader;
use windows::{
    Win32::Foundation::*,
    Win32::{System::Console::SetConsoleCtrlHandler, UI::WindowsAndMessaging::*},
};
mod kbdhook;
use clap::*;
use kbdhook::*;

type KeyHandlerFunc = unsafe extern "system" fn(u32, KBDLLHOOKSTRUCT) -> PluginResult;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CommandLineArgs {
    /// インストールするDLLファイルパスを指定します。
    #[arg(long="install_dll")]
    install_dll: Option<String>,
}
fn try_install_plugin() -> CommandLineArgs {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && (args[1] == "-h" || args[1] == "--help") {
        let mut t =
            command!().arg(arg!(--install_dll "インストールするDLLファイルパスを指定します。"));
        t.print_help();
        println!("\n⚡アドオンによる追加オプション⚡\n（-h/--helpでヘルプ表示をサポートしているアドオンでのみ表示されます）");
    } else if args.len() > 1 &&(args[1] == "--install_dll"){
        return CommandLineArgs::parse()
    }
    CommandLineArgs { install_dll: None }
}

#[async_std::main]
async fn main() {
    let conf: MasterConfig = ConfigLoader::load_file("config.toml");
    if let Some(install_dll) = try_install_plugin().install_dll {
        let mkdir_result = std::fs::create_dir(&conf.plugin_directory);
        let mut dest_path = PathBuf::from(conf.plugin_directory);
        let dll = PathBuf::from(&install_dll);
        if let Some(file_name) = dll.file_name() {
            dest_path.push(file_name);
            let src = match OpenOptions::new().read(true).write(false).open(file_name) {
                Ok(file) => file,
                Err(e) => {
                    println!("プラグインはインストールされませんでした。({e})");
                    return;
                }
            };
            let dest = match OpenOptions::new()
                .create_new(true)
                .read(false)
                .truncate(true)
                .write(true)
                .open(dest_path)
            {
                Ok(file) => file,
                Err(e) => {
                    let msg = match mkdir_result{
                        Ok(_)=>format!("同名のプラグインがすでにインストールされています。("),
                        Err(e)=>format!("プラグインフォルダがありません。({e} / ")
                    };
                    println!("{}{e})",msg);
                    return;
                }
            };
            let mut buf = Vec::new();
            let mut src = BufReader::new(src);
            if let Err(e) = src.read_to_end(&mut buf) {
                println!("読み込みエラー({e})");
            }
            let mut dest = BufWriter::new(dest);
            if let Err(e) = dest.write(&mut buf) {
                println!("書き込みエラー({e})");
            }
            println!("プラグイン \"{install_dll}\" は正しくインストールされました。");
        };
        return;
    }
    let mut pm = PluginManager::new(&conf.plugin_directory);
    if let Err(e) = pm.load_plugin(&conf.addon_name) {
        println!("メインロジック・アドオンがロードできませんでした。\n{}", e);
        return;
    }
    sethook();
    let mut msg = MSG::default();

    let mut stroke = StrokeMessage::default();
    let kf: Symbol<KeyHandlerFunc> = pm
        .get_plugin_function(&conf.addon_name, "key_down")
        .unwrap();
    let kd = *kf;
    let kf: Symbol<KeyHandlerFunc> = pm.get_plugin_function(&conf.addon_name, "key_up").unwrap();
    let ku = *kf;
    stroke.set_key_down(Box::new(move |keystate, kbdllhook_struct| unsafe {
        kd(keystate, kbdllhook_struct)
    }));
    stroke.set_key_up(Box::new(move |keystate, kbdllhook_struct| unsafe {
        ku(keystate, kbdllhook_struct)
    }));
    set_stroke_callback(stroke);
    if let Ok(init_plugin) = pm.get_plugin_function::<fn()>(&conf.addon_name, "init_plugin") {
        init_plugin()
    }
    unsafe {
        SetConsoleCtrlHandler(Some(exit_handler), BOOL(1));
        while GetMessageW(&mut msg, HWND::default(), 0, 0).into() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn exit_handler(_ctrltype: u32) -> BOOL {
    unhook();
    BOOL(0)
}
