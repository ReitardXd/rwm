use x11rb::connection::Connection;
use x11rb::rust_connection::RustConnection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;

// ── Keysym constants (from X11/keysymdef.h) ─────────────────────────────────
const XK_RETURN: u32 = 0xff0d;
const XK_J: u32      = 0x006a;
const XK_K: u32      = 0x006b;
const XK_C: u32      = 0x0063;
const XK_Q: u32      = 0x0071;

// ── Config ──────────────────────────────────────────────────────────────────
const MODKEY: u16    = u16_from_modmask(ModMask::M4);
const SHIFT: u16     = u16_from_modmask(ModMask::SHIFT);
const NUMLOCK: u16   = u16_from_modmask(ModMask::M2);
const CAPSLOCK: u16  = u16_from_modmask(ModMask::LOCK);
const TERMINAL: &str = "alacritty"; // change to "alacritty", "kitty", etc.

/// const-compatible conversion since From::from isn't const
const fn u16_from_modmask(m: ModMask) -> u16 {
    // ModMask is repr(transparent) newtype around u16.
    // This is safe because From<ModMask> for u16 just returns the inner field.
    unsafe { std::mem::transmute(m) }
}

struct WM {
    conn: RustConnection,
    #[allow(dead_code)]
    root: Window,
    screen_width: u16,
    screen_height: u16,
    clients: Vec<Window>,
    focused: Option<Window>,
}

impl WM {
    // ── Tiling ──────────────────────────────────────────────────────────
    fn tile(&self) {
        let n = self.clients.len();
        if n == 0 { return; }

        let w = self.screen_width as u32;
        let h = self.screen_height as u32;

        if n == 1 {
            self.conn.configure_window(self.clients[0], &ConfigureWindowAux::new()
                .x(0).y(0).width(w).height(h)
                .border_width(0u32)
            ).unwrap();
        } else {
            // master takes left half
            let master = self.clients[0];
            self.conn.configure_window(master, &ConfigureWindowAux::new()
                .x(0).y(0).width(w / 2).height(h)
                .border_width(0u32)
            ).unwrap();

            // stack takes right half, evenly divided
            let stack_count = (n - 1) as u32;
            let stack_h = h / stack_count;
            for (i, &win) in self.clients[1..].iter().enumerate() {
                self.conn.configure_window(win, &ConfigureWindowAux::new()
                    .x((w / 2) as i32)
                    .y((i as u32 * stack_h) as i32)
                    .width(w / 2)
                    .height(stack_h)
                    .border_width(0u32)
                ).unwrap();
            }
        }
        self.conn.flush().unwrap();
    }

    // ── Focus management ────────────────────────────────────────────────
    fn focus(&mut self, win: Window) {
        // unfocus previous
        if let Some(prev) = self.focused {
            let _ = self.conn.change_window_attributes(
                prev,
                &ChangeWindowAttributesAux::new().border_pixel(0x444444),
            );
        }
        self.conn.set_input_focus(InputFocus::POINTER_ROOT, win, x11rb::CURRENT_TIME)
            .unwrap();
        let _ = self.conn.change_window_attributes(
            win,
            &ChangeWindowAttributesAux::new().border_pixel(0xbbbbbb),
        );
        self.focused = Some(win);
        self.conn.flush().unwrap();
    }

    // ── Kill focused window ─────────────────────────────────────────────
    fn kill_focused(&mut self) {
        if let Some(win) = self.focused {
            // try WM_DELETE_WINDOW first, fall back to XKillClient
            let wm_protocols = self.intern_atom("WM_PROTOCOLS");
            let wm_delete = self.intern_atom("WM_DELETE_WINDOW");

            if self.supports_protocol(win, wm_protocols, wm_delete) {
                let event = ClientMessageEvent::new(
                    32,
                    win,
                    wm_protocols,
                    [wm_delete, x11rb::CURRENT_TIME, 0, 0, 0],
                );
                self.conn.send_event(false, win, EventMask::NO_EVENT, event).unwrap();
            } else {
                self.conn.kill_client(win).unwrap();
            }
            self.conn.flush().unwrap();
        }
    }

    fn supports_protocol(&self, win: Window, wm_protocols: Atom, proto: Atom) -> bool {
        let Ok(cookie) = self.conn.get_property(
            false, win, wm_protocols, AtomEnum::ATOM, 0, 1024,
        ) else { return false };

        let Ok(reply) = cookie.reply() else { return false };

        if let Some(iter) = reply.value32() {
            let atoms: Vec<Atom> = iter.collect();
            return atoms.contains(&proto);
        }
        false
    }

