use once_cell::unsync::*;
use std::sync::RwLock;
use windows::Win32::{
    Foundation::*,
    UI::{ WindowsAndMessaging::*},
};

use multiline_parser_pluginlib::result::*;

static mut hook: HHOOK = HHOOK(0);
static mut stroke_callback: Lazy<RwLock<StrokeMessage>> =
    Lazy::new(|| RwLock::new(StrokeMessage::default()));

pub struct StrokeMessage {
    key_down: Box<dyn Fn(u32, KBDLLHOOKSTRUCT) -> PluginResult>,
    key_up: Box<dyn Fn(u32, KBDLLHOOKSTRUCT) -> PluginResult>,
}
pub fn set_stroke_callback(stroke_msg: StrokeMessage) {
    let mut stroke_msg_cb = unsafe { stroke_callback.write().unwrap() };
    *stroke_msg_cb = stroke_msg;
}
impl StrokeMessage {
    pub fn set_key_down(
        &mut self,
        callback: Box<dyn Fn(u32, KBDLLHOOKSTRUCT) -> PluginResult>,
    ) -> &Self {
        self.key_down = callback;
        self
    }
    pub fn set_key_up(
        &mut self,
        callback: Box<dyn Fn(u32, KBDLLHOOKSTRUCT) -> PluginResult>,
    ) -> &Self {
        self.key_up = callback;
        self
    }
}

impl Default for StrokeMessage {
    fn default() -> Self {
        let default_key_down_up = Box::new(|state, ks| -> PluginResult { PluginResult::Success });
        Self {
            key_down: default_key_down_up.clone(),
            key_up: default_key_down_up,
        }
    }
}

#[no_mangle]
pub extern "system" fn hook_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if HC_ACTION as i32 == ncode {
        let keystate = wparam.0 as u32;
        let stroke_msg = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let cb = unsafe { stroke_callback.read().unwrap() };
        match keystate {
            WM_KEYDOWN => match (cb.key_down)(keystate, stroke_msg) {
                PluginResult::NoChain => {
                    return LRESULT(0);
                }
                PluginResult::NoChainAndCancel => {
                    return LRESULT(1);
                }
                _ => {}
            },
            WM_SYSKEYDOWN => match (cb.key_down)(keystate, stroke_msg) {
                PluginResult::NoChain => {
                    return LRESULT(0);
                }
                PluginResult::NoChainAndCancel => {
                    return LRESULT(1);
                }
                _ => {}
            },
            WM_KEYUP => match (cb.key_up)(keystate, stroke_msg) {
                PluginResult::NoChain => {
                    return LRESULT(0);
                }
                PluginResult::NoChainAndCancel => {
                    return LRESULT(1);
                }
                _ => {}
            },
            WM_SYSKEYUP => match (cb.key_up)(keystate, stroke_msg) {
                PluginResult::NoChain => {
                    return LRESULT(0);
                }
                PluginResult::NoChainAndCancel => {
                    return LRESULT(1);
                }
                _ => {}
            },
            _ => {}
        }
    }
    unsafe { CallNextHookEx(hook, ncode, wparam, lparam) }
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
