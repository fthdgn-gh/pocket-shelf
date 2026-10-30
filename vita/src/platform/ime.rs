//! The system's keyboard (`sceImeDialog`), drawn by the system over the app
//! while it is open: the main loop calls `vita2d_common_dialog_update`
//! between drawing and the swap (`main.rs`). The text arrives when the user
//! presses the dialog's enter button.

use std::sync::Mutex;

use vitasdk_sys::*;

/// The open dialog: its parameters and buffers, which the system reads and
/// writes until it is finished, so they stay at one address.
struct Open {
    param: Box<SceImeDialogParam>,
    _title: Box<[u16]>,
    _initial: Box<[u16]>,
    buffer: Box<[u16]>,
}

// The pointers inside point into the boxes this value owns.
unsafe impl Send for Open {}

static OPEN: Mutex<Option<Open>> = Mutex::new(None);

/// A string as UTF-16 with a terminating zero.
fn utf16(text: &str, room: usize) -> Box<[u16]> {
    let mut units: Vec<u16> = text.encode_utf16().take(room).collect();
    units.resize(room + 1, 0);
    units.into_boxed_slice()
}

/// Open the system keyboard with `title` over `text`, at most `max`
/// characters; `search` labels its enter button Search. False when it
/// could not open (one already open, or the system refused).
pub fn open(title: &str, text: &str, max: usize, search: bool) -> bool {
    let Ok(mut open) = OPEN.lock() else { return false };
    if open.is_some() {
        return false;
    }
    let max = max.clamp(1, SCE_IME_DIALOG_MAX_TEXT_LENGTH as usize);
    let title = utf16(title, 128);
    let mut initial = utf16(text, max);
    let mut buffer = utf16(text, max);
    let mut param: Box<SceImeDialogParam> = Box::new(unsafe { core::mem::zeroed() });
    // `sceImeDialogParamInit`, a header function the bindings do not have:
    // the SDK version, and the magic number plus the common part's address.
    param.sdkVersion = PSP2_SDK_VERSION;
    param.commonParam.magic = SCE_COMMON_DIALOG_MAGIC_NUMBER.wrapping_add(core::ptr::addr_of!(param.commonParam) as u32);
    param.supportedLanguages = 0;
    param.languagesForced = 0;
    param.type_ = SCE_IME_TYPE_DEFAULT;
    param.option = 0;
    param.title = title.as_ptr();
    param.maxTextLength = max as u32;
    param.initialText = initial.as_mut_ptr();
    param.inputTextBuffer = buffer.as_mut_ptr();
    if search {
        param.enterLabel = SCE_IME_ENTER_LABEL_SEARCH as u8;
    }
    let code = unsafe { sceImeDialogInit(&*param) };
    if code < 0 {
        crate::platform::vita_log(format_args!("[ime] open: {:#x}", code as u32));
        return false;
    }
    *open = Some(Open { param, _title: title, _initial: initial, buffer });
    true
}

/// Whether the dialog is open (the main loop draws it).
pub fn active() -> bool {
    OPEN.lock().is_ok_and(|open| open.is_some())
}

/// The dialog's answer once it is finished: the text on enter, `None` when
/// cancelled; `None` from the outer option while it is still open.
pub fn poll() -> Option<Option<String>> {
    let mut open = OPEN.lock().ok()?;
    let Some(dialog) = open.as_ref() else { return Some(None) };
    if unsafe { sceImeDialogGetStatus() } != SCE_COMMON_DIALOG_STATUS_FINISHED {
        return None;
    }
    let mut result: SceImeDialogResult = unsafe { core::mem::zeroed() };
    unsafe { sceImeDialogGetResult(&mut result) };
    let text = (result.button == SCE_IME_DIALOG_BUTTON_ENTER as i32).then(|| {
        let end = dialog.buffer.iter().position(|&u| u == 0).unwrap_or(dialog.buffer.len());
        String::from_utf16_lossy(&dialog.buffer[..end])
    });
    unsafe { sceImeDialogTerm() };
    let _ = &dialog.param;
    *open = None;
    Some(text)
}
