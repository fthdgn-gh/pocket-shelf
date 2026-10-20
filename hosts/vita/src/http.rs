//! HTTPS GET for apps (cargo feature `http`).
//!
//! An app starts a request and polls it once per frame; the request itself
//! runs on its own thread, so the 60 fps main thread never waits on the
//! network. A response either stays in memory as text (an API reply) or goes
//! straight to a file in the app's data folder (a PNG image, or the app's own
//! update package), so a large file never passes through the guest.
//!
//! TLS is rustls with the RustCrypto provider and Mozilla's root
//! certificates, over std::net sockets. The system's SceSsl is not used: it
//! fails the handshake with servers that present an ECDSA certificate, which
//! is what Cloudflare-fronted sites serve.
//!
//! Host extras on `ui` (not spec ops):
//!
//!   __netGet(url, authorization) -> id | -1    text response, at most TEXT_MAX
//!   __netSave(url, path) -> id | -1            PNG or update VPK to <data folder>/<path>;
//!                                              for `libretro/<name>.txt`, the file
//!                                              names a thumbnail listing page links to
//!   __netSaveIcon(url, path) -> id | -1        a PNG fitted whole into a 128 px
//!                                              square, to `art/<name>.png`
//!   __netState(id) -> "busy <received> <total>"
//!                   | "done <http status> <received>"
//!                   | "error <reason>"
//!   __netText(id) -> the text response of a finished request
//!   __netClose(id)                             cancel and forget a request

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::string::String;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use std::vec::Vec;

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

/// Largest text response kept in memory.
const TEXT_MAX: usize = 512 * 1024;
/// Largest picture written. Matches the largest picture the decoders read.
const FILE_MAX: usize = 8 * 1024 * 1024;
/// Largest update package written. A release VPK is about 2.5 MB.
const VPK_MAX: usize = 64 * 1024 * 1024;
/// Largest thumbnail listing page read. The NES box art page is 4 MB.
const INDEX_MAX: usize = 16 * 1024 * 1024;
/// The one path an update package is saved to (src/updates.ts `UPDATE_FILE`).
pub const UPDATE_VPK: &str = "update/pocket-shelf.vpk";
/// Requests running or waiting to be read at once.
const SLOTS_MAX: usize = 4;
const READ_CHUNK: usize = 16 * 1024;
/// Largest response header block read.
const HEAD_MAX: usize = 16 * 1024;
const REDIRECTS_MAX: usize = 3;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const IO_TIMEOUT: Duration = Duration::from_secs(30);
/// The handshake verifies a certificate chain in software; the default
/// thread stack is too small for it.
const WORKER_STACK: usize = 512 * 1024;
const USER_AGENT: &str = "PocketShelf/1.0 (PS Vita)";
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
/// A VPK is a ZIP file: its first local file header.
const ZIP_SIGNATURE: [u8; 4] = [b'P', b'K', 3, 4];

/// What a request does with its response.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Kept in memory as text.
    Text,
    /// A PNG written as it is.
    File,
    /// A PNG fitted whole into a square icon and written as a new PNG: a
    /// RetroArch box art is 512 px wide and up to 700 KB; the icon is 128 px.
    Icon,
    /// The update package.
    Package,
    /// A thumbnail folder's listing page (libretro's thumbnail server): read
    /// in memory, and the file names it links to written one per line.
    Index,
}

enum Outcome {
    Busy,
    Done { status: i32 },
    Failed(String),
}

struct Slot {
    id: i32,
    outcome: Outcome,
    received: usize,
    total: usize,
    body: Vec<u8>,
    cancel: Arc<AtomicBool>,
}

static SLOTS: Mutex<Vec<Slot>> = Mutex::new(Vec::new());
static NEXT_ID: AtomicI32 = AtomicI32::new(1);
static TLS: OnceLock<Option<Arc<ClientConfig>>> = OnceLock::new();

