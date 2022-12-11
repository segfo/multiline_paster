use windows::{
    Win32::Foundation::*,
    Win32::{
        UI::WindowsAndMessaging::*,
    },
};

mod kbdhook;
use kbdhook::*;

fn main() {
    sethook();
    let mut msg = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, HWND::default(), 0, 0).into() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
