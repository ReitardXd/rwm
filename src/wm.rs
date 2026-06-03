use x11rb::connection::Connection;
use x11rb::rust_connection::RustConnection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;

use crate::config::*;
use crate::client::Client;
use crate::layout;
use crate::bar::StatusBar;

pub struct WM {
    pub conn: RustConnection,
    pub root: Window,
    pub screen_width: u16,
    pub screen_height: u16,
    pub clients: Vec<Client>,
    pub focused: Option<Window>,
    pub current_ws: usize,
    pub bar: StatusBar,
}

impl WM {
    // ── Layout ──────────────────────────────────────────────────────────

    /// Apply tiling layout to current workspace, then configure all windows
    pub fn apply_layout(&mut self) {
        let sw = self.screen_width as u32;
        let sh = self.screen_height as u32;

        // collect mutable refs to clients on current workspace
        let mut ws_clients: Vec<&mut Client> = self.clients.iter_mut()
            .filter(|c| c.workspace == self.current_ws)
            .collect();

        layout::tile(&mut ws_clients, sw, sh);

        // apply geometry to X11
        for c in &ws_clients {
            self.conn.configure_window(c.window, &ConfigureWindowAux::new()
                .x(c.x).y(c.y).width(c.w).height(c.h)
                .border_width(0u32)
            ).unwrap();
        }
        self.conn.flush().unwrap();
        self.update_bar();
    }

    // ── Focus ───────────────────────────────────────────────────────────

    pub fn focus(&mut self, win: Window) {
        if let Some(prev) = self.focused {
            let _ = self.conn.change_window_attributes(
                prev, &ChangeWindowAttributesAux::new().border_pixel(BORDER_UNFOCUSED),
            );
        }
        self.conn.set_input_focus(InputFocus::POINTER_ROOT, win, x11rb::CURRENT_TIME)
            .unwrap();
        let _ = self.conn.change_window_attributes(
            win, &ChangeWindowAttributesAux::new().border_pixel(BORDER_FOCUSED),
        );
        self.focused = Some(win);
        self.conn.flush().unwrap();
        self.update_bar();
    }

    /// Focus the first tiled window on current workspace, or clear focus
    pub fn focus_top(&mut self) {
        let top = self.clients.iter()
            .find(|c| c.workspace == self.current_ws)
            .map(|c| c.window);
        if let Some(w) = top {
            self.focus(w);
        } else {
            self.focused = None;
            self.conn.set_input_focus(InputFocus::POINTER_ROOT, self.root, x11rb::CURRENT_TIME)
                .unwrap();
            self.conn.flush().unwrap();
            self.update_bar();
        }
    }

    // ── Manage / Unmanage ───────────────────────────────────────────────

    pub fn manage(&mut self, win: Window) {
        // don't manage the bar
        if win == self.bar.window { return; }

        // subscribe to events on the client
        let _ = self.conn.change_window_attributes(win,
            &ChangeWindowAttributesAux::new()
                .event_mask(EventMask::ENTER_WINDOW | EventMask::STRUCTURE_NOTIFY
                    | EventMask::PROPERTY_CHANGE),
        );

        let mut client = Client::new(win, self.current_ws);

        // check if window requests floating (transient_for or fixed size)
        if let Ok(cookie) = self.conn.get_property(
            false, win, AtomEnum::WM_TRANSIENT_FOR, AtomEnum::WINDOW, 0, 1,
        ) {
            if let Ok(reply) = cookie.reply() {
                if reply.value_len > 0 { client.floating = true; }
            }
        }

        self.clients.push(client);
        self.conn.map_window(win).unwrap();
        self.apply_layout();
        self.focus(win);
    }

    pub fn unmanage(&mut self, win: Window) {
        if !self.clients.iter().any(|c| c.window == win) { return; }
        self.clients.retain(|c| c.window != win);
        if self.focused == Some(win) { self.focused = None; }
        self.apply_layout();
        self.focus_top();
    }

    // ── Kill ────────────────────────────────────────────────────────────

