//! `globalThis.fs` for the Vita host (cargo feature `data-fs`).
//!
//! Mounts the reference fs core (`engine/crates/pocket-fs`, docs/FS.md) over a
//! folder under `ux0:/data`. Guest paths are relative to that folder
//! and cannot leave it; writes are atomic through a sibling temp directory.

use std::cell::RefCell;
use std::ffi::CString;
use std::fs;
use std::string::String;

use libquickjs_sys::*;
use pocket_fs::{FsModule, Storage};

use crate::ffi::{add_fn, arg_f64, with_str_arg};

/// The app's own folder under `ux0:/data`.
const DATA_DIR: &str = "ux0:/data/PocketShelf";
/// Scratch space for atomic writes. It sits beside the data folder because the
/// fs module requires it outside the tree the guest can see.
const TMP_DIR: &str = "ux0:/data/PocketShelf.tmp";

/// The app's data folder.
pub fn data_dir() -> String {
    String::from(DATA_DIR)
}

thread_local! {
    static MODULE: RefCell<Option<FsModule>> = const { RefCell::new(None) };
}

fn with_module<R>(miss: R, f: impl FnOnce(&mut FsModule) -> R) -> R {
    MODULE.with(|cell| match cell.borrow_mut().as_mut() {
        Some(module) => f(module),
        None => miss,
    })
}

unsafe fn js_string(ctx: *mut JSContext, value: &str) -> JSValue {
    crate::ffi::new_js_string(ctx, value)
}

unsafe extern "C" fn js_read(ctx: *mut JSContext, _t: JSValue, argc: i32, argv: *mut JSValue) -> JSValue {
    let offset = arg_f64(ctx, argc, argv, 1) as i64;
    let max = arg_f64(ctx, argc, argv, 2) as i64;
    let out = with_str_arg(ctx, argc, argv, 0, String::new(), |path| {
        with_module(String::new(), |m| m.read(path, offset, max))
    });
    js_string(ctx, &out)
}

unsafe extern "C" fn js_write(ctx: *mut JSContext, _t: JSValue, argc: i32, argv: *mut JSValue) -> JSValue {
    let mode = arg_f64(ctx, argc, argv, 2) as u32;
    let status = with_str_arg(ctx, argc, argv, 0, -1, |path| {
        with_str_arg(ctx, argc, argv, 1, -1, |data| {
            with_module(-1, |m| m.write(path, data, mode))
        })
    });
    JS_NewInt32(ctx, status)
}

unsafe extern "C" fn js_remove(ctx: *mut JSContext, _t: JSValue, argc: i32, argv: *mut JSValue) -> JSValue {
    let recursive = arg_f64(ctx, argc, argv, 1) as u32;
    let status = with_str_arg(ctx, argc, argv, 0, -1, |path| {
        with_module(-1, |m| m.remove(path, recursive))
    });
    JS_NewInt32(ctx, status)
}

unsafe extern "C" fn js_list(ctx: *mut JSContext, _t: JSValue, argc: i32, argv: *mut JSValue) -> JSValue {
    let offset = arg_f64(ctx, argc, argv, 1) as i64;
    let out = with_str_arg(ctx, argc, argv, 0, String::new(), |path| {
        with_module(String::new(), |m| m.list(path, offset))
    });
    js_string(ctx, &out)
}

unsafe extern "C" fn js_stat(ctx: *mut JSContext, _t: JSValue, argc: i32, argv: *mut JSValue) -> JSValue {
    let out = with_str_arg(ctx, argc, argv, 0, String::new(), |path| {
        with_module(String::new(), |m| m.stat(path))
    });
    js_string(ctx, &out)
}

unsafe extern "C" fn js_mkdir(ctx: *mut JSContext, _t: JSValue, argc: i32, argv: *mut JSValue) -> JSValue {
    let status = with_str_arg(ctx, argc, argv, 0, -1, |path| {
        with_module(-1, |m| m.mkdir(path))
    });
    JS_NewInt32(ctx, status)
}

unsafe extern "C" fn js_rename(ctx: *mut JSContext, _t: JSValue, argc: i32, argv: *mut JSValue) -> JSValue {
    let status = with_str_arg(ctx, argc, argv, 0, -1, |from| {
        with_str_arg(ctx, argc, argv, 1, -1, |to| {
            with_module(-1, |m| m.rename(from, to))
        })
    });
    JS_NewInt32(ctx, status)
}

unsafe extern "C" fn js_usage(ctx: *mut JSContext, _t: JSValue, _argc: i32, _argv: *mut JSValue) -> JSValue {
    let out = with_module(String::new(), |m| m.usage());
    js_string(ctx, &out)
}

unsafe extern "C" fn js_last_error(ctx: *mut JSContext, _t: JSValue, _argc: i32, _argv: *mut JSValue) -> JSValue {
    let out = with_module(String::from("fs: not mounted"), |m| m.last_error());
    js_string(ctx, &out)
}

/// Create the title's data folder and install `globalThis.fs`. When the folder
/// cannot be created the namespace stays unmounted and the SDK throws, which
/// the app treats as "no saved settings".
///
/// # Safety
///
/// `ctx` must be the live guest context and `global` its global object.
pub unsafe fn mount(ctx: *mut JSContext, global: JSValue) {
    let root = String::from(DATA_DIR);
    let tmp = String::from(TMP_DIR);
    if fs::create_dir_all(&root).is_err() || fs::create_dir_all(&tmp).is_err() {
        return;
    }
    MODULE.with(|cell| {
        *cell.borrow_mut() = Some(FsModule::new(Storage::Dir {
            root: root.into(),
            tmp: tmp.into(),
        }));
    });
    let ns = JS_NewObject(ctx);
    add_fn(ctx, ns, b"read\0", js_read, 3);
    add_fn(ctx, ns, b"write\0", js_write, 3);
    add_fn(ctx, ns, b"remove\0", js_remove, 2);
    add_fn(ctx, ns, b"list\0", js_list, 2);
    add_fn(ctx, ns, b"stat\0", js_stat, 1);
    add_fn(ctx, ns, b"mkdir\0", js_mkdir, 1);
    add_fn(ctx, ns, b"rename\0", js_rename, 2);
    add_fn(ctx, ns, b"usage\0", js_usage, 0);
    add_fn(ctx, ns, b"lastError\0", js_last_error, 0);
    let name = CString::new("fs").unwrap();
    JS_SetPropertyStr(ctx, global, name.as_ptr(), ns);
}
