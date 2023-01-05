use once_cell::unsync::*;
use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::{collections::VecDeque, ffi::CStr, sync::Mutex};
use windows::Win32::UI::Input;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::{DataExchange::*, Memory::*, SystemServices::*, WindowsProgramming::*},
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Clipboard,
    DirectKeyInput,
}

static mut hook: HHOOK = HHOOK(0);

static mut map: Lazy<Mutex<Vec<bool>>> = Lazy::new(|| Mutex::new(vec![false; 256]));
static mut clipboard: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));
static mut g_mode: Lazy<Mutex<InputMode>> = Lazy::new(|| Mutex::new(InputMode::DirectKeyInput));
// クリップボード挿入モードか、DirectInputモードで動作するか選択できるようにする。
pub fn set_mode(mode: InputMode) {
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
                if stroke_msg.flags == KBDLLHOOKSTRUCT_FLAGS(0) {
                    println!("[general key down] ncode={ncode} stroke={stroke_msg:?}");
                    let mut lmap = unsafe { &mut map.lock().unwrap() };
                    lmap[stroke_msg.vkCode as usize] = true;
                    // VK_CONTROL=0xA2
                    // C=0x43
                    // V=0x76
                    judge_combo_key(&mut lmap);
                } else {
                    println!("[general key down] ncode={ncode} stroke={stroke_msg:?}");
                }
            }
            WM_SYSKEYDOWN => {
                let lmap = unsafe { &mut map.lock().unwrap() };
                lmap[stroke_msg.vkCode as usize] = true;
            }
            WM_KEYUP => {
                if stroke_msg.flags == KBDLLHOOKSTRUCT_FLAGS(128) {
                    let lmap = unsafe { &mut map.lock().unwrap() };
                    println!("[general key up] ncode={ncode} stroke={stroke_msg:?}");
                    lmap[stroke_msg.vkCode as usize] = false;
                } else {
                    println!("[general key down] ncode={ncode} stroke={stroke_msg:?}");
                }
            }
            WM_SYSKEYUP => {
                let lmap = unsafe { &mut map.lock().unwrap() };
                lmap[stroke_msg.vkCode as usize] = false;
            }
            _ => {}
        }
    }
    unsafe { CallNextHookEx(hook, ncode, wparam, lparam) }
}

fn judge_combo_key(lmap: &mut Vec<bool>) {
    // 0xA2:CTRL
    if lmap[0xA2] == true {
        if lmap[0x43] || lmap[0x58] {
            // 0x43:C
            // 0x58:X
            println!("copy");
            reset_clipboard();
        } else if lmap[0x56] {
            // 0x56: V
            println!("paste!");
            write_clipboard(lmap);
        }
    }
}

fn reset_clipboard() {
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

fn write_clipboard(lmap: &mut Vec<bool>) {
    unsafe {
        // クリップボードを開く
        let mut cb = clipboard.lock().unwrap();
        // DropTraitを有効にするために変数に束縛する
        // 束縛先の変数は未使用だが、最適化によってOpenClipboardが実行されなくなるので変数束縛は必ず行う。
        let iclip = Clipboard::open();
        if cb.len() == 0 {
            let hText = GetClipboardData(CF_UNICODETEXT.0);
            match hText {
                Err(_) => {
                    println!("クリップボードにデータないよｗ");
                    return;
                }
                Ok(hText) => {
                    // クリップボードにデータがあったらロックする
                    let pText = GlobalLock(hText.0);
                    // 今クリップボードにある内容をコピーする（改行で分割される）
                    // 後でここの挙動を変えても良さそう。

                    if cb.len() == 0 {
                        let text = u16_ptr_to_string(pText as *const _).into_string().unwrap();
                        // println!("copy: {text}");
                        for line in text.lines() {
                            if line.len() != 0 {
                                cb.push_front(line.to_owned());
                            }
                        }
                    }
                    GlobalUnlock(hText.0);
                }
            }
        }
        // コピーしたデータを1行ずつ貼り付ける。
        // コピーしたデータが全部なくなるまでこっちの挙動になる。
        // 嫌なら自分で直して。オープンソースだし。
        EmptyClipboard();
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
        let mode = g_mode.lock().unwrap();
        if *mode == InputMode::DirectKeyInput {
            let active_window = GetForegroundWindow();
            if active_window.0 != 0 {
                println!(
                    "ウィンドウ {} に対してペーストが行われました。",
                    get_window_text(active_window)
                );
            } else {
                println!("アクティブウィンドウに対するフォーカスが失われています。");
            }
            for c in data {
                send_key_input(c as u16, lmap);
            }
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
}

fn control_key(pressed: bool) -> INPUT {
    unsafe {
        let vk = VIRTUAL_KEY(162);
        keyinput_generator(pressed, vk)
    }
}

fn shift_key(pressed: bool) -> INPUT {
    keyinput_generator(pressed, VIRTUAL_KEY(160))
}
fn keyinput_generator(pressed: bool, vk: VIRTUAL_KEY) -> INPUT {
    unsafe {
        let mut kbd = KEYBDINPUT::default();
        let vk = vk;
        kbd.wVk = vk;
        kbd.wScan = MapVirtualKeyA(vk.0 as u32, MAPVK_VK_TO_VSC as u32) as u16;
        kbd.dwFlags = if pressed {
            KEYBD_EVENT_FLAGS(0)
        } else {
            KEYEVENTF_KEYUP
        };
        kbd.time = 0;
        kbd.dwExtraInfo = GetMessageExtraInfo().0 as usize;
        let mut input = INPUT::default();
        input.r#type = INPUT_KEYBOARD;
        input.Anonymous.ki = kbd;
        input
    }
}

fn send_key_input(c: u16, lmap: &mut Vec<bool>) {
    unsafe {
        let mut kbd = KEYBDINPUT::default();
        let mut input_list = Vec::new();
        let kl = GetKeyboardLayout(0);
        let vk = VIRTUAL_KEY(VkKeyScanExA(CHAR(c as u8), kl) as u16);
        input_list.push(control_key(false));
        if c < 0x7f {
            if vk.0 & 0x100 == 0x100 {
                input_list.push(shift_key(true));
            }
            println!(
                "shift key: {} ctrl key: {}",
                (vk.0) & 0x100 == 0x100,
                (vk.0) & 0x200 == 0x200
            );
            kbd.wVk = VIRTUAL_KEY(vk.0 & 0xff);
            kbd.wScan = MapVirtualKeyA(kbd.wVk.0 as u32, MAPVK_VK_TO_VSC as u32) as u16;
            kbd.dwFlags = KEYEVENTF_SCANCODE; //KEYBD_EVENT_FLAGS(0);
        } else {
            kbd.wVk = VIRTUAL_KEY(0);
            kbd.wScan = c;
            kbd.dwFlags = KEYEVENTF_UNICODE;
        }
        kbd.time = 0;
        kbd.dwExtraInfo = GetMessageExtraInfo().0 as usize;
        let mut input = INPUT::default();
        input.r#type = INPUT_KEYBOARD;
        input.Anonymous.ki = kbd;
        input_list.push(input);
        if vk.0 & 0x100 == 0x100 {
            input_list.push(shift_key(false));
        }
        input_list.push(control_key(true));
        // control_key(false, lmap);
        let result = SendInput(&input_list, std::mem::size_of::<INPUT>() as i32);
        // control_key(true, lmap);
    }
}

fn get_window_text(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd) as usize;
        let mut buf = vec![0u16; len];
        GetWindowTextW(hwnd, &mut buf);
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
