use multiline_parser_pluginlib::plugin::PluginManager;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use windows::{
    w,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::{DataExchange::AddClipboardFormatListener, LibraryLoader::GetModuleHandleW},
        UI::WindowsAndMessaging::{
            CreateWindowExW, RegisterClassExW, HWND_MESSAGE, WINDOW_EX_STYLE, WINDOW_STYLE,
            WM_CLIPBOARDUPDATE, WM_NCCREATE, WM_NCDESTROY, WNDCLASSEXW,
        },
    },
};
// クリップボード変更イベントを収集しDLLに通知する
unsafe extern "system" fn window_message_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            AddClipboardFormatListener(hwnd);
            #[cfg(debug_assertions)]
            println!("起動した");
        }
        WM_NCDESTROY => {
            #[cfg(debug_assertions)]
            println!("後処理");
        }
        WM_CLIPBOARDUPDATE => {
            let pm = unsafe { crate::plugin.lock().unwrap() };
            let laddon_name = unsafe { crate::addon_name.lock().unwrap() };
            if let Ok(update_clipboard) = pm
                .get_plugin_function::<fn()>(&laddon_name, "update_clipboard")
            {
                update_clipboard();
            }
        }
        _ => {}
    }
    LRESULT(1)
}

pub unsafe fn create_message_recv_window() {
    let class_name = w!("MessageRecvWnd");
    let mut wx = WNDCLASSEXW::default();
    wx.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
    wx.lpfnWndProc = Some(window_message_proc);
    wx.hInstance = GetModuleHandleW(None).unwrap();
    wx.lpszClassName = class_name;
    if RegisterClassExW(&wx) != 0 {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("MessageRecvWnd"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            None,
            None,
        );
    }
}

