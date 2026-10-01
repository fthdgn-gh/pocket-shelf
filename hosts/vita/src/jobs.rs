//! Background decoding of pictures (cargo feature `installed-apps`).
//!
//! Reading and decoding a PNG takes long enough to drop frames, and a retail
//! game's files also need a decrypting mount first. So icons, custom art and
//! backdrops are decoded on worker threads. A module asks for a picture with
//! `poll`: the first call starts the work and answers `Pending`, and the
//! guest's request is answered `PENDING`. The guest asks again on a later
//! frame; the call that finds the pixels ready gets them and uploads the
//! texture, so GPU work stays on the main thread.

use std::string::String;
use std::sync::Mutex;
use std::vec::Vec;

/// A decoded picture: width, height and tightly packed RGBA.
pub type Pixels = (u32, u32, Vec<u8>);

/// What the guest is told while a picture is still being decoded.
pub const PENDING: i32 = -2;

/// Pictures decoding at once. A request beyond this is not started; the
/// guest's next request for it finds a free worker.
const WORKERS_MAX: usize = 2;
/// Decoded pictures nobody has collected, in bytes of RGBA. The guest stops
/// asking for a picture when the selection moves away from it, so the oldest
/// uncollected ones are dropped beyond this.
const READY_BYTES_MAX: usize = 4 * 1024 * 1024;
const WORKER_STACK: usize = 128 * 1024;

pub enum Poll {
    Pending,
    /// The decoded pixels, or None when there is no picture that decodes.
    Ready(Option<Pixels>),
}

enum Job {
    Decoding,
    Ready(Option<Pixels>),
}

/// Work in progress and finished work, by key, oldest first.
static JOBS: Mutex<Vec<(String, Job)>> = Mutex::new(Vec::new());

fn ready_bytes(jobs: &[(String, Job)]) -> usize {
    jobs.iter()
        .map(|(_, job)| match job {
            Job::Ready(Some((_, _, rgba))) => rgba.len(),
            _ => 0,
        })
        .sum()
}

/// Ask for the picture `key`. `work` reads and decodes it; it runs on a worker
/// thread, at most once per key at a time. A `Ready` answer hands the result
/// over and forgets the key, so the caller keeps what it needs from it.
pub fn poll(key: &str, work: impl FnOnce() -> Option<Pixels> + Send + 'static) -> Poll {
    let Ok(mut jobs) = JOBS.lock() else {
        return Poll::Ready(None);
    };
    if let Some(index) = jobs.iter().position(|(id, _)| id == key) {
        if matches!(jobs[index].1, Job::Decoding) {
            return Poll::Pending;
        }
        return match jobs.remove(index).1 {
            Job::Ready(pixels) => Poll::Ready(pixels),
            Job::Decoding => Poll::Pending,
        };
    }
    if jobs.iter().filter(|(_, job)| matches!(job, Job::Decoding)).count() >= WORKERS_MAX {
        return Poll::Pending;
    }
    while ready_bytes(&jobs) > READY_BYTES_MAX {
        let Some(index) = jobs.iter().position(|(_, job)| matches!(job, Job::Ready(_))) else {
            break;
        };
        jobs.remove(index);
    }
    jobs.push((String::from(key), Job::Decoding));
    drop(jobs);
    let owned = String::from(key);
    let spawned = std::thread::Builder::new()
        .name(String::from("pocket-decode"))
        .stack_size(WORKER_STACK)
        .spawn(move || {
            let pixels = work();
            if let Ok(mut jobs) = JOBS.lock() {
                if let Some(entry) = jobs.iter_mut().find(|(id, _)| *id == owned) {
                    entry.1 = Job::Ready(pixels);
                }
            }
        });
    if spawned.is_err() {
        if let Ok(mut jobs) = JOBS.lock() {
            jobs.retain(|(id, _)| id != key);
        }
        return Poll::Ready(None);
    }
    Poll::Pending
}
