//! The Vita's network stack, brought up once for `http.rs` (the part of
//! the current host's `net.rs` that is not its developer link).

use std::sync::atomic::{AtomicU8, Ordering};

/// sceNet's memory pool; sceNet keeps referencing it for the process
/// lifetime.
const NET_POOL_BYTES: u32 = 1024 * 1024;

/// 0 not tried, 1 up, 2 failed.
static INIT_STATE: AtomicU8 = AtomicU8::new(0);

/// Bring up sceNet and sceNetCtl; true once they are up.
pub(crate) fn ensure_stack() -> bool {
    match INIT_STATE.load(Ordering::Acquire) {
        1 => return true,
        2 => return false,
        _ => {}
    }
    let ok = unsafe { init_scenet() };
    INIT_STATE.store(if ok { 1 } else { 2 }, Ordering::Release);
    if !ok {
        crate::platform::vita_log(format_args!("net: sceNet init failed"));
    }
    ok
}

unsafe fn init_scenet() -> bool {
    use vitasdk_sys::*;
    static mut NET_POOL: [u8; NET_POOL_BYTES as usize] = [0; NET_POOL_BYTES as usize];
    let loaded = sceSysmoduleLoadModule(SCE_SYSMODULE_NET as _);
    // Already loaded is fine too (the error code differs per firmware).
    if loaded < 0 && sceSysmoduleIsLoaded(SCE_SYSMODULE_NET as _) != 0 {
        return false;
    }
    let mut init = SceNetInitParam {
        memory: core::ptr::addr_of_mut!(NET_POOL) as *mut core::ffi::c_void,
        size: NET_POOL_BYTES as i32,
        flags: 0,
    };
    let rc = sceNetInit(&mut init);
    // Already initialized is success here.
    if rc < 0 && rc != SCE_NET_ERROR_EBUSY as i32 {
        return false;
    }
    // SCE_NET_CTL_ERROR_NOT_TERMINATED: already initialized.
    const NETCTL_NOT_TERMINATED: u32 = 0x8041_2102;
    let rc = sceNetCtlInit();
    if rc < 0 && rc as u32 != NETCTL_NOT_TERMINATED {
        return false;
    }
    true
}
