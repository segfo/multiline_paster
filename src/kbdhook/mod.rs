use once_cell::unsync::*;
use std::f32::consts::E;
use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::time::Duration;
use std::{
    collections::VecDeque,
    sync::{Mutex, RwLock},
};

use windows::Win32::{
    Foundation::*,
    System::{DataExchange::*, Memory::*, SystemServices::*, WindowsProgramming::*},
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

// キーボード入力の発行元を簡易的に識別するためのメタデータ
// 値については特に意味はない。ただし、0以外であること（ハードウェアキーボードは0のため）
enum KeyExtraInfo {
    EmulateKeyData = 12345,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Clipboard,
    DirectKeyInput,
}

static mut hook: HHOOK = HHOOK(0);
static mut map: Lazy<RwLock<Vec<bool>>> = Lazy::new(|| RwLock::new(vec![false; 256]));
static mut clipboard: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));
static mut g_mode: Lazy<Mutex<RunMode>> = Lazy::new(|| Mutex::new(RunMode::default()));

#[derive(Debug, PartialEq)]
pub struct RunMode {
    input_mode: InputMode,
    burst_mode: bool,
}
impl Default for RunMode {
    fn default() -> Self {
        RunMode {
            input_mode: InputMode::DirectKeyInput,
            burst_mode: false,
        }
    }
}
impl RunMode {
    pub fn new() -> Self {
        RunMode::default()
    }
    pub fn set_burst_mode(&mut self, enable: bool) {
        self.burst_mode = enable;
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
}

// クリップボード挿入モードか、DirectInputモードで動作するか選択できるようにする。
pub fn set_mode(mode: RunMode) {
    unsafe {
        let mut locked_gmode = g_mode.lock().unwrap();
        *locked_gmode = mode;
    };
}
use async_std::task;
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
            reset_clipboard();
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

fn reset_clipboard() {
    show_operation_message("コピー");
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
            "ウィンドウ 「{}」 に対して{}が行われました。",
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
        let iclip = Clipboard::open();
        // クリップボードを開く
        let mut cb = clipboard.lock().unwrap();

        if cb.len() == 0 {
            if let None = load_data_from_clipboard(&mut cb) {
                println!("クリップボードにデータがありません。");
                return;
            }
        }
        EmptyClipboard();
        // バーストモード
        // 将来的にはTAB以外でもできるようにする。
        // 今は仮の姿
        let next_key = vec![
            (VK_LCONTROL, virtual_key_to_scancode(VK_CONTROL)),
            (VK_TAB, virtual_key_to_scancode(VK_TAB)),
        ];
        let is_burst_mode = g_mode.lock().unwrap().is_burst_mode();
        if is_burst_mode {
            let len = cb.len();
            for _i in 0..len {
                paste(&mut cb);
                send_next_key(&next_key);
            }
        } else {
            paste(&mut cb);
        }
    }
}

unsafe fn load_data_from_clipboard(cb: &mut VecDeque<String>) -> Option<()> {
    let hText = GetClipboardData(CF_UNICODETEXT.0);
    match hText {
        Err(_) => None,
        Ok(hText) => {
            // クリップボードにデータがあったらロックする
            let pText = GlobalLock(hText.0);
            // 今クリップボードにある内容をコピーする（改行で分割される）
            // 後でここの挙動を変えても良さそう。
            if cb.len() == 0 {
                let text = u16_ptr_to_string(pText as *const _).into_string().unwrap();
                for line in text.lines() {
                    if line.len() != 0 {
                        cb.push_front(line.to_owned());
                    }
                }
            }
            GlobalUnlock(hText.0);
            Some(())
        }
    }
}

fn key_input_generator(vk: VIRTUAL_KEY, char_code: u16) -> Vec<INPUT> {
    if char_code < 0x7f {
        let vk = VIRTUAL_KEY(vk.0 & 0xff);
        vec![
            keyinput_generator_ascii(true, vk),
            keyinput_generator_ascii(false, vk),
        ]
    } else {
        vec![
            key_input_generator_unicode(true, char_code),
            key_input_generator_unicode(false, char_code),
        ]
    }
}
// 使う側視点での「次のキー」
// 入力先UI的に次に進むキーを投げるためのもの。
fn send_next_key(nextkey: &Vec<(VIRTUAL_KEY, u16)>) {
    for (vk, char_code) in nextkey {
        let input = key_input_generator(*vk, *char_code);
        let _result = unsafe { SendInput(&input, std::mem::size_of::<INPUT>() as i32) };
    }
}

