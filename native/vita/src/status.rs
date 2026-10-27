//! The status bar's readings: local time, the system's clock format,
//! battery and Bluetooth (from hosts/vita/src/status.rs). Wi-Fi comes with
//! the network module.

use shelf_core::host::Status;
use vitasdk_sys::*;

static mut APP_UTIL: Option<bool> = None;

/// AppUtil answers the system's settings (the clock format). Started once.
unsafe fn app_util() -> bool {
    if let Some(ready) = APP_UTIL {
        return ready;
    }
    let mut init: SceAppUtilInitParam = core::mem::zeroed();
    let mut boot: SceAppUtilBootParam = core::mem::zeroed();
    let ready = sceAppUtilInit(&mut init, &mut boot) >= 0;
    APP_UTIL = Some(ready);
    ready
}

unsafe fn clock_24() -> Option<bool> {
    if !app_util() {
        return None;
    }
    let mut value = 0;
    if sceAppUtilSystemParamGetInt(SCE_SYSTEM_PARAM_ID_TIME_FORMAT, &mut value) < 0 {
        return None;
    }
    Some(value == SCE_SYSTEM_PARAM_TIME_FORMAT_24HR as i32)
}

unsafe fn bluetooth() -> Option<bool> {
    let mut value = 0;
    if sceRegMgrGetKeyInt(c"/CONFIG/BT".as_ptr(), c"bt_enable".as_ptr(), &mut value) < 0 {
        return None;
    }
    Some(value != 0)
}

pub fn read() -> Status {
    unsafe {
        let mut now: SceDateTime = core::mem::zeroed();
        let (hour, minute) = if sceRtcGetCurrentClockLocalTime(&mut now) >= 0 { (now.hour as u8, now.minute as u8) } else { (0, 0) };
        let battery = scePowerGetBatteryLifePercent();
        Status {
            hour,
            minute,
            clock24: clock_24(),
            battery: (0..=100).contains(&battery).then_some(battery as u8),
            charging: scePowerIsBatteryCharging() != 0,
            wifi: None,
            bluetooth: bluetooth(),
        }
    }
}