/// The TLS client settings, built on first use: Mozilla's roots, TLS 1.2 and 1.3.
fn tls_config() -> Option<Arc<ClientConfig>> {
    TLS.get_or_init(|| {
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = ClientConfig::builder_with_provider(Arc::new(rustls_rustcrypto::provider()))
            .with_safe_default_protocol_versions()
            .ok()?
            .with_root_certificates(roots)
            .with_no_client_auth();
        Some(Arc::new(config))
    })
    .clone()
}

/// A file a download may be written to: `art/<name>.png` or
/// `backdrops/<name>.png` with a plain file name, `libretro/<name>.txt` for a
/// listing's names, or `UPDATE_VPK`.
fn valid_dest(path: &str) -> bool {
    if path == UPDATE_VPK {
        return true;
    }
    let Some((dir, name)) = path.split_once('/') else {
        return false;
    };
    let extension = if dir == "libretro" { ".txt" } else { ".png" };
    (dir == "art" || dir == "backdrops" || dir == "libretro")
        && name.len() >= 5
        && name.len() <= 64
        && !name.starts_with('.')
        && name.ends_with(extension)
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

fn with_slot<R>(id: i32, f: impl FnOnce(&mut Slot) -> R) -> Option<R> {
    let mut slots = SLOTS.lock().ok()?;
    slots.iter_mut().find(|slot| slot.id == id).map(f)
}

fn finish(id: i32, outcome: Outcome) {
    with_slot(id, |slot| slot.outcome = outcome);
}

/// Start a GET. `auth` is the Authorization header's value ("" for none);
/// `dest` is a path under the data folder for a file download, or None to keep
/// the response as text; with `icon`, a picture is fitted into a square icon
/// first. Returns the request id, or -1 when the request is refused: not an
/// https URL, a bad path, no free slot, or no network stack.
pub fn start(url: &str, auth: &str, dest: Option<&str>, icon: bool) -> i32 {
    let plain = |text: &str| !text.bytes().any(|byte| byte < 0x20 || byte == 0x7f);
    if url.len() > 1024 || auth.len() > 256 || !plain(url) || !plain(auth) || parse_url(url).is_none() {
        return -1;
    }
    if dest.is_some_and(|path| !valid_dest(path)) {
        return -1;
    }
    if !crate::net::ensure_stack() {
        return -1;
    }
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let Ok(mut slots) = SLOTS.lock() else {
            return -1;
        };
        if slots.len() >= SLOTS_MAX {
            return -1;
        }
        slots.push(Slot {
            id,
            outcome: Outcome::Busy,
            received: 0,
            total: 0,
            body: Vec::new(),
            cancel: cancel.clone(),
        });
    }
    let url = String::from(url);
    let auth = String::from(auth);
    let kind = match dest {
        None => Kind::Text,
        Some(UPDATE_VPK) => Kind::Package,
        Some(path) if path.starts_with("libretro/") => Kind::Index,
        Some(path) if icon && path.starts_with("art/") => Kind::Icon,
        Some(_) => Kind::File,
    };
    if icon && kind != Kind::Icon {
        close(id);
        return -1;
    }
    let dest = dest.map(|path| format!("{}/{path}", crate::datafs::data_dir()));
    let spawned = std::thread::Builder::new()
        .name(String::from("pocket-http"))
        .stack_size(WORKER_STACK)
        .spawn(move || {
            let outcome = match run(id, &url, &auth, dest.as_deref(), kind, &cancel) {
                Ok(status) => Outcome::Done { status },
                Err(reason) => {
                    if let Some(path) = dest.as_deref() {
                        let _ = fs::remove_file(format!("{path}.part"));
                    }
                    Outcome::Failed(reason)
                }
            };
            finish(id, outcome);
        });
    if spawned.is_err() {
        close(id);
        return -1;
    }
    id
}

