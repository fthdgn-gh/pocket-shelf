//! Memory for the log: the heap the app holds, counted by the allocator,
//! and the system's free memory by kind.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The system allocator with a count of the bytes held and the most held.
pub struct Counting;

static HELD: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn add(bytes: usize) {
    let held = HELD.fetch_add(bytes, Ordering::Relaxed) + bytes;
    PEAK.fetch_max(held, Ordering::Relaxed);
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            add(layout.size());
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc_zeroed(layout);
        if !ptr.is_null() {
            add(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        HELD.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new = System.realloc(ptr, layout, new_size);
        if !new.is_null() {
            HELD.fetch_sub(layout.size(), Ordering::Relaxed);
            add(new_size);
        }
        new
    }
}

/// "heap 12345 KB (most 23456 KB), free 1234 KB, CDRAM 567 KB, phycont 89 KB".
pub fn line() -> String {
    let kb = |bytes: usize| bytes / 1024;
    let mut info = vitasdk_sys::SceKernelFreeMemorySizeInfo { size: core::mem::size_of::<vitasdk_sys::SceKernelFreeMemorySizeInfo>() as i32, size_user: 0, size_cdram: 0, size_phycont: 0 };
    let free = if unsafe { vitasdk_sys::sceKernelGetFreeMemorySize(&mut info) } >= 0 {
        format!(", free {} KB, CDRAM {} KB, phycont {} KB", info.size_user / 1024, info.size_cdram / 1024, info.size_phycont / 1024)
    } else {
        String::new()
    };
    format!("heap {} KB (most {} KB){free}", kb(HELD.load(Ordering::Relaxed)), kb(PEAK.load(Ordering::Relaxed)))
}
