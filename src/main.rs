mod config;
mod keys;
mod client;
mod layout;
mod bar;
mod wm;
//you can add more files here if you wish to make it more feature rich but this is what i prefer
//since i mainly use dwm specifically luke smith's configs 
use x11rb::connection::Connection;
use x11rb::rust_connection::RustConnection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;

use config::*;
use keys::*;
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

    // ── Grab keybindings ────────────────────

    // core wm controls
    keys::grab_key(&conn, root, MODKEY, XK_RETURN);              // terminal
    keys::grab_key(&conn, root, MODKEY, XK_KP_ENTER);           // terminal (numpad enter)
    keys::grab_key(&conn, root, MODKEY, XK_D);                   // dmenu
    keys::grab_key(&conn, root, MODKEY, XK_Q);                   // kill window
    keys::grab_key(&conn, root, MODKEY | SHIFT, XK_Q);           // sysact / quit wm
    keys::grab_key(&conn, root, MODKEY, XK_J);                   // focus next
    keys::grab_key(&conn, root, MODKEY, XK_K);                   // focus prev
    keys::grab_key(&conn, root, MODKEY, XK_F);                   // fullscreen
    keys::grab_key(&conn, root, MODKEY, XK_TAB);                 // last workspace
    keys::grab_key(&conn, root, MODKEY, XK_BACKSPACE);           // sysact

    // app launchers
    keys::grab_key(&conn, root, MODKEY, XK_W);                   // browser
    keys::grab_key(&conn, root, MODKEY | SHIFT, XK_W);           // nmtui
    keys::grab_key(&conn, root, MODKEY, XK_R);                   // file manager
    keys::grab_key(&conn, root, MODKEY | SHIFT, XK_R);           // htop
    keys::grab_key(&conn, root, MODKEY, XK_N);                   // nvim wiki
    keys::grab_key(&conn, root, MODKEY, XK_M);                   // ncmpcpp
    keys::grab_key(&conn, root, MODKEY | SHIFT, XK_M);           // mute toggle
    keys::grab_key(&conn, root, MODKEY, XK_P);                   // mpc toggle
    keys::grab_key(&conn, root, MODKEY | SHIFT, XK_P);           // mpc pause

    // volume
    keys::grab_key(&conn, root, MODKEY, XK_MINUS);               // vol -5%
    keys::grab_key(&conn, root, MODKEY | SHIFT, XK_MINUS);       // vol -15%
    keys::grab_key(&conn, root, MODKEY, XK_EQUAL);               // vol +5%
    keys::grab_key(&conn, root, MODKEY | SHIFT, XK_EQUAL);       // vol +15%

    // screenshot
    keys::grab_key(&conn, root, 0, XK_PRINT);                    // full screenshot
    keys::grab_key(&conn, root, SHIFT, XK_PRINT);                // maimpick

    // XF86 media keys (no modifier)
    keys::grab_key(&conn, root, 0, XF86XK_AUDIO_MUTE);
    keys::grab_key(&conn, root, 0, XF86XK_AUDIO_LOWER_VOLUME);
    keys::grab_key(&conn, root, 0, XF86XK_AUDIO_RAISE_VOLUME);
    keys::grab_key(&conn, root, 0, XF86XK_AUDIO_PLAY);
    keys::grab_key(&conn, root, 0, XF86XK_AUDIO_STOP);
    keys::grab_key(&conn, root, 0, XF86XK_AUDIO_PREV);
    keys::grab_key(&conn, root, 0, XF86XK_AUDIO_NEXT);
    keys::grab_key(&conn, root, 0, XF86XK_MON_BRIGHTNESS_UP);
    keys::grab_key(&conn, root, 0, XF86XK_MON_BRIGHTNESS_DOWN);
    keys::grab_key(&conn, root, 0, XF86XK_AUDIO_MIC_MUTE);

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
    let mut last_ws: usize = 0; // for Super+Tab
    wm.update_bar();

    // ── Event loop ──────────────────────────────────────────────────
    loop {
        let event = wm.conn.wait_for_event().expect("X11 connection error");
        match event {
            Event::MapRequest(e) => {
                wm.manage(e.window);
            }

            Event::UnmapNotify(e) => {
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
                if e.event == wm.bar.window {
                    if let Some(ws) = wm.bar.ws_from_click(e.event_x) {
                        let prev = wm.current_ws;
                        wm.switch_workspace(ws);
                        last_ws = prev;
                    }
                }
            }

            Event::KeyPress(e) => {
                let keysym = keys::keycode_to_keysym(&wm.conn, e.detail);
                let mods = keys::clean_modifiers(e.state);
                let has_shift = mods & SHIFT != 0;
                let modkey_only = mods == MODKEY;
                let mod_shift = mods == MODKEY | SHIFT;

                match keysym {
                    // ── Core WM ─────────────────────────────────────
                    XK_RETURN if modkey_only => wm.spawn(TERMINAL),
                    XK_KP_ENTER if modkey_only => wm.spawn(TERMINAL),
                    XK_Q if modkey_only      => wm.kill_focused(),
                    XK_Q if mod_shift        => { eprintln!("rwm: quitting"); break; }
                    XK_J if modkey_only      => wm.focus_next(),
                    XK_K if modkey_only      => wm.focus_prev(),
                    XK_D if modkey_only      => wm.spawn(LAUNCHER),
                    XK_F if modkey_only      => wm.toggle_fullscreen(),
                    XK_TAB if modkey_only    => {
                        let prev = wm.current_ws;
                        wm.switch_workspace(last_ws);
                        last_ws = prev;
                    }
                    XK_BACKSPACE if modkey_only => wm.spawn("sysact"),

                    // ── App launchers ───────────────────────────────
                    XK_W if modkey_only      => wm.spawn(BROWSER),
                    XK_W if mod_shift        => wm.spawn_args(TERMINAL, &["nmtui"]),
                    XK_R if modkey_only      => wm.spawn_args(TERMINAL, &["lfub"]),
                    XK_R if mod_shift        => wm.spawn_args(TERMINAL, &["htop"]),
                    XK_N if modkey_only      => wm.spawn_args(TERMINAL, &["nvim", "-c", "VimwikiIndex"]),
                    XK_M if modkey_only      => wm.spawn_args(TERMINAL, &["ncmpcpp"]),
                    XK_M if mod_shift        => wm.spawn_sh("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"),
                    XK_P if modkey_only      => wm.spawn_args("mpc", &["toggle"]),
                    XK_P if mod_shift        => wm.spawn_sh("mpc pause; pauseallmpv"),

                    // ── Volume (Super+minus/equal) ──────────────────
                    XK_MINUS if modkey_only  => wm.spawn_sh("wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-"),
                    XK_MINUS if mod_shift    => wm.spawn_sh("wpctl set-volume @DEFAULT_AUDIO_SINK@ 15%-"),
                    XK_EQUAL if modkey_only  => wm.spawn_sh("wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%+"),
                    XK_EQUAL if mod_shift    => wm.spawn_sh("wpctl set-volume @DEFAULT_AUDIO_SINK@ 15%+"),

                    // ── Screenshot ──────────────────────────────────
                    XK_PRINT if mods == 0    => wm.spawn_sh("maim pic-full-$(date '+%y%m%d-%H%M-%S').png"),
                    XK_PRINT if mods == SHIFT => wm.spawn("maimpick"),

                    // ── XF86 media keys (no modifier) ───────────────
                    XF86XK_AUDIO_MUTE if mods == 0          => wm.spawn_sh("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"),
                    XF86XK_AUDIO_LOWER_VOLUME if mods == 0  => wm.spawn_sh("wpctl set-volume @DEFAULT_AUDIO_SINK@ 3%-"),
                    XF86XK_AUDIO_RAISE_VOLUME if mods == 0  => wm.spawn_sh("wpctl set-volume @DEFAULT_AUDIO_SINK@ 3%+"),
                    XF86XK_AUDIO_PLAY if mods == 0          => wm.spawn_args("mpc", &["toggle"]),
                    XF86XK_AUDIO_STOP if mods == 0          => wm.spawn_args("mpc", &["stop"]),
                    XF86XK_AUDIO_PREV if mods == 0          => wm.spawn_args("mpc", &["prev"]),
                    XF86XK_AUDIO_NEXT if mods == 0          => wm.spawn_args("mpc", &["next"]),
                    XF86XK_MON_BRIGHTNESS_UP if mods == 0   => wm.spawn_args("xbacklight", &["-inc", "15"]),
                    XF86XK_MON_BRIGHTNESS_DOWN if mods == 0 => wm.spawn_args("xbacklight", &["-dec", "15"]),
                    XF86XK_AUDIO_MIC_MUTE if mods == 0      => wm.spawn_sh("pactl set-source-mute @DEFAULT_SOURCE@ toggle"),

                    // ── Workspaces ──────────────────────────────────
                    k @ XK_1..=XK_9 => {
                        let ws = (k - XK_1) as usize;
                        if has_shift {
                            wm.move_to_workspace(ws);
                        } else {
                            let prev = wm.current_ws;
                            wm.switch_workspace(ws);
                            last_ws = prev;
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