/// Host, port and path of an `https://` URL.
fn parse_url(url: &str) -> Option<(String, u16, String)> {
    let rest = url.strip_prefix("https://")?;
    let (authority, path) = match rest.find(['/', '?']) {
        Some(index) if rest.as_bytes()[index] == b'/' => (&rest[..index], String::from(&rest[index..])),
        Some(index) => (&rest[..index], format!("/{}", &rest[index..])),
        None => (rest, String::from("/")),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, port.parse().ok()?),
        None => (authority, 443),
    };
    let valid = !host.is_empty()
        && host.len() <= 253
        && host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'));
    valid.then(|| (String::from(host), port, path))
}

struct Response {
    status: i32,
    /// `Content-Length`, when the server sent one.
    length: Option<usize>,
    chunked: bool,
    location: Option<String>,
}

/// Connect, send the GET and read the response's header block.
fn open(url: &str, auth: &str) -> Result<(BufReader<StreamOwned<ClientConnection, TcpStream>>, Response), String> {
    let (host, port, path) = parse_url(url).ok_or_else(|| String::from("url"))?;
    let address = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|_| String::from("dns"))?
        .next()
        .ok_or_else(|| String::from("dns"))?;
    let socket = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT).map_err(|_| String::from("connect"))?;
    let _ = socket.set_read_timeout(Some(IO_TIMEOUT));
    let _ = socket.set_write_timeout(Some(IO_TIMEOUT));
    let config = tls_config().ok_or_else(|| String::from("tls setup"))?;
    let name = ServerName::try_from(host.clone()).map_err(|_| String::from("host name"))?;
    let session = ClientConnection::new(config, name).map_err(|error| format!("tls {error}"))?;
    let mut stream = StreamOwned::new(session, socket);
    let mut request = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: {USER_AGENT}\r\nAccept: */*\r\nConnection: close\r\n"
    );
    if !auth.is_empty() {
        request.push_str(&format!("Authorization: {auth}\r\n"));
    }
    request.push_str("\r\n");
    // The first write drives the handshake, so a certificate the roots do not
    // vouch for is reported here.
    stream
        .write_all(request.as_bytes())
        .and_then(|()| stream.flush())
        .map_err(|error| format!("tls {error}"))?;

    let mut reader = BufReader::with_capacity(READ_CHUNK, stream);
    let mut line = String::new();
    let mut head = 0usize;
    let mut next_line = |reader: &mut BufReader<_>, line: &mut String| -> Result<(), String> {
        line.clear();
        let read = reader.read_line(line).map_err(|error| format!("read {error}"))?;
        head += read;
        if read == 0 || head > HEAD_MAX {
            return Err(String::from("response header"));
        }
        Ok(())
    };
    next_line(&mut reader, &mut line)?;
    let status = line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<i32>().ok())
        .ok_or_else(|| String::from("status line"))?;
    let mut response = Response { status, length: None, chunked: false, location: None };
    loop {
        next_line(&mut reader, &mut line)?;
        let field = line.trim_end();
        if field.is_empty() {
            break;
        }
        let Some((name, value)) = field.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if name.eq_ignore_ascii_case("content-length") {
            response.length = value.parse().ok();
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            response.chunked = value.to_ascii_lowercase().contains("chunked");
        } else if name.eq_ignore_ascii_case("location") {
            response.location = Some(String::from(value));
        }
    }
    Ok((reader, response))
}

/// The request itself, on the worker thread. Returns the HTTP status.
fn run(id: i32, url: &str, auth: &str, dest: Option<&str>, kind: Kind, cancel: &AtomicBool) -> Result<i32, String> {
    let mut url = String::from(url);
    let mut auth = String::from(auth);
    let mut redirects = 0;
    let (mut reader, response) = loop {
        let (reader, response) = open(&url, &auth)?;
        if !matches!(response.status, 301 | 302 | 303 | 307 | 308) {
            break (reader, response);
        }
        let Some(location) = response.location else {
            break (reader, response);
        };
        redirects += 1;
        if redirects > REDIRECTS_MAX {
            return Err(String::from("too many redirects"));
        }
        let (host, port, _) = parse_url(&url).ok_or_else(|| String::from("url"))?;
        let next = if location.starts_with('/') {
            format!("https://{host}:{port}{location}")
        } else {
            location
        };
        // The key is for the host it was given to; another host does not get it.
        if parse_url(&next).map(|(next_host, _, _)| next_host) != Some(host) {
            auth.clear();
        }
        url = next;
    };
    let status = response.status;
    if let Some(length) = response.length {
        with_slot(id, |slot| slot.total = length);
    }
    // Only a 200 is written to disk; any other reply is kept as text so the
    // app can report it.
    let saving = dest.filter(|_| status == 200);
    let limit = match (saving, kind) {
        (None, _) | (_, Kind::Text) => TEXT_MAX,
        (Some(_), Kind::File | Kind::Icon) => FILE_MAX,
        (Some(_), Kind::Package) => VPK_MAX,
        (Some(_), Kind::Index) => INDEX_MAX,
    };
    // A listing page is read in memory; everything else saved goes to a file.
    let part = saving.filter(|_| kind != Kind::Index).map(|path| format!("{path}.part"));
    let mut file = match (&part, saving) {
        (Some(part), Some(path)) if kind != Kind::Index => {
            if let Some((dir, _)) = path.rsplit_once('/') {
                let _ = fs::create_dir_all(dir);
            }
            Some(fs::File::create(part).map_err(|_| String::from("file create"))?)
        }
        _ => None,
    };
    let mut received = 0usize;
    let mut head = [0u8; 8];
    let mut sink = |bytes: &[u8]| -> Result<(), String> {
        if cancel.load(Ordering::Relaxed) {
            return Err(String::from("cancelled"));
        }
        for (index, byte) in bytes.iter().enumerate() {
            if received + index < head.len() {
                head[received + index] = *byte;
            }
        }
        received += bytes.len();
        if received > limit {
            return Err(String::from("too large"));
        }
        match file.as_mut() {
            Some(file) => file.write_all(bytes).map_err(|_| String::from("file write"))?,
            None => {
                with_slot(id, |slot| slot.body.extend_from_slice(bytes));
            }
        }
        with_slot(id, |slot| slot.received = received);
        Ok(())
    };
    let mut chunk = vec![0u8; READ_CHUNK];
    if response.chunked {
        let mut line = String::new();
        loop {
            line.clear();
            reader.read_line(&mut line).map_err(|error| format!("read {error}"))?;
            let size = line.trim().split(';').next().unwrap_or("");
            let mut left = usize::from_str_radix(size, 16).map_err(|_| String::from("chunk size"))?;
            if left == 0 {
                break;
            }
            while left > 0 {
                let want = left.min(READ_CHUNK);
                reader.read_exact(&mut chunk[..want]).map_err(|error| format!("read {error}"))?;
                sink(&chunk[..want])?;
                left -= want;
            }
            // The line break that ends the chunk.
            line.clear();
            reader.read_line(&mut line).map_err(|error| format!("read {error}"))?;
        }
    } else {
        let mut left = response.length;
        while left != Some(0) {
            let want = left.map_or(READ_CHUNK, |left| left.min(READ_CHUNK));
            match reader.read(&mut chunk[..want]) {
                Ok(0) => break,
                Ok(read) => {
                    sink(&chunk[..read])?;
                    left = left.map(|left| left - read);
                }
                // A server that closes the socket without a TLS goodbye ends a
                // response that has no length.
                Err(error) if left.is_none() && error.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(format!("read {error}")),
            }
        }
        if left.is_some_and(|left| left > 0) {
            return Err(String::from("response cut short"));
        }
    }
    if let (Kind::Index, Some(path)) = (kind, saving) {
        let body = with_slot(id, |slot| core::mem::take(&mut slot.body)).unwrap_or_default();
        let names = listing_names(&String::from_utf8_lossy(&body));
        if names.is_empty() {
            return Err(String::from("no names in the listing"));
        }
        write_atomic(path, (names.join("\n") + "\n").as_bytes())?;
    }
    if let (Some(file), Some(part), Some(path)) = (file, part, saving) {
        drop(file);
        if kind == Kind::Package {
            if head[..4] != ZIP_SIGNATURE {
                return Err(String::from("not a vpk"));
            }
        } else if received < head.len() || head != PNG_SIGNATURE {
            return Err(String::from("not a png"));
        }
        if kind == Kind::Icon {
            let icon = shrink_icon(&part);
            let _ = fs::remove_file(&part);
            write_atomic(path, &icon?)?;
        } else {
            let _ = fs::remove_file(path);
            fs::rename(&part, path).map_err(|_| String::from("file rename"))?;
        }
    }
    Ok(status)
}

#[cfg(feature = "installed-apps")]
fn listing_names(html: &str) -> Vec<String> {
    crate::retroarch::index_names(html)
}

/// Without RetroArch support there are no listings to read.
#[cfg(not(feature = "installed-apps"))]
fn listing_names(_html: &str) -> Vec<String> {
    Vec::new()
}

/// Write `bytes` to `path` through a `.part` file, so a reader never sees half of it.
fn write_atomic(path: &str, bytes: &[u8]) -> Result<(), String> {
    if let Some((dir, _)) = path.rsplit_once('/') {
        let _ = fs::create_dir_all(dir);
    }
    let part = format!("{path}.part");
    fs::write(&part, bytes).map_err(|_| String::from("file write"))?;
    let _ = fs::remove_file(path);
    fs::rename(&part, path).map_err(|_| String::from("file rename"))
}

/// The PNG at `path` fitted whole into a 128 px square with clear bars
/// (`pspemu::fit_png`, as a PSP icon), encoded as an RGBA PNG.
#[cfg(feature = "installed-apps")]
fn shrink_icon(path: &str) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path).map_err(|_| String::from("file read"))?;
    let (width, height, rgba) = crate::pspemu::fit_png(&bytes).ok_or_else(|| String::from("not a png it can read"))?;
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|_| String::from("png encode"))?;
        writer.write_image_data(&rgba).map_err(|_| String::from("png encode"))?;
    }
    Ok(out)
}

/// Without the PNG decoder, the picture is kept as it is.
#[cfg(not(feature = "installed-apps"))]
fn shrink_icon(path: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|_| String::from("file read"))
}

/// One line describing a request; see the module comment.
pub fn state(id: i32) -> String {
    with_slot(id, |slot| match &slot.outcome {
        Outcome::Busy => {
            // The guest polls a running request once per frame. Fetching
            // artwork for many titles takes minutes with no button pressed;
            // the tick resets the idle timer so the console does not suspend
            // and drop the connection.
            extern "C" {
                fn sceKernelPowerTick(kind: u32) -> i32;
            }
            unsafe { sceKernelPowerTick(0) };
            format!("busy {} {}", slot.received, slot.total)
        }
        Outcome::Done { status } => format!("done {} {}", status, slot.received),
        Outcome::Failed(reason) => format!("error {reason}"),
    })
    .unwrap_or_else(|| String::from("error unknown request"))
}

/// The response text of a finished request, with invalid UTF-8 replaced.
pub fn text(id: i32) -> String {
    with_slot(id, |slot| String::from_utf8_lossy(&slot.body).into_owned()).unwrap_or_default()
}

/// Cancel a request that is still running and forget it.
pub fn close(id: i32) {
    if let Ok(mut slots) = SLOTS.lock() {
        if let Some(index) = slots.iter().position(|slot| slot.id == id) {
            slots[index].cancel.store(true, Ordering::Relaxed);
            slots.remove(index);
        }
    }
}
