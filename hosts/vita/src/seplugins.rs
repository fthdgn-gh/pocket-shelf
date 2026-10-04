//! Adrenaline's plugin lists in `ms0:/seplugins/`, read and changed the way
//! each Adrenaline reads them. Pure text functions; `pspemu.rs` does the file
//! access. The tests run natively:
//! `rustc --edition 2021 --test hosts/vita/src/seplugins.rs -o /tmp/t && /tmp/t`.
//!
//! - TheOfficialFloW's Adrenaline reads the first 1024 bytes of `vsh.txt`.
//!   A line is a module path, a space or tab, and `1` for on; any other
//!   ending is off. Every line that is on is loaded.
//! - isage's Adrenaline reads `EPIplugins.txt` when it exists, else
//!   `plugins.txt`. A line is `<runlevel>, <path>, <state>`; a runlevel that
//!   holds `vsh` or `xmb`, or is `all` or `always`, loads at the XMB; a state of
//!   `on`, `1`, `true` or `enabled` is on. A path without a device is under
//!   `ms0:/seplugins/`. Lines starting with `#`, `;` or `//` are comments.
//!   Lines are applied in order, so the last line for a module decides.

use std::string::String;

/// Bytes of `vsh.txt` that TheOfficialFloW's Adrenaline reads.
pub const FLOW_READ_MAX: usize = 1024;

/// Whether a list names the module, and whether it loads at the XMB.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListState {
    Absent,
    Off,
    On,
}

/// The lines of a list without their line ends.
fn lines(text: &str) -> impl Iterator<Item = &str> {
    text.split('\n').map(|line| line.strip_suffix('\r').unwrap_or(line))
}

/// The line end the file uses, `\n` for a new file.
fn newline(text: &str) -> &'static str {
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// `text` with the line at `index` replaced by `line`, other bytes kept.
fn replace_line(text: &str, index: usize, line: &str) -> String {
    let mut out = String::with_capacity(text.len() + line.len());
    for (at, part) in text.split('\n').enumerate() {
        if at > 0 {
            out.push('\n');
        }
        if at == index {
            out.push_str(line);
            if part.ends_with('\r') {
                out.push('\r');
            }
        } else {
            out.push_str(part);
        }
    }
    out
}

/// `text` with `line` added at the end.
fn append_line(text: &str, line: &str) -> String {
    let end = newline(text);
    let mut out = String::from(text);
    if !out.is_empty() && !out.ends_with('\n') {
        out.push_str(end);
    }
    out.push_str(line);
    out.push_str(end);
    out
}

/// The module path and whether it is on, for one `vsh.txt` line.
fn flow_line(line: &str) -> Option<(&str, bool)> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    match line.find([' ', '\t']) {
        Some(at) => Some((&line[..at], &line[at + 1..] == "1")),
        None => Some((line, false)),
    }
}

/// How `vsh.txt` lists `module` (a full `ms0:/...` path), within the bytes
/// Adrenaline reads.
pub fn flow_state(text: &str, module: &str) -> ListState {
    let read = &text[..floor_char_boundary(text, FLOW_READ_MAX)];
    let mut state = ListState::Absent;
    for line in lines(read) {
        if let Some((path, on)) = flow_line(line) {
            if path.eq_ignore_ascii_case(module) {
                if on {
                    return ListState::On;
                }
                state = ListState::Off;
            }
        }
    }
    state
}

/// `vsh.txt` with `module` on: its first line set to `<module> 1`, or a new
/// line, put first when appending would leave it past what Adrenaline reads.
pub fn flow_enable(text: &str, module: &str) -> String {
    let line = format!("{module} 1");
    let index = lines(text).position(|line| flow_line(line).is_some_and(|(path, _)| path.eq_ignore_ascii_case(module)));
    let updated = match index {
        Some(index) => replace_line(text, index, &line),
        None => append_line(text, &line),
    };
    if flow_state(&updated, module) == ListState::On {
        return updated;
    }
    // Past the bytes Adrenaline reads: the line goes first instead.
    let rest = match index {
        Some(index) => replace_line(text, index, ""),
        None => String::from(text),
    };
    format!("{line}{}{rest}", newline(text))
}

/// The runlevel, module path and state of one isage list line, or None for a
/// comment or a line with fewer than three fields.
fn isage_line(line: &str) -> Option<(String, String, bool)> {
    let line = line.trim();
    if line.starts_with("//") || line.starts_with(';') || line.starts_with('#') {
        return None;
    }
    let mut fields = line.splitn(3, ',');
    let runlevel = fields.next()?.trim().to_ascii_lowercase();
    let path = fields.next()?.trim();
    let state = fields.next()?;
    // A comment may follow the state.
    let state = state.split(['#', ';']).next().unwrap_or("");
    let state = state.split("//").next().unwrap_or("").trim();
    let path = if path.contains(':') { String::from(path) } else { format!("ms0:/seplugins/{path}") };
    let on = ["on", "true", "enabled"].iter().any(|word| state.eq_ignore_ascii_case(word)) || state == "1";
    Some((runlevel, path, on))
}

/// Whether a runlevel loads at the XMB in isage's Adrenaline.
fn isage_vsh(runlevel: &str) -> bool {
    if runlevel.contains("cfw=") || runlevel.contains('/') {
        return false;
    }
    runlevel == "all" || runlevel == "always" || runlevel.contains("vsh") || runlevel.contains("xmb")
}