    fn intern_atom(&self, name: &str) -> Atom {
        self.conn.intern_atom(false, name.as_bytes())
            .unwrap().reply().unwrap().atom
    }

    // ── Stack rotation ──────────────────────────────────────────────────
    /// Rotate forward: master goes to end of stack, second becomes master
    fn rotate_next(&mut self) {
        if self.clients.len() < 2 { return; }
        let first = self.clients.remove(0);
        self.clients.push(first);
        self.tile();
        self.focus(self.clients[0]);
    }

    /// Rotate backward: last in stack becomes master
    fn rotate_prev(&mut self) {
        if self.clients.len() < 2 { return; }
        let last = self.clients.pop().unwrap();
        self.clients.insert(0, last);
        self.tile();
        self.focus(self.clients[0]);
    }

    // ── Spawn ───────────────────────────────────────────────────────────
    fn spawn(&self, cmd: &str) {
        use std::process::Command;
        let _ = Command::new(cmd)
            .spawn();
    }
}

// ── Keysym → keycode translation ────────────────────────────────────────────
fn keysym_to_keycode(conn: &RustConnection, keysym: u32) -> Option<Keycode> {
    let setup = conn.setup();
    let min_kc = setup.min_keycode;
    let max_kc = setup.max_keycode;
    let count = max_kc - min_kc + 1;

    let reply = conn.get_keyboard_mapping(min_kc, count)
        .unwrap().reply().unwrap();
    let syms_per_kc = reply.keysyms_per_keycode as usize;

    for i in 0..(count as usize) {
        for col in 0..syms_per_kc {
            if reply.keysyms[i * syms_per_kc + col] == keysym {
                return Some(min_kc + i as u8);
            }
        }
    }
    None
}

fn grab_key(conn: &RustConnection, root: Window, modmask: u16, keysym: u32) {
    if let Some(keycode) = keysym_to_keycode(conn, keysym) {
        // grab with exact modifier
        conn.grab_key(
            true, root,
            ModMask::from(modmask),
            keycode,
            GrabMode::ASYNC, GrabMode::ASYNC,
        ).unwrap();
        // also grab with NumLock (Mod2) held
        conn.grab_key(
            true, root,
            ModMask::from(modmask | NUMLOCK),
            keycode,
            GrabMode::ASYNC, GrabMode::ASYNC,
        ).unwrap();
        // also grab with CapsLock (Lock) held
        conn.grab_key(
            true, root,
            ModMask::from(modmask | CAPSLOCK),
            keycode,
            GrabMode::ASYNC, GrabMode::ASYNC,
        ).unwrap();
        // NumLock + CapsLock
        conn.grab_key(
            true, root,
            ModMask::from(modmask | NUMLOCK | CAPSLOCK),
            keycode,
            GrabMode::ASYNC, GrabMode::ASYNC,
        ).unwrap();
    } else {
        eprintln!("rwm: warning: no keycode for keysym 0x{:04x}", keysym);
    }
}

/// Determine keysym from a KeyPress keycode (column 0 = unshifted)
fn keycode_to_keysym(conn: &RustConnection, keycode: Keycode) -> u32 {
    let setup = conn.setup();
    let min_kc = setup.min_keycode;
    let max_kc = setup.max_keycode;
    let count = max_kc - min_kc + 1;

    let reply = conn.get_keyboard_mapping(min_kc, count)
        .unwrap().reply().unwrap();
    let syms_per_kc = reply.keysyms_per_keycode as usize;

    let idx = (keycode - min_kc) as usize;
    if idx * syms_per_kc < reply.keysyms.len() {
        reply.keysyms[idx * syms_per_kc] // column 0 = unshifted
    } else {
        0
    }
}

/// Strip NumLock, CapsLock, ScrollLock from modifier state
fn clean_modifiers(state: KeyButMask) -> u16 {
    let raw = u16::from(state);
    raw & !(NUMLOCK | CAPSLOCK)
}

