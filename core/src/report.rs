//! The Diagnostics screen's lines from the host's reports: what the last
//! title scan found (`Host::scan_report`) and the Adrenaline boot plugin's
//! last log lines (`Host::boot_log`).

/// Lines from a report of `Label=value` pairs separated by commas:
/// `PSM folders=3,PSM source=app.db,PSP titles=2`. Nothing when this start
/// read the saved title list instead of scanning.
pub fn pairs(report: &str) -> Vec<(String, String)> {
    report
        .split(',')
        .filter_map(|pair| {
            let mut parts = pair.split('=');
            let (key, value) = (parts.next()?, parts.next()?);
            (!key.is_empty() && !value.is_empty()).then(|| (key.to_string(), value.to_string()))
        })
        .collect()
}

/// The ISO drivers by the boot config index the plugin logs; each Adrenaline
/// numbers its drivers its own way, these are the plugin's numbers.
fn driver_name(index: &str) -> &str {
    match index {
        "1" | "4" => "Inferno",
        "2" => "March33",
        "3" => "NP9660",
        "5" => "ME",
        other => other,
    }
}

/// At most 16 characters, the rest as "...".
fn short(text: &str) -> String {
    if text.chars().count() > 16 {
        format!("{}...", text.chars().take(13).collect::<String>())
    } else {
        text.to_string()
    }
}

const OUTCOMES: [&str; 6] = ["stale", "bad kind", "bad path", "bad boot file", "missing", "refused"];

/// One boot log line (`psp-boot/main.c`), oldest first:
///
///   `trigger=after <module>` or `trigger=fallback`: what the plugin waited for
///   `driver=<n> [(default)] <file>`: the ISO driver an image boots with
///   `age=3s path=ms0:/ISO/Game.cso boot iso [code=<hex>]`: a request and
///   what became of it, shown as "Boot iso" with the age and the file name
///   (an EBOOT by its folder, the title id)
fn boot_line(line: &str) -> (String, String) {
    if let Some(name) = line.strip_prefix("trigger=") {
        let name = name.strip_prefix("after ").unwrap_or(name);
        return ("Boot trigger".into(), short(name));
    }
    if let Some(rest) = line.strip_prefix("driver=") {
        let mut words = rest.splitn(2, ' ');
        let (index, rest) = (words.next().unwrap_or(""), words.next().unwrap_or(""));
        let (default, file) = match rest.strip_prefix("(default) ") {
            Some(file) => (" (default)", file),
            None => ("", rest),
        };
        if !index.is_empty() && index.bytes().all(|b| b.is_ascii_digit()) && !file.is_empty() && !file.contains(' ') {
            return ("Boot driver".into(), format!("{}{default} {file}", driver_name(index)));
        }
    }
    let parsed = (|| {
        let rest = line.strip_prefix("age=")?;
        let (age, rest) = rest.split_once(" path=")?;
        let (rest, code) = match rest.rsplit_once(" code=") {
            Some((rest, code)) if !code.contains(' ') => (rest, Some(code)),
            _ => (rest, None),
        };
        // The outcome is the line's last words: one of `OUTCOMES`, or
        // `boot <kind>`.
        let (path, outcome) = OUTCOMES
            .iter()
            .find_map(|o| rest.strip_suffix(o).and_then(|p| p.strip_suffix(' ')).map(|p| (p, o.to_string())))
            .or_else(|| {
                let (path, kind) = rest.rsplit_once(" boot ")?;
                (!kind.is_empty() && kind.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')).then(|| (path, kind.to_string()))
            })?;
        Some((age.to_string(), path.to_string(), outcome, code.map(String::from)))
    })();
    let Some((age, path, outcome, code)) = parsed else {
        return ("Boot".into(), line.chars().take(32).collect());
    };
    let parts: Vec<&str> = path.split('/').collect();
    let file = if parts.last() == Some(&"EBOOT.PBP") { parts.iter().rev().nth(1).copied().unwrap_or("") } else { parts.last().copied().unwrap_or("") };
    let code = code.map(|c| format!("{c} ")).unwrap_or_default();
    (format!("Boot {outcome}"), format!("{age} {code}{}", short(file)))
}

/// The boot log's lines, oldest first.
pub fn boot(log: &str) -> Vec<(String, String)> {
    log.lines().filter(|line| !line.is_empty()).map(boot_line).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(label: &str, value: &str) -> (String, String) {
        (label.to_string(), value.to_string())
    }

    #[test]
    fn reports_are_label_value_pairs() {
        assert_eq!(pairs("PSM folders=3,PSM source=app.db,PSP titles=2"), [line("PSM folders", "3"), line("PSM source", "app.db"), line("PSP titles", "2")]);
        assert_eq!(pairs("skip=PCSE00001 noEboot,broken,=x,y="), [line("skip", "PCSE00001 noEboot")]);
        assert!(pairs("").is_empty());
    }

    #[test]
    fn boot_log_lines_name_their_outcome() {
        let log = "trigger=after sceVshBridge_Driver_Long_Name\ndriver=3 (default) EBOOT.OLD\ndriver=1 BOOT.BIN\n\
                   age=3s path=ms0:/ISO/Racing/Ridge Racer.cso boot iso\nage=41s path=ms0:/PSP/GAME/SLUS00373/EBOOT.PBP stale\n\
                   age=2s path=ms0:/PSP/GAME/SCUS94163/EBOOT.PBP refused code=0x80020001\nsomething else entirely here";
        assert_eq!(
            boot(log),
            [
                line("Boot trigger", "sceVshBridge_..."),
                line("Boot driver", "NP9660 (default) EBOOT.OLD"),
                line("Boot driver", "Inferno BOOT.BIN"),
                line("Boot iso", "3s Ridge Racer.cso"),
                line("Boot stale", "41s SLUS00373"),
                line("Boot refused", "2s 0x80020001 SCUS94163"),
                line("Boot", "something else entirely here"),
            ]
        );
        assert_eq!(boot("trigger=fallback"), [line("Boot trigger", "fallback")]);
    }
}
