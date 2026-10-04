//! Adrenaline's Vita-side settings file, for where it keeps the PSP's memory
//! stick (`ms0:`). Pure byte work; `pspemu.rs` reads the file. The tests run
//! natively: `rustc --edition 2021 --test hosts/vita/src/adrenaline_config.rs`.
//!
//! The file is `ux0:data/PSPEMUCFW/adrenaline.bin` (isage's Adrenaline, which
//! reads it first) or `ux0:app/PSPEMUCFW/adrenaline.bin` (both). Two layouts,
//! told apart by the second magic number:
//! - `0x334F4E33`, TheOfficialFloW's (and isage's before 8.0): `int` fields,
//!   the location at byte 24.
//! - `0x8451860B`, isage's: one-byte fields, the location at byte 11.
//!
//! Locations: 0 `ux0:pspemu`, 1 `ur0:pspemu`, 2 `imc0:pspemu`, 3
//! `xmc0:pspemu`, 4 `uma0:pspemu`, and in isage's only 5, the root of `uma0:`.

const MAGIC_1: u32 = 0x3148_3943;
const MAGIC_2_INT: u32 = 0x334F_4E33;
const MAGIC_2_BYTE: u32 = 0x8451_860B;

/// Where Adrenaline keeps `ms0:` by default, and when its settings cannot be read.
pub const DEFAULT_STICK: &str = "ux0:/pspemu";

fn word(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// The folder that is `ms0:` to the PSP side, from the settings file's bytes;
/// None for a file that is not one of the two layouts or names no location.
pub fn stick_folder(config: &[u8]) -> Option<&'static str> {
    if word(config, 0)? != MAGIC_1 {
        return None;
    }
    let location = match word(config, 4)? {
        MAGIC_2_INT => word(config, 24)?,
        MAGIC_2_BYTE => *config.get(11)? as u32,
        _ => return None,
    };
    Some(match location {
        0 => "ux0:/pspemu",
        1 => "ur0:/pspemu",
        2 => "imc0:/pspemu",
        3 => "xmc0:/pspemu",
        4 => "uma0:/pspemu",
        5 => "uma0:",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int_layout(location: u32) -> Vec<u8> {
        let mut bytes = vec![0u8; 56];
        bytes[..4].copy_from_slice(&MAGIC_1.to_le_bytes());
        bytes[4..8].copy_from_slice(&MAGIC_2_INT.to_le_bytes());
        bytes[24..28].copy_from_slice(&location.to_le_bytes());
        bytes
    }

    fn byte_layout(location: u8) -> Vec<u8> {
        let mut bytes = vec![0u8; 34];
        bytes[..4].copy_from_slice(&MAGIC_1.to_le_bytes());
        bytes[4..8].copy_from_slice(&MAGIC_2_BYTE.to_le_bytes());
        bytes[11] = location;
        bytes
    }

    #[test]
    fn reads_both_layouts() {
        assert_eq!(stick_folder(&int_layout(0)), Some("ux0:/pspemu"));
        assert_eq!(stick_folder(&int_layout(1)), Some("ur0:/pspemu"));
        assert_eq!(stick_folder(&int_layout(4)), Some("uma0:/pspemu"));
        assert_eq!(stick_folder(&byte_layout(2)), Some("imc0:/pspemu"));
        assert_eq!(stick_folder(&byte_layout(5)), Some("uma0:"));
    }

    #[test]
    fn refuses_other_files() {
        assert_eq!(stick_folder(&[]), None);
        assert_eq!(stick_folder(&int_layout(9)), None);
        let mut wrong = int_layout(1);
        wrong[4] ^= 1;
        assert_eq!(stick_folder(&wrong), None);
        // Too short for the int layout's field.
        assert_eq!(stick_folder(&int_layout(1)[..20]), None);
    }
}
