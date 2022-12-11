use once_cell::unsync::*;
use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::{collections::VecDeque, ffi::CStr, sync::Mutex};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::{DataExchange::*, Memory::*, SystemServices::*, WindowsProgramming::*},
        UI::WindowsAndMessaging::*,
    },
};
static mut hook: HHOOK = HHOOK(0);

static mut map: Lazy<Mutex<Vec<bool>>> = Lazy::new(|| Mutex::new(vec![false; 256]));
static mut clipboard: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

#[no_mangle]
pub extern "system" fn hook_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if HC_ACTION == 0 {
        let keystate = wparam.0 as u32;
        let stroke_msg = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };
        match keystate {
            WM_KEYDOWN => {
                println!("[general key] ncode={ncode} stroke={stroke_msg:?}");
                let lmap = unsafe { &mut map.lock().unwrap() };
                lmap[stroke_msg.vkCode as usize] = true;
                // VK_CONTROL=0xA2
                // C=0x43
                // V=0x76
                judge_combo_key(&lmap);
            }
            WM_SYSKEYDOWN => {
                let lmap = unsafe { &mut map.lock().unwrap() };
                lmap[stroke_msg.vkCode as usize] = true;
            }
            WM_KEYUP => {
                let lmap = unsafe { &mut map.lock().unwrap() };
                lmap[stroke_msg.vkCode as usize] = false;
            }
            WM_SYSKEYUP => {
                let lmap = unsafe { &mut map.lock().unwrap() };
                lmap[stroke_msg.vkCode as usize] = false;
            }
            _ => {}
        }
        if keystate == WM_KEYDOWN {
            // key
        } else if keystate == WM_SYSKEYDOWN {
            // system key(ALT+?/F10)
        }
    }
    unsafe { CallNextHookEx(hook, ncode, wparam, lparam) }
}

fn judge_combo_key(lmap: &Vec<bool>) {
    // 0xA2:CTRL
    if lmap[0xA2] == true {
        if lmap[0x43] || lmap[0x58] {
            // 0x43:C
            // 0x58:X
            println!("copy");
        } else if lmap[0x56] {
            // 0x56: V
            println!("paste!");
            open_clipboard();
        }
    }
}

fn open_clipboard() {
    unsafe {
        // クリップボードを開く
        let mut cb = clipboard.lock().unwrap();
        OpenClipboard(HWND::default());
        if cb.len() == 0 {
            let hText = GetClipboardData(CF_UNICODETEXT.0).unwrap();
            if hText.is_invalid() {
                println!("クリップボードにデータないよｗ");
            } else {
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
            }
            GlobalUnlock(hText.0);
        }
        // コピーしたデータを1行ずつ貼り付ける。
        // コピーしたデータが全部なくなるまでこっちの挙動になる。
        // 嫌なら自分で直して。オープンソースだし。
        EmptyClipboard();
        let data = cb.pop_back().unwrap();
        let data = OsString::from(data).encode_wide().collect::<Vec<u16>>();
        let strdata_len = data.len() * 2;
        let data = data.as_ptr();
        let gdata = GlobalAlloc(GHND | GLOBAL_ALLOC_FLAGS(GMEM_SHARE), strdata_len + 2);
        let locked_data = GlobalLock(gdata);
        std::ptr::copy_nonoverlapping(data as *const u8, locked_data as *mut u8, strdata_len + 2);
        match SetClipboardData(CF_UNICODETEXT.0, HANDLE(gdata)) {
            Ok(handle) => {
                println!("set clipboard success.")
            }
            Err(e) => {
                println!("SetClipboardData failed. {:?}", e);
            }
        }
        // 終わったらアンロックする
        GlobalUnlock(gdata);
        // クリップボードも閉じる。
        CloseClipboard();
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