/// How an isage list loads `module` at the XMB: the last line for it decides.
pub fn isage_state(text: &str, module: &str) -> ListState {
    let mut state = ListState::Absent;
    for line in lines(text) {
        if let Some((runlevel, path, on)) = isage_line(line) {
            if isage_vsh(&runlevel) && path.eq_ignore_ascii_case(module) {
                state = if on { ListState::On } else { ListState::Off };
            }
        }
    }
    state
}

/// An isage list with `module` on at the XMB: the last line for it set to
/// `vsh, <module>, on`, or a new line at the end.
pub fn isage_enable(text: &str, module: &str) -> String {
    let line = format!("vsh, {module}, on");
    let index = lines(text)
        .enumerate()
        .filter(|(_, line)| {
            isage_line(line).is_some_and(|(runlevel, path, _)| isage_vsh(&runlevel) && path.eq_ignore_ascii_case(module))
        })
        .map(|(index, _)| index)
        .last();
    match index {
        Some(index) => replace_line(text, index, &line),
        None => append_line(text, &line),
    }
}

/// The largest index up to `max` that is on a character boundary.
fn floor_char_boundary(text: &str, max: usize) -> usize {
    let mut at = max.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODULE: &str = "ms0:/seplugins/pocketshelf.prx";

    #[test]
    fn flow_reads_on_off_and_absent() {
        assert_eq!(flow_state("", MODULE), ListState::Absent);
        assert_eq!(flow_state("ms0:/seplugins/other.prx 1\n", MODULE), ListState::Absent);
        assert_eq!(flow_state("ms0:/seplugins/pocketshelf.prx 1\n", MODULE), ListState::On);
        assert_eq!(flow_state("MS0:/SEPLUGINS/PocketShelf.prx\t1\r\n", MODULE), ListState::On);
        assert_eq!(flow_state("ms0:/seplugins/pocketshelf.prx 0\n", MODULE), ListState::Off);
        assert_eq!(flow_state("ms0:/seplugins/pocketshelf.prx\n", MODULE), ListState::Off);
        // Adrenaline compares the whole rest of the line with "1".
        assert_eq!(flow_state("ms0:/seplugins/pocketshelf.prx  1\n", MODULE), ListState::Off);
    }

    #[test]
    fn flow_ignores_a_line_past_what_adrenaline_reads() {
        let filler = format!("ms0:/seplugins/{}.prx 1\n", "x".repeat(40)).repeat(20);
        assert!(filler.len() > FLOW_READ_MAX);
        let text = format!("{filler}{MODULE} 1\n");
        assert_eq!(flow_state(&text, MODULE), ListState::Absent);
    }

    #[test]
    fn flow_enable_turns_on_adds_or_puts_first() {
        assert_eq!(flow_enable("", MODULE), format!("{MODULE} 1\n"));
        assert_eq!(flow_enable("a.prx 1\r\n", MODULE), format!("a.prx 1\r\n{MODULE} 1\r\n"));
        assert_eq!(flow_enable(&format!("a.prx 1\n{MODULE} 0\nb.prx 0\n"), MODULE), format!("a.prx 1\n{MODULE} 1\nb.prx 0\n"));
        let filler = format!("ms0:/seplugins/{}.prx 1\n", "x".repeat(40)).repeat(20);
        let enabled = flow_enable(&filler, MODULE);
        assert!(enabled.starts_with(&format!("{MODULE} 1\n")));
        assert!(enabled.ends_with(&filler));
        assert_eq!(flow_state(&enabled, MODULE), ListState::On);
    }

    #[test]
    fn isage_reads_runlevels_states_and_comments() {
        assert_eq!(isage_state("", MODULE), ListState::Absent);
        assert_eq!(isage_state("vsh, ms0:/seplugins/pocketshelf.prx, on\n", MODULE), ListState::On);
        assert_eq!(isage_state("VSH, pocketshelf.prx, enabled\n", MODULE), ListState::On);
        assert_eq!(isage_state("always, ms0:/seplugins/pocketshelf.prx, 1 # boot\n", MODULE), ListState::On);
        assert_eq!(isage_state("vsh, ms0:/seplugins/pocketshelf.prx, off\n", MODULE), ListState::Off);
        assert_eq!(isage_state("# vsh, ms0:/seplugins/pocketshelf.prx, on\n", MODULE), ListState::Absent);
        assert_eq!(isage_state("game, ms0:/seplugins/pocketshelf.prx, on\n", MODULE), ListState::Absent);
        // The last line for the module decides.
        let both = format!("vsh, {MODULE}, on\nvsh, {MODULE}, off\n");
        assert_eq!(isage_state(&both, MODULE), ListState::Off);
    }

    #[test]
    fn isage_enable_turns_on_the_deciding_line_or_adds_one() {
        assert_eq!(isage_enable("", MODULE), format!("vsh, {MODULE}, on\n"));
        let off = format!("game, a.prx, on\nvsh, {MODULE}, on\nxmb, pocketshelf.prx, off\n");
        let enabled = isage_enable(&off, MODULE);
        assert_eq!(enabled, format!("game, a.prx, on\nvsh, {MODULE}, on\nvsh, {MODULE}, on\n"));
        assert_eq!(isage_state(&enabled, MODULE), ListState::On);
    }
}
