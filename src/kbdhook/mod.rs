use once_cell::unsync::*;
use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::time::Duration;
use std::{
    collections::VecDeque,
    sync::{Mutex, RwLock},
};
use send_input::keyboard::windows::*;
use windows::Win32::{
    Foundation::*,
    System::{DataExchange::*, Memory::*, SystemServices::*, WindowsProgramming::*},
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Clipboard,
    DirectKeyInput,
}
use crate::Config;
use async_std::task;

static mut hook: HHOOK = HHOOK(0);
static mut map: Lazy<RwLock<Vec<bool>>> = Lazy::new(|| RwLock::new(vec![false; 256]));
static mut clipboard: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));
static mut g_mode: Lazy<Mutex<RunMode>> = Lazy::new(|| Mutex::new(RunMode::default()));

#[derive(Debug, PartialEq)]
pub struct RunMode {
    pub input_mode: InputMode,
    pub burst_mode: bool,
    pub tabindex_keyseq: String,
    pub line_delay_msec: u64,
    pub char_delay_msec: u64,
}
impl Default for RunMode {
    fn default() -> Self {
        RunMode {
            input_mode: InputMode::DirectKeyInput,
            burst_mode: false,
            tabindex_keyseq: String::new(),
            line_delay_msec: 200,
            char_delay_msec: 0,
        }
    }
}
impl RunMode {
    pub fn new() -> Self {
        RunMode::default()
    }
    pub fn set_config(&mut self, config: Config) {
        self.tabindex_keyseq = config.tabindex_key;
        self.line_delay_msec = config.line_delay_msec;
        self.char_delay_msec = config.char_delay_msec;
    }
    pub fn set_burst_mode(&mut self, burst_mode: bool) {
        self.burst_mode = burst_mode
    }
    pub fn set_input_mode(&mut self, input_mode: InputMode) {
        self.input_mode = input_mode;
    }
    pub fn is_burst_mode(&self) -> bool {
        self.burst_mode
    }
    pub fn get_input_mode(&self) -> InputMode {
        self.input_mode
    }
    pub fn get_tabindex_keyseq(&self) -> String {
        self.tabindex_keyseq.clone()
    }
    pub fn get_line_delay_msec(&self) -> u64 {
        self.line_delay_msec
    }
    pub fn get_char_delay_msec(&self) -> u64 {
        self.char_delay_msec
    }
}

// クリップボード挿入モードか、DirectInputモードで動作するか選択できるようにする。
pub fn set_mode(mode: RunMode) {
    unsafe {
        let mut locked_gmode = g_mode.lock().unwrap();
        *locked_gmode = mode;
    };
}

#[no_mangle]
pub extern "system" fn hook_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if HC_ACTION as i32 == ncode {
        let keystate = wparam.0 as u32;
        let stroke_msg = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };
        match keystate {
            WM_KEYDOWN => {
                {
                    let lmap = unsafe { &mut map.write().unwrap() };
                    lmap[stroke_msg.vkCode as usize] = true;
                }
                // キーボードイベントで無いもの（ユーザ操作）に限定してペースト操作を行う
                if stroke_msg.flags.0 & (LLKHF_INJECTED.0 | LLKHF_LOWER_IL_INJECTED.0) == 0 {
                    println!("[general key down] ncode={ncode} stroke={stroke_msg:?}");
                    let combokey = judge_combo_key();
                    // コンボキーであった場合は、フックチェーンに流さない。（意図しないキー入力の防止）
                    if combokey {
                        return LRESULT(0);
                    }
                }
            }
            WM_SYSKEYDOWN => {
                let lmap = unsafe { &mut map.write().unwrap() };
                lmap[stroke_msg.vkCode as usize] = true;
            }
            WM_KEYUP => {
                let lmap = unsafe { &mut map.write().unwrap() };
                if stroke_msg.flags.0 & (LLKHF_INJECTED.0 | LLKHF_LOWER_IL_INJECTED.0) == 0 {
                    println!("[general key up] ncode={ncode} stroke={stroke_msg:?}");
                }
                lmap[stroke_msg.vkCode as usize] = false;
            }
            WM_SYSKEYUP => {
                let lmap = unsafe { &mut map.write().unwrap() };
                lmap[stroke_msg.vkCode as usize] = false;
            }
            _ => {}
        }
    }
    unsafe { CallNextHookEx(hook, ncode, wparam, lparam) }
}