unsafe fn paste(cb: &mut VecDeque<String>) {
    let data = OsString::from(cb.pop_back().unwrap())
        .encode_wide()
        .collect::<Vec<u16>>();
    let strdata_len = data.len() * 2;
    let data_ptr = data.as_ptr();
    let gdata = GlobalAlloc(GHND | GLOBAL_ALLOC_FLAGS(GMEM_SHARE), strdata_len + 2);
    let locked_data = GlobalLock(gdata);
    std::ptr::copy_nonoverlapping(
        data_ptr as *const u8,
        locked_data as *mut u8,
        strdata_len + 2,
    );
    let input_mode = g_mode.lock().unwrap().get_input_mode();
    if input_mode == InputMode::DirectKeyInput {
        show_operation_message("ペースト");
        let get_key_state = |vk: usize| -> bool {
            let lmap = &mut map.read().unwrap();
            lmap[vk]
        };
        // 現在のキーボードの状況（KeyboardLLHookから取得した状況）に合わせて制御キーの解除と設定を行う。
        // その後に、ペースト対象のデータを送る
        // さらに、現在のキーボードの状況に合わせて今度は制御キーを復旧させる。
        let mut input_list = Vec::new();
        for c in data {
            send_key_input(c as u16);
        }
        if get_key_state(162) {
            input_list.push(control_key(true));
        } else {
            input_list.push(control_key(false));
        }
        let _result = SendInput(&input_list, std::mem::size_of::<INPUT>() as i32);
    } else {
        match SetClipboardData(CF_UNICODETEXT.0, HANDLE(gdata)) {
            Ok(_handle) => {
                println!("set clipboard success.");
            }
            Err(e) => {
                println!("SetClipboardData failed. {:?}", e);
            }
        }
    }
    // 終わったらアンロックしてからメモリを開放する
    GlobalUnlock(gdata);
    GlobalFree(gdata);
}

fn control_key(pressed: bool) -> INPUT {
    keyinput_generator_ascii(pressed, VIRTUAL_KEY(162))
}

fn shift_key(pressed: bool) -> INPUT {
    keyinput_generator_ascii(pressed, VIRTUAL_KEY(160))
}

fn virtual_key_to_scancode(vk: VIRTUAL_KEY) -> u16 {
    unsafe { MapVirtualKeyA(vk.0 as u32, MAPVK_VK_TO_VSC as u32) as u16 }
}
fn keyinput_generator_ascii(pressed: bool, vk: VIRTUAL_KEY) -> INPUT {
    unsafe {
        let vk = VIRTUAL_KEY(vk.0 & 0xff);
        keyinput_generator_detail(
            vk,
            virtual_key_to_scancode(vk),
            if pressed {
                KEYEVENTF_SCANCODE
            } else {
                KEYBD_EVENT_FLAGS(KEYEVENTF_KEYUP.0 | KEYEVENTF_SCANCODE.0)
            },
        )
    }
}

fn key_input_generator_unicode(pressed: bool, scan: u16) -> INPUT {
    keyinput_generator_detail(
        VIRTUAL_KEY(0),
        scan,
        if pressed {
            KEYEVENTF_UNICODE
        } else {
            KEYBD_EVENT_FLAGS(KEYEVENTF_KEYUP.0 | KEYEVENTF_UNICODE.0)
        },
    )
}

fn keyinput_generator_detail(vk: VIRTUAL_KEY, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    let mut kbd = KEYBDINPUT::default();
    let vk = vk;
    kbd.wVk = vk;
    kbd.wScan = scan;
    kbd.dwFlags = flags;
    kbd.time = 0;
    // ExtraInfoは特に意味のある値ではない。
    // このアプリから生成されたことを主張するだけの値。（物理キーの入力ではないという印）
    // もちろん他のアプリがこの値を設定してたら区別はつかないだろう。
    // ただし、物理キーボード入力は常に0であるのでそれとかぶらなければ正直何でも良いので12345という値にしている。
    kbd.dwExtraInfo = 12345;

    let mut input = INPUT::default();
    input.r#type = INPUT_KEYBOARD;
    input.Anonymous.ki = kbd;
    input
}

fn send_key_input(c: u16) {
    unsafe {
        let mut input_list = Vec::new();
        let kl = GetKeyboardLayout(0);
        let vk = VIRTUAL_KEY(VkKeyScanExA(CHAR(c as u8), kl) as u16);

        input_list.push(control_key(false));
        if vk.0 & 0x100 == 0x100 {
            input_list.push(shift_key(true));
        }
        input_list.append(&mut key_input_generator(vk, c));
        if vk.0 & 0x100 == 0x100 {
            input_list.push(shift_key(false));
        }
        input_list.push(control_key(true));
        let _result = SendInput(&input_list, std::mem::size_of::<INPUT>() as i32);
    }
}

fn get_window_text(hwnd: HWND) -> String {
    unsafe {
        // GetWindowTextLengthW + GetWindowTextWは別プロセスへの取得を意図したものではないとの記述がMSDNにあるので
        // SendMessageWで取得することにする。
        let len = (SendMessageW(hwnd, WM_GETTEXTLENGTH, WPARAM(0), LPARAM(0)).0 as usize + 1) * 2;
        let mut buf = vec![0u16; len];
        SendMessageW(
            hwnd,
            WM_GETTEXT,
            WPARAM(len / 2),
            LPARAM(buf.as_mut_ptr() as isize),
        );
        OsString::from_wide(&buf)
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
