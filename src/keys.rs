#![allow(dead_code)]

use x11rb::connection::Connection;
use x11rb::rust_connection::RustConnection;
use x11rb::protocol::xproto::*;

// ── Keysym constants (X11/keysymdef.h) ──────────────────────────────────────
pub const XK_RETURN: u32 = 0xff0d;
pub const XK_SPACE: u32  = 0x0020;
pub const XK_J: u32      = 0x006a;
pub const XK_K: u32      = 0x006b;
pub const XK_C: u32      = 0x0063;
pub const XK_F: u32      = 0x0066;
pub const XK_Q: u32      = 0x0071;
pub const XK_1: u32      = 0x0031;
pub const XK_2: u32      = 0x0032;
pub const XK_3: u32      = 0x0033;
pub const XK_4: u32      = 0x0034;
pub const XK_5: u32      = 0x0035;
pub const XK_6: u32      = 0x0036;
pub const XK_7: u32      = 0x0037;
pub const XK_8: u32      = 0x0038;
pub const XK_9: u32      = 0x0039;

// modifier masks (raw values matching ModMask/KeyButMask internals)
pub const SHIFT: u16    = 1 << 0;
pub const NUMLOCK: u16  = 1 << 4; // Mod2
pub const CAPSLOCK: u16 = 1 << 1; // Lock

/// Translate keysym → keycode using the server's keyboard mapping
pub fn keysym_to_keycode(conn: &RustConnection, keysym: u32) -> Option<Keycode> {
    let setup = conn.setup();
    let min = setup.min_keycode;
    let count = setup.max_keycode - min + 1;
    let reply = conn.get_keyboard_mapping(min, count).unwrap().reply().unwrap();
    let per = reply.keysyms_per_keycode as usize;

    for i in 0..(count as usize) {
        for col in 0..per {
            if reply.keysyms[i * per + col] == keysym {
                return Some(min + i as u8);
            }
        }
    }
    None
}

/// Translate keycode → keysym (column 0 = unshifted)
pub fn keycode_to_keysym(conn: &RustConnection, keycode: Keycode) -> u32 {
    let setup = conn.setup();
    let min = setup.min_keycode;
    let count = setup.max_keycode - min + 1;
    let reply = conn.get_keyboard_mapping(min, count).unwrap().reply().unwrap();
    let per = reply.keysyms_per_keycode as usize;
    let idx = (keycode - min) as usize;
    if idx * per < reply.keysyms.len() {
        reply.keysyms[idx * per]
    } else {
        0
    }
}

/// Grab a key on root, covering NumLock/CapsLock combos
pub fn grab_key(conn: &RustConnection, root: Window, modmask: u16, keysym: u32) {
    let Some(kc) = keysym_to_keycode(conn, keysym) else {
        eprintln!("rwm: warning: no keycode for keysym 0x{:04x}", keysym);
        return;
    };
    for extra in [0, NUMLOCK, CAPSLOCK, NUMLOCK | CAPSLOCK] {
        conn.grab_key(
            true, root,
            ModMask::from(modmask | extra),
            kc,
            GrabMode::ASYNC, GrabMode::ASYNC,
        ).unwrap();
    }
}

/// Strip NumLock + CapsLock from modifier state
pub fn clean_modifiers(state: KeyButMask) -> u16 {
    u16::from(state) & !(NUMLOCK | CAPSLOCK)
}