    pub fn kill_focused(&mut self) {
        let Some(win) = self.focused else { return };
        let wm_protocols = self.intern_atom("WM_PROTOCOLS");
        let wm_delete = self.intern_atom("WM_DELETE_WINDOW");

        if self.supports_protocol(win, wm_protocols, wm_delete) {
            let event = ClientMessageEvent::new(
                32, win, wm_protocols,
                [wm_delete, x11rb::CURRENT_TIME, 0, 0, 0],
            );
            self.conn.send_event(false, win, EventMask::NO_EVENT, event).unwrap();
        } else {
            self.conn.kill_client(win).unwrap();
        }
        self.conn.flush().unwrap();
    }

    // ── Rotation ────────────────────────────────────────────────────────

    pub fn rotate_next(&mut self) {
        let ws = self.current_ws;
        let ws_indices: Vec<usize> = self.clients.iter().enumerate()
            .filter(|(_, c)| c.workspace == ws)
            .map(|(i, _)| i).collect();
        if ws_indices.len() < 2 { return; }

        // rotate: first ws client goes to end
        let first_idx = ws_indices[0];
        let client = self.clients.remove(first_idx);
        // insert after the last ws client (which shifted left by 1)
        let last_idx = ws_indices[ws_indices.len() - 1] - 1;
        self.clients.insert(last_idx + 1, client);
        self.apply_layout();
        self.focus_top();
    }

    pub fn rotate_prev(&mut self) {
        let ws = self.current_ws;
        let ws_indices: Vec<usize> = self.clients.iter().enumerate()
            .filter(|(_, c)| c.workspace == ws)
            .map(|(i, _)| i).collect();
        if ws_indices.len() < 2 { return; }

        let last_idx = ws_indices[ws_indices.len() - 1];
        let client = self.clients.remove(last_idx);
        let first_idx = ws_indices[0];
        self.clients.insert(first_idx, client);
        self.apply_layout();
        self.focus_top();
    }

    // ── Workspaces ──────────────────────────────────────────────────────

    pub fn switch_workspace(&mut self, ws: usize) {
        if ws == self.current_ws || ws >= NUM_WORKSPACES { return; }

        // unmap current workspace windows
        for c in &self.clients {
            if c.workspace == self.current_ws {
                self.conn.unmap_window(c.window).unwrap();
            }
        }

        self.current_ws = ws;

        // map new workspace windows
        for c in &self.clients {
            if c.workspace == self.current_ws {
                self.conn.map_window(c.window).unwrap();
            }
        }

        self.conn.flush().unwrap();
        self.apply_layout();
        self.focus_top();
    }

    pub fn move_to_workspace(&mut self, ws: usize) {
        if ws >= NUM_WORKSPACES { return; }
        let Some(win) = self.focused else { return };

        if let Some(c) = self.clients.iter_mut().find(|c| c.window == win) {
            c.workspace = ws;
        }

        // if moving to a different workspace, unmap
        if ws != self.current_ws {
            self.conn.unmap_window(win).unwrap();
            self.conn.flush().unwrap();
            self.focused = None;
            self.apply_layout();
            self.focus_top();
        } else {
            self.apply_layout();
        }
    }

    // ── Floating ────────────────────────────────────────────────────────

    pub fn toggle_floating(&mut self) {
        let Some(win) = self.focused else { return };
        if let Some(c) = self.clients.iter_mut().find(|c| c.window == win) {
            c.floating = !c.floating;
            if c.floating {
                // give floating window a reasonable centered position
                let sw = self.screen_width as i32;
                let sh = self.screen_height as i32;
                c.w = (sw as u32) / 2;
                c.h = (sh as u32) / 2;
                c.x = sw / 4;
                c.y = sh / 4;
            }
        }
        self.apply_layout();
    }