fn main() {
    let (conn, screen_num) = RustConnection::connect(None)
        .expect("failed to connect to X11");
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;
    let screen_width = screen.width_in_pixels;
    let screen_height = screen.height_in_pixels;

    eprintln!("rwm: connected to X11 — {}x{}", screen_width, screen_height);

    // ── Register for substructure events + enter notify ─────────────
    let mask = EventMask::SUBSTRUCTURE_REDIRECT
        | EventMask::SUBSTRUCTURE_NOTIFY
        | EventMask::ENTER_WINDOW
        | EventMask::STRUCTURE_NOTIFY;

    conn.change_window_attributes(root, &ChangeWindowAttributesAux::new()
        .event_mask(mask))
        .unwrap().check()
        .expect("another WM is already running");

    // ── Grab keybindings ────────────────────────────────────────────
    // Mod4 + Return  → spawn terminal
    grab_key(&conn, root, MODKEY, XK_RETURN);
    // Mod4 + Shift + c → kill focused window
    grab_key(&conn, root, MODKEY | SHIFT, XK_C);
    // Mod4 + j → rotate stack forward (next window becomes master)
    grab_key(&conn, root, MODKEY, XK_J);
    // Mod4 + k → rotate stack backward (last window becomes master)
    grab_key(&conn, root, MODKEY, XK_K);
    // Mod4 + Shift + q → quit WM
    grab_key(&conn, root, MODKEY | SHIFT, XK_Q);

    conn.flush().unwrap();

    eprintln!("rwm: running");

    let mut wm = WM {
        conn,
        root,
        screen_width,
        screen_height,
        clients: vec![],
        focused: None,
    };

    loop {
        let event = wm.conn.wait_for_event().expect("X11 connection error");
        match event {
            // ── New window wants to be mapped ───────────────────────
            Event::MapRequest(e) => {
                eprintln!("rwm: MapRequest 0x{:x}", e.window);
                // subscribe to enter events on this window
                let _ = wm.conn.change_window_attributes(
                    e.window,
                    &ChangeWindowAttributesAux::new()
                        .event_mask(EventMask::ENTER_WINDOW | EventMask::STRUCTURE_NOTIFY),
                );
                wm.clients.push(e.window);
                wm.conn.map_window(e.window).unwrap();
                wm.tile();
                wm.focus(e.window);
            }

            // ── Window unmapped / destroyed ─────────────────────────
            Event::UnmapNotify(e) => {
                if wm.clients.contains(&e.window) {
                    eprintln!("rwm: UnmapNotify 0x{:x}", e.window);
                    wm.clients.retain(|&w| w != e.window);
                    if wm.focused == Some(e.window) {
                        wm.focused = None;
                    }
                    wm.tile();
                    if let Some(&first) = wm.clients.first() {
                        wm.focus(first);
                    }
                }
            }

            Event::DestroyNotify(e) => {
                if wm.clients.contains(&e.window) {
                    eprintln!("rwm: DestroyNotify 0x{:x}", e.window);
                    wm.clients.retain(|&w| w != e.window);
                    if wm.focused == Some(e.window) {
                        wm.focused = None;
                    }
                    wm.tile();
                    if let Some(&first) = wm.clients.first() {
                        wm.focus(first);
                    }
                }
            }

            // ── Mouse enters a window → focus follows mouse ────────
            Event::EnterNotify(e) => {
                if wm.clients.contains(&e.event) && wm.focused != Some(e.event) {
                    wm.focus(e.event);
                }
            }

            // ── Keybindings ─────────────────────────────────────────
            Event::KeyPress(e) => {
                let keysym = keycode_to_keysym(&wm.conn, e.detail);
                let mods = clean_modifiers(e.state);
                let has_shift = mods & SHIFT != 0;

                match keysym {
                    // Mod4 + Return → terminal
                    XK_RETURN if !has_shift => {
                        eprintln!("rwm: spawning {}", TERMINAL);
                        wm.spawn(TERMINAL);
                    }

                    // Mod4 + Shift + c → kill
                    XK_C if has_shift => {
                        eprintln!("rwm: killing focused window");
                        wm.kill_focused();
                    }

                    // Mod4 + j → rotate forward
                    XK_J if !has_shift => {
                        wm.rotate_next();
                    }

                    // Mod4 + k → rotate backward
                    XK_K if !has_shift => {
                        wm.rotate_prev();
                    }

                    // Mod4 + Shift + q → quit
                    XK_Q if has_shift => {
                        eprintln!("rwm: quitting");
                        break;
                    }

                    _ => {}
                }
            }

            // ── Configure requests (pass through) ───────────────────
            Event::ConfigureRequest(e) => {
                let aux = ConfigureWindowAux::from_configure_request(&e)
                    .sibling(None)
                    .stack_mode(None);
                wm.conn.configure_window(e.window, &aux).unwrap();
                wm.conn.flush().unwrap();
            }

            _ => {}
        }
    }
}
