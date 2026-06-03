mod config;
mod keys;
mod client;
mod layout;
mod bar;
mod wm;

use x11rb::connection::Connection;
use x11rb::rust_connection::RustConnection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;

use config::*;
use keys::{
    XK_RETURN, XK_SPACE, XK_J, XK_K, XK_C, XK_Q,
    XK_1, XK_9, SHIFT, NUMLOCK, CAPSLOCK,
};
use bar::StatusBar;
use wm::WM;

fn main() {
    let (conn, screen_num) = RustConnection::connect(None)
        .expect("failed to connect to X11");
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;
    let screen_width = screen.width_in_pixels;
    let screen_height = screen.height_in_pixels;
    let depth = screen.root_depth;
    let visual = screen.root_visual;

    eprintln!("rwm: connected — {}x{}", screen_width, screen_height);

    // ── Become the window manager ───────────────────────────────────
    let mask = EventMask::SUBSTRUCTURE_REDIRECT
        | EventMask::SUBSTRUCTURE_NOTIFY
        | EventMask::ENTER_WINDOW
        | EventMask::STRUCTURE_NOTIFY
        | EventMask::BUTTON_PRESS;

    conn.change_window_attributes(root, &ChangeWindowAttributesAux::new()
        .event_mask(mask))
        .unwrap().check()
        .expect("another WM is already running");

    // ── Grab keybindings ────────────────────────────────────────────
    keys::grab_key(&conn, root, MODKEY, XK_RETURN);
    keys::grab_key(&conn, root, MODKEY | SHIFT, XK_C);
    keys::grab_key(&conn, root, MODKEY, XK_J);
    keys::grab_key(&conn, root, MODKEY, XK_K);
    keys::grab_key(&conn, root, MODKEY | SHIFT, XK_Q);
    keys::grab_key(&conn, root, MODKEY, XK_SPACE);

    // workspace keys: Mod4+1..9 and Mod4+Shift+1..9
    for keysym in XK_1..=XK_9 {
        keys::grab_key(&conn, root, MODKEY, keysym);
        keys::grab_key(&conn, root, MODKEY | SHIFT, keysym);
    }

    // ── Grab mouse buttons for floating ─────────────────────────────
    // Mod4 + Button1 = move, Mod4 + Button3 = resize
    for extra in [0, NUMLOCK, CAPSLOCK, NUMLOCK | CAPSLOCK] {
        conn.grab_button(
            false, root,
            EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE | EventMask::POINTER_MOTION,
            GrabMode::ASYNC, GrabMode::ASYNC,
            0u32, 0u32,
            ButtonIndex::M1,
            ModMask::from(MODKEY | extra),
        ).unwrap();
        conn.grab_button(
            false, root,
            EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE | EventMask::POINTER_MOTION,
            GrabMode::ASYNC, GrabMode::ASYNC,
            0u32, 0u32,
            ButtonIndex::M3,
            ModMask::from(MODKEY | extra),
        ).unwrap();
    }

    conn.flush().unwrap();

    // ── Create bar ──────────────────────────────────────────────────
    let bar = StatusBar::new(&conn, root, screen_width, depth, visual);

    eprintln!("rwm: running");

    let mut wm = WM {
        conn, root, screen_width, screen_height,
        clients: vec![],
        focused: None,
        current_ws: 0,
        bar,
    };

    wm.update_bar();

    // ── Event loop ──────────────────────────────────────────────────
    loop {
        let event = wm.conn.wait_for_event().expect("X11 connection error");
        match event {
            Event::MapRequest(e) => {
                wm.manage(e.window);
            }

            Event::UnmapNotify(e) => {
                wm.unmanage(e.window);
            }

            Event::DestroyNotify(e) => {
                wm.unmanage(e.window);
            }

            Event::EnterNotify(e) => {
                let dominated = wm.clients.iter().any(|c| c.window == e.event
                    && c.workspace == wm.current_ws);
                if dominated && wm.focused != Some(e.event) {
                    wm.focus(e.event);
                }
            }

            Event::Expose(e) => {
                if e.window == wm.bar.window && e.count == 0 {
                    wm.update_bar();
                }
            }

            Event::PropertyNotify(e) => {
                // redraw bar if a window title changes
                if e.atom == AtomEnum::WM_NAME.into() || e.atom == wm.intern_atom("_NET_WM_NAME") {
                    if wm.focused == Some(e.window) {
                        wm.update_bar();
                    }
                }
            }

            Event::ButtonPress(e) => {
                let mods = keys::clean_modifiers(e.state);
                let has_mod = mods & MODKEY != 0;
                if !has_mod { continue; }

                // find the actual client window (e.child is the clicked window)
                let win = e.child;
                if win == 0 || !wm.clients.iter().any(|c| c.window == win) {
                    continue;
                }

                wm.focus(win);

                match e.detail {
                    1 => wm.drag_move(win, e.root_x, e.root_y),   // Button1 = move
                    3 => wm.drag_resize(win, e.root_x, e.root_y), // Button3 = resize
                    _ => {}
                }
            }

            Event::KeyPress(e) => {
                let keysym = keys::keycode_to_keysym(&wm.conn, e.detail);
                let mods = keys::clean_modifiers(e.state);
                let has_shift = mods & SHIFT != 0;

                match keysym {
                    XK_RETURN if !has_shift => wm.spawn(TERMINAL),
                    XK_C if has_shift       => wm.kill_focused(),
                    XK_J if !has_shift      => wm.rotate_next(),
                    XK_K if !has_shift      => wm.rotate_prev(),
                    XK_SPACE if !has_shift  => wm.toggle_floating(),
                    XK_Q if has_shift       => { eprintln!("rwm: quitting"); break; }

                    // workspace switching: Mod4+N / Mod4+Shift+N
                    k @ XK_1..=XK_9 => {
                        let ws = (k - XK_1) as usize;
                        if has_shift {
                            wm.move_to_workspace(ws);
                        } else {
                            wm.switch_workspace(ws);
                        }
                    }

                    _ => {}
                }
            }

            Event::ConfigureRequest(e) => {
                let aux = ConfigureWindowAux::from_configure_request(&e)
                    .sibling(None).stack_mode(None);
                wm.conn.configure_window(e.window, &aux).unwrap();
                wm.conn.flush().unwrap();
            }

            _ => {}
        }
    }
}