    /// Move a window by mouse drag (dwm-style inner event loop)
    pub fn drag_move(&mut self, win: Window, start_x: i16, start_y: i16) {
        // mark as floating
        if let Some(c) = self.clients.iter_mut().find(|c| c.window == win) {
            c.floating = true;
        }
        self.apply_layout();

        let geom = self.conn.get_geometry(win).unwrap().reply().unwrap();
        let ox = geom.x as i32;
        let oy = geom.y as i32;

        self.conn.grab_pointer(
            false, self.root,
            EventMask::BUTTON_RELEASE | EventMask::POINTER_MOTION,
            GrabMode::ASYNC, GrabMode::ASYNC,
            0u32, 0u32, x11rb::CURRENT_TIME,
        ).unwrap();
        self.conn.flush().unwrap();

        loop {
            let ev = self.conn.wait_for_event().unwrap();
            match ev {
                Event::MotionNotify(e) => {
                    let nx = ox + (e.root_x - start_x) as i32;
                    let ny = oy + (e.root_y - start_y) as i32;
                    self.conn.configure_window(win, &ConfigureWindowAux::new()
                        .x(nx).y(ny)).unwrap();
                    self.conn.flush().unwrap();
                    // update stored geometry
                    if let Some(c) = self.clients.iter_mut().find(|c| c.window == win) {
                        c.x = nx; c.y = ny;
                    }
                }
                Event::ButtonRelease(_) => break,
                _ => {}
            }
        }

        self.conn.ungrab_pointer(x11rb::CURRENT_TIME).unwrap();
        self.conn.flush().unwrap();
    }

    /// Resize a window by mouse drag (mod+right-click)
    pub fn drag_resize(&mut self, win: Window, start_x: i16, start_y: i16) {
        if let Some(c) = self.clients.iter_mut().find(|c| c.window == win) {
            c.floating = true;
        }
        self.apply_layout();

        let geom = self.conn.get_geometry(win).unwrap().reply().unwrap();
        let ow = geom.width as i32;
        let oh = geom.height as i32;

        self.conn.grab_pointer(
            false, self.root,
            EventMask::BUTTON_RELEASE | EventMask::POINTER_MOTION,
            GrabMode::ASYNC, GrabMode::ASYNC,
            0u32, 0u32, x11rb::CURRENT_TIME,
        ).unwrap();
        self.conn.flush().unwrap();

        loop {
            let ev = self.conn.wait_for_event().unwrap();
            match ev {
                Event::MotionNotify(e) => {
                    let nw = (ow + (e.root_x - start_x) as i32).max(100) as u32;
                    let nh = (oh + (e.root_y - start_y) as i32).max(100) as u32;
                    self.conn.configure_window(win, &ConfigureWindowAux::new()
                        .width(nw).height(nh)).unwrap();
                    self.conn.flush().unwrap();
                    if let Some(c) = self.clients.iter_mut().find(|c| c.window == win) {
                        c.w = nw; c.h = nh;
                    }
                }
                Event::ButtonRelease(_) => break,
                _ => {}
            }
        }

        self.conn.ungrab_pointer(x11rb::CURRENT_TIME).unwrap();
        self.conn.flush().unwrap();
    }

    // ── Bar ─────────────────────────────────────────────────────────────

    pub fn update_bar(&self) {
        let title = self.focused
            .map(|w| self.get_window_title(w))
            .unwrap_or_default();

        let mut occupied = vec![false; NUM_WORKSPACES];
        for c in &self.clients {
            if c.workspace < NUM_WORKSPACES {
                occupied[c.workspace] = true;
            }
        }

        self.bar.draw(&self.conn, self.current_ws, &title, &occupied);
    }

    fn get_window_title(&self, win: Window) -> String {
        // try _NET_WM_NAME (UTF8)
        let net_name = self.intern_atom("_NET_WM_NAME");
        let utf8 = self.intern_atom("UTF8_STRING");
        if let Ok(cookie) = self.conn.get_property(false, win, net_name, utf8, 0, 256) {
            if let Ok(r) = cookie.reply() {
                if r.value_len > 0 {
                    return String::from_utf8_lossy(&r.value).into_owned();
                }
            }
        }
        // fall back to WM_NAME
        if let Ok(cookie) = self.conn.get_property(
            false, win, AtomEnum::WM_NAME, AtomEnum::STRING, 0, 256,
        ) {
            if let Ok(r) = cookie.reply() {
                if r.value_len > 0 {
                    return String::from_utf8_lossy(&r.value).into_owned();
                }
            }
        }
        String::new()
    }

    // ── Spawn ───────────────────────────────────────────────────────────

    pub fn spawn(&self, cmd: &str) {
        use std::process::Command;
        let _ = Command::new(cmd).spawn();
    }

    // ── Helpers ─────────────────────────────────────────────────────────

    pub fn intern_atom(&self, name: &str) -> Atom {
        self.conn.intern_atom(false, name.as_bytes())
            .unwrap().reply().unwrap().atom
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
}
