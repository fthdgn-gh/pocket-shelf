//! The status bar's readings: local time, the clock format the system is set
//! to, battery, Wi-Fi and Bluetooth. The guest asks for them with
//! `ui.__status()` about once a second.

use std::string::String;
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

/// 1 for a 24-hour clock, 0 for a 12-hour clock, -1 when unknown.
unsafe fn clock_24() -> i32 {
    if !app_util() {
        return -1;
    }
    let mut value = 0;
    if sceAppUtilSystemParamGetInt(SCE_SYSTEM_PARAM_ID_TIME_FORMAT, &mut value) < 0 {
        return -1;
    }
    (value == SCE_SYSTEM_PARAM_TIME_FORMAT_24HR as i32) as i32
}

/// Wi-Fi: -1 when the network stack is not up, 0 when not connected,
/// otherwise the signal in percent (at least 1).
unsafe fn wifi() -> i32 {
    if !crate::net::ensure_stack() {
        return -1;
    }
    let mut state = 0;
    if sceNetCtlInetGetState(&mut state) < 0 || state != SCE_NETCTL_STATE_CONNECTED as i32 {
        return 0;
    }
    let mut info: SceNetCtlInfo = core::mem::zeroed();
    if sceNetCtlInetGetInfo(SCE_NETCTL_INFO_GET_RSSI_PERCENTAGE as i32, &mut info) < 0 {
        return 100;
    }
    (info.rssi_percentage as i32).clamp(1, 100)
}

/// Bluetooth switched on in Settings: 1 or 0, -1 when the setting cannot be read.
unsafe fn bluetooth() -> i32 {
    let mut value = 0;
    if sceRegMgrGetKeyInt(c"/CONFIG/BT".as_ptr(), c"bt_enable".as_ptr(), &mut value) < 0 {
        return -1;
    }
    (value != 0) as i32
}

/// `hour minute clock24 battery charging wifi bluetooth`, space separated.
/// The time is local. `battery` is a percent or -1; the others are described
/// on their functions.
pub unsafe fn read() -> String {
    let mut now: SceDateTime = core::mem::zeroed();
    let (hour, minute) = if sceRtcGetCurrentClockLocalTime(&mut now) >= 0 {
        (now.hour as i32, now.minute as i32)
    } else {
        (-1, -1)
    };
    let battery = scePowerGetBatteryLifePercent();
    let battery = if (0..=100).contains(&battery) { battery } else { -1 };
    let charging = (scePowerIsBatteryCharging() != 0) as i32;
    format!("{hour} {minute} {} {battery} {charging} {} {}", clock_24(), wifi(), bluetooth())
}
