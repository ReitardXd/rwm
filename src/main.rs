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
    XK_RETURN, XK_J, XK_K, XK_C, XK_F, XK_Q,
    XK_1, XK_9, SHIFT,
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
        | EventMask::STRUCTURE_NOTIFY;

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
    keys::grab_key(&conn, root, MODKEY, XK_F);

    // workspace keys: Mod4+1..9 and Mod4+Shift+1..9
    for keysym in XK_1..=XK_9 {
        keys::grab_key(&conn, root, MODKEY, keysym);
        keys::grab_key(&conn, root, MODKEY | SHIFT, keysym);
    }

    conn.flush().unwrap();

    // ── Create bar ──────────────────────────────────────────────────
    let bar = StatusBar::new(&conn, root, screen_width, depth, visual);

    eprintln!("rwm: running");

    let mut wm = WM::new(conn, root, screen_width, screen_height, bar);
    wm.update_bar();

    // ── Event loop ──────────────────────────────────────────────────
    loop {
        let event = wm.conn.wait_for_event().expect("X11 connection error");
        match event {
            Event::MapRequest(e) => {
                wm.manage(e.window);
            }

            Event::UnmapNotify(e) => {
                // ignore unmaps we caused (workspace switch)
                if !wm.consume_pending_unmap(e.window) {
                    wm.unmanage(e.window);
                }
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
                if e.atom == AtomEnum::WM_NAME.into() || e.atom == wm.intern_atom("_NET_WM_NAME") {
                    if wm.focused == Some(e.window) {
                        wm.update_bar();
                    }
                }
            }

            Event::ButtonPress(e) => {
                // clicking on the bar workspace area → switch workspace
                if e.event == wm.bar.window {
                    if let Some(ws) = wm.bar.ws_from_click(e.event_x) {
                        wm.switch_workspace(ws);
                    }
                }
            }

            Event::KeyPress(e) => {
                let keysym = keys::keycode_to_keysym(&wm.conn, e.detail);
                let mods = keys::clean_modifiers(e.state);
                let has_shift = mods & SHIFT != 0;

                match keysym {
                    XK_RETURN if !has_shift => wm.spawn(TERMINAL),
                    XK_C if has_shift       => wm.kill_focused(),
                    XK_J if !has_shift      => wm.focus_next(),
                    XK_K if !has_shift      => wm.focus_prev(),
                    XK_F if !has_shift      => wm.toggle_fullscreen(),
                    XK_Q if has_shift       => { eprintln!("rwm: quitting"); break; }

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