static mut thread_mutex: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(0));
fn judge_combo_key() -> bool {
    let lmap = unsafe { &mut map.read().unwrap() };
    // 0xA2:CTRL
    if lmap[0xA2] == true {
        if lmap[0x43] || lmap[0x58] {
            // 0x43:C
            // 0x58:X
            if lmap[VK_LMENU.0 as usize] | lmap[VK_RMENU.0 as usize] {
                task::spawn(reset_clipboard());
            }else{
                task::spawn(copy_clipboard());
            }
            // task::spawn(reset_clipboard());
            return true;
        } else if lmap[0x56] {
            // 0x56: V
            // 基本的に重たい操作なので非同期で行う
            // 意訳：さっさとフックプロシージャから復帰しないとキーボードがハングする。
            task::spawn(write_clipboard());
            return true;
        }
    }
    false
}

async fn copy_clipboard() {
    show_operation_message("コピー");
    // WindowsがCTRL+Cして、クリップボードにデータを格納するまで待機する。
    std::thread::sleep(Duration::from_millis(250));
    let mut cb = unsafe { clipboard.lock().unwrap() };
    let iclip = Clipboard::open();
    unsafe {
        load_data_from_clipboard(&mut *cb);
        reset_clipboard();
    }
}

async fn reset_clipboard() {
    show_operation_message("クリップボードデータの削除");
    let mut cb = unsafe { clipboard.lock().unwrap() };
    cb.clear();
}
struct Clipboard {}
impl Clipboard {
    fn open() -> Self {
        unsafe {
            OpenClipboard(HWND::default());
        }
        Clipboard {}
    }
}
impl Drop for Clipboard {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}

fn show_operation_message<T: Into<String>>(operation: T) {
    let active_window = unsafe { GetForegroundWindow() };
    if active_window.0 != 0 {
        println!(
            "ウィンドウ「{}」上で{}操作が行われました。",
            get_window_text(active_window),
            operation.into()
        );
    } else {
        println!("アクティブウィンドウに対するフォーカスが失われています。");
    }
}

async fn write_clipboard() {
    let mutex = unsafe { thread_mutex.lock().unwrap() };
    unsafe {
        // DropTraitを有効にするために変数に束縛する
        // 束縛先の変数は未使用だが、最適化によってOpenClipboardが実行されなくなるので変数束縛は必ず行う。
        // ここでクリップボードを開いている理由は、CTRL+VによってWindowsがショートカットに反応してペーストしないようにロックする意図がある。
        let iclip = Clipboard::open();
        // クリップボードを開く
        let mut cb = clipboard.lock().unwrap();
        EmptyClipboard();
        if cb.len() == 0 {
            // if let None = load_data_from_clipboard(&mut cb) {
                println!("クリップボードにデータがありません。");
                return;
            // }
        }
        // オプションをロードする
        let (is_burst_mode, tabindex_keyseq, get_line_delay_msec, char_delay_msec) = {
            let mode = g_mode.lock().unwrap();
            (
                mode.is_burst_mode(),
                mode.get_tabindex_keyseq(),
                mode.get_line_delay_msec(),
                mode.get_char_delay_msec(),
            )
        };

        if is_burst_mode {
            let mut kbd = Keyboard::new();
            let len = cb.len();
            kbd.new_delay(char_delay_msec);
            kbd.append_input_chain(
                KeycodeBuilder::default()
                    .vk(VK_LCONTROL.0)
                    .scan_code(virtual_key_to_scancode(VK_LCONTROL))
                    .build(),
            );
            for key in tabindex_keyseq.chars() {
                KeycodeBuilder::default()
                    .char_build(key)
                    .iter()
                    .for_each(|keycode| kbd.append_input_chain(keycode.clone()));
            }
            for _i in 0..len {
                paste(&mut cb);
                kbd.send_key();
                // キーストロークとの間に数ミリ秒の待機時間を設ける
                std::thread::sleep(Duration::from_millis(get_line_delay_msec))
            }
        } else {
            paste(&mut cb);
        }
    }
}

unsafe fn load_data_from_clipboard(cb: &mut VecDeque<String>) -> Option<()> {
    let h_text = GetClipboardData(CF_UNICODETEXT.0);
    match h_text {
        Err(_) => None,
        Ok(h_text) => {
            // クリップボードにデータがあったらロックする
            let p_text = GlobalLock(h_text.0);
            // 今クリップボードにある内容をコピーする（改行で分割される）
            // 後でここの挙動を変えても良さそう。
            // if cb.len() == 0 {
                let text = u16_ptr_to_string(p_text as *const _).into_string().unwrap();
                for line in text.lines() {
                    if line.len() != 0 {
                        cb.push_front(line.to_owned());
                    } else {
                        cb.push_front("".to_owned());
                    }
                }
            // }
            GlobalUnlock(h_text.0);
            Some(())
        }
    }
}

unsafe fn paste(cb: &mut VecDeque<String>) {
    let s = cb.pop_back().unwrap();
    let (input_mode, char_delay_msec) = {
        let mode = g_mode.lock().unwrap();
        (mode.get_input_mode(), mode.get_char_delay_msec())
    };

    show_operation_message("ペースト");
    if input_mode == InputMode::DirectKeyInput {
        let is_key_pressed = |vk: usize| -> bool {
            let lmap = &mut map.read().unwrap();
            lmap[vk]
        };
        // 現在のキーボードの状況（KeyboardLLHookから取得した状況）に合わせて制御キーの解除と設定を行う。
        // その後に、ペースト対象のデータを送る
        // さらに、現在のキーボードの状況に合わせて今度は制御キーを復旧させる。
        let mut kbd = Keyboard::new();
        // CTRLキーを一旦解除する
        kbd.new_delay(char_delay_msec);
        kbd.append_input_chain(
            KeycodeBuilder::default()
                .vk(VK_LCONTROL.0)
                .scan_code(virtual_key_to_scancode(VK_LCONTROL))
                .build(),
        );
        // ペースト対象の文字列を登録する
        for c in s.as_str().chars() {
            KeycodeBuilder::default()
                .char_build(char::from_u32(c as u32).unwrap())
                .iter()
                .for_each(|key_code| kbd.append_input_chain(key_code.clone()));
        }
        // CTRLキーが押されている状況をチェックしてチェーンに登録する
        let mode = if is_key_pressed(162) {
            KeySendMode::KeyDown
        } else {
            KeySendMode::KeyUp
        };
        kbd.append_input_chain(
            KeycodeBuilder::default()
                .vk(VK_LCONTROL.0)
                .scan_code(virtual_key_to_scancode(VK_LCONTROL))
                .key_send_mode(mode)
                .build(),
        );
        kbd.send_key();
    } else {
        let data = OsString::from(s).encode_wide().collect::<Vec<u16>>();
        let strdata_len = data.len() * 2;
        let data_ptr = data.as_ptr();
        let gdata = GlobalAlloc(GHND | GLOBAL_ALLOC_FLAGS(GMEM_SHARE), strdata_len + 2);
        let locked_data = GlobalLock(gdata);
        std::ptr::copy_nonoverlapping(
            data_ptr as *const u8,
            locked_data as *mut u8,
            strdata_len + 2,
        );
        match SetClipboardData(CF_UNICODETEXT.0, HANDLE(gdata)) {
            Ok(_handle) => {
                println!("set clipboard success.");
            }
            Err(e) => {
                println!("SetClipboardData failed. {:?}", e);
            }
        }
        // 終わったらアンロックしてからメモリを開放する
        GlobalUnlock(gdata);
        GlobalFree(gdata);
    }
}

fn virtual_key_to_scancode(vk: VIRTUAL_KEY) -> u16 {
    unsafe { MapVirtualKeyA(vk.0 as u32, MAPVK_VK_TO_VSC as u32) as u16 }
}

fn get_window_text(hwnd: HWND) -> String {
    unsafe {
        // GetWindowTextLengthW + GetWindowTextWは別プロセスへの取得を意図したものではないとの記述がMSDNにあるので
        // SendMessageWで取得することにする。
        let len = SendMessageW(hwnd, WM_GETTEXTLENGTH, WPARAM(0), LPARAM(0)).0 as usize + 1;
        let mut buf = vec![0u16; len];
        SendMessageW(
            hwnd,
            WM_GETTEXT,
            WPARAM(len),
            LPARAM(buf.as_mut_ptr() as isize),
        );
        OsString::from_wide(&buf[0..buf.len() - 1])
            .to_os_string()
            .into_string()
            .unwrap()
    }
}

#[no_mangle]
pub extern "C" fn sethook() -> bool {
    unsafe {
        let dll: HINSTANCE = HINSTANCE(0); // dllの場合はPROCESS_ATTACHされたときのh_instを入れる。
        hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), dll, 0) {
            Ok(handle) => handle,
            Err(_) => {
                return false;
            }
        };
    }
    true
}

#[no_mangle]
pub extern "C" fn unhook() -> bool {
    unsafe {
        if !hook.is_invalid() {
            return UnhookWindowsHookEx(hook).as_bool();
        }
        false
    }
}

unsafe fn u16_ptr_to_string(ptr: *const u16) -> OsString {
    let len = (0..).take_while(|&i| *ptr.offset(i) != 0).count();
    let slice = std::slice::from_raw_parts(ptr, len);
    OsString::from_wide(slice)
}
