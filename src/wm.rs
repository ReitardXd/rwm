use x11rb::connection::Connection;
use x11rb::rust_connection::RustConnection;
use x11rb::protocol::xproto::*;

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
    pub mfact: f32,
    pub bar: StatusBar,
    /// Windows we've unmapped ourselves (workspace switch).
    /// UnmapNotify for these should be ignored.
    pending_unmaps: Vec<Window>,
}

impl WM {
    pub fn new(
        conn: RustConnection, root: Window,
        screen_width: u16, screen_height: u16,
        bar: StatusBar,
    ) -> Self {
        Self {
            conn, root, screen_width, screen_height,
            clients: vec![], focused: None, current_ws: 0,
            mfact: 0.55, bar, pending_unmaps: vec![],
        }
    }

    // ── Layout ──────────────────────────────────────────────────────────

    pub fn apply_layout(&mut self) {
        let sw = self.screen_width as u32;
        let sh = self.screen_height as u32;

        let mut ws_clients: Vec<&mut Client> = self.clients.iter_mut()
            .filter(|c| c.workspace == self.current_ws)
            .collect();

        layout::tile(&mut ws_clients, sw, sh, self.mfact);

        for c in &ws_clients {
            if c.fullscreen {
                let _ = self.conn.configure_window(c.window, &ConfigureWindowAux::new()
                    .x(0).y(0).width(sw).height(sh)
                    .border_width(0u32)
                    .stack_mode(StackMode::ABOVE)
                );
            } else {
                let _ = self.conn.configure_window(c.window, &ConfigureWindowAux::new()
                    .x(c.x).y(c.y).width(c.w).height(c.h)
                    .border_width(0u32)
                );
            }
        }
        let _ = self.conn.flush();
        self.update_bar();
    }

    // ── Focus ───────────────────────────────────────────────────────────

    pub fn focus(&mut self, win: Window) {
        if let Some(prev) = self.focused {
            let _ = self.conn.change_window_attributes(
                prev, &ChangeWindowAttributesAux::new().border_pixel(BORDER_UNFOCUSED),
            );
        }
        let _ = self.conn.set_input_focus(InputFocus::POINTER_ROOT, win, x11rb::CURRENT_TIME);
        let _ = self.conn.change_window_attributes(
            win, &ChangeWindowAttributesAux::new().border_pixel(BORDER_FOCUSED),
        );
        self.focused = Some(win);
        let _ = self.conn.flush();
        self.update_bar();
    }

    pub fn focus_top(&mut self) {
        let top = self.clients.iter()
            .find(|c| c.workspace == self.current_ws)
            .map(|c| c.window);
        if let Some(w) = top {
            self.focus(w);
        } else {
            self.focused = None;
            let _ = self.conn.set_input_focus(InputFocus::POINTER_ROOT, self.root, x11rb::CURRENT_TIME);
            let _ = self.conn.flush();
            self.update_bar();
        }
    }

    /// Focus the next window on the current workspace (cycle forward)
    pub fn focus_next(&mut self) {
        let ws_windows: Vec<Window> = self.clients.iter()
            .filter(|c| c.workspace == self.current_ws)
            .map(|c| c.window).collect();
        if ws_windows.is_empty() { return; }

        let current_idx = self.focused
            .and_then(|f| ws_windows.iter().position(|&w| w == f))
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % ws_windows.len();
        self.focus(ws_windows[next_idx]);
    }

    /// Focus the previous window on the current workspace (cycle backward)
    pub fn focus_prev(&mut self) {
        let ws_windows: Vec<Window> = self.clients.iter()
            .filter(|c| c.workspace == self.current_ws)
            .map(|c| c.window).collect();
        if ws_windows.is_empty() { return; }

        let current_idx = self.focused
            .and_then(|f| ws_windows.iter().position(|&w| w == f))
            .unwrap_or(0);
        let prev_idx = if current_idx == 0 { ws_windows.len() - 1 } else { current_idx - 1 };
        self.focus(ws_windows[prev_idx]);
    }

    // ── Manage / Unmanage ───────────────────────────────────────────────

    pub fn manage(&mut self, win: Window) {
        if win == self.bar.window { return; }
        if self.clients.iter().any(|c| c.window == win) { return; }

        // Only subscribe to ENTER_WINDOW and PROPERTY_CHANGE.
        // Do NOT subscribe to STRUCTURE_NOTIFY here — we get
        // UnmapNotify/DestroyNotify from root's SUBSTRUCTURE_NOTIFY.
        // Subscribing here too causes double events that break workspace switching.
        let _ = self.conn.change_window_attributes(win,
            &ChangeWindowAttributesAux::new()
                .event_mask(EventMask::ENTER_WINDOW | EventMask::PROPERTY_CHANGE),
        );

        let client = Client::new(win, self.current_ws);
        self.clients.push(client);
        let _ = self.conn.map_window(win);
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

    /// Returns true if this UnmapNotify was caused by us and should be ignored.
    pub fn consume_pending_unmap(&mut self, win: Window) -> bool {
        if let Some(pos) = self.pending_unmaps.iter().position(|&w| w == win) {
            self.pending_unmaps.remove(pos);
            true
        } else {
            false
        }
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
            let _ = self.conn.send_event(false, win, EventMask::NO_EVENT, event);
        } else {
            let _ = self.conn.kill_client(win);
        }
        let _ = self.conn.flush();
    }

    // ── Window swapping ─────────────────────────────────────────────────

    /// Swap the focused window with the next one in the stack
    pub fn swap_next(&mut self) {
        let ws_indices: Vec<usize> = self.clients.iter().enumerate()
            .filter(|(_, c)| c.workspace == self.current_ws)
            .map(|(i, _)| i).collect();
        if ws_indices.len() < 2 { return; }

        let Some(focused_win) = self.focused else { return };
        let Some(pos) = ws_indices.iter().position(|&i| self.clients[i].window == focused_win) else { return };
        let next_pos = (pos + 1) % ws_indices.len();

        let a = ws_indices[pos];
        let b = ws_indices[next_pos];
        self.clients.swap(a, b);
        self.apply_layout();
        self.focus(focused_win);
    }

    /// Swap the focused window with the previous one in the stack
    pub fn swap_prev(&mut self) {
        let ws_indices: Vec<usize> = self.clients.iter().enumerate()
            .filter(|(_, c)| c.workspace == self.current_ws)
            .map(|(i, _)| i).collect();
        if ws_indices.len() < 2 { return; }

        let Some(focused_win) = self.focused else { return };
        let Some(pos) = ws_indices.iter().position(|&i| self.clients[i].window == focused_win) else { return };
        let prev_pos = if pos == 0 { ws_indices.len() - 1 } else { pos - 1 };

        let a = ws_indices[pos];
        let b = ws_indices[prev_pos];
        self.clients.swap(a, b);
        self.apply_layout();
        self.focus(focused_win);
    }

    // ── Workspaces ──────────────────────────────────────────────────────

    pub fn switch_workspace(&mut self, ws: usize) {
        if ws == self.current_ws || ws >= NUM_WORKSPACES { return; }

        // unmap current workspace windows, track as pending
        for c in &self.clients {
            if c.workspace == self.current_ws {
                self.pending_unmaps.push(c.window);
                let _ = self.conn.unmap_window(c.window);
            }
        }

        self.current_ws = ws;

        // map new workspace windows
        for c in &self.clients {
            if c.workspace == self.current_ws {
                let _ = self.conn.map_window(c.window);
            }
        }

        let _ = self.conn.flush();
        self.apply_layout();
        self.focus_top();
    }

    pub fn move_to_workspace(&mut self, ws: usize) {
        if ws >= NUM_WORKSPACES { return; }
        let Some(win) = self.focused else { return };

        if let Some(c) = self.clients.iter_mut().find(|c| c.window == win) {
            c.workspace = ws;
        }

        if ws != self.current_ws {
            self.pending_unmaps.push(win);
            let _ = self.conn.unmap_window(win);
            let _ = self.conn.flush();
            self.focused = None;
            self.apply_layout();
            self.focus_top();
        } else {
            self.apply_layout();
        }
    }

    // ── Fullscreen ──────────────────────────────────────────────────────

    pub fn toggle_fullscreen(&mut self) {
        let Some(win) = self.focused else { return };

        let is_now_fullscreen = {
            let Some(c) = self.clients.iter_mut().find(|c| c.window == win) else { return };
            if c.fullscreen {
                if let Some((x, y, w, h)) = c.pre_fs.take() {
                    c.x = x; c.y = y; c.w = w; c.h = h;
                }
                c.fullscreen = false;
                false
            } else {
                c.pre_fs = Some((c.x, c.y, c.w, c.h));
                c.fullscreen = true;
                true
            }
        };

        self.apply_layout();

        if is_now_fullscreen {
            let _ = self.conn.configure_window(win, &ConfigureWindowAux::new()
                .stack_mode(StackMode::ABOVE)
            );
            let _ = self.conn.flush();
        }
    }

    // ── Master factor resize ────────────────────────────────────────

    pub fn adjust_mfact(&mut self, delta: f32) {
        self.mfact = (self.mfact + delta).clamp(0.05, 0.95);
        self.apply_layout();
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
        let net_name = self.intern_atom("_NET_WM_NAME");
        let utf8 = self.intern_atom("UTF8_STRING");
        if let Ok(cookie) = self.conn.get_property(false, win, net_name, utf8, 0, 256) {
            if let Ok(r) = cookie.reply() {
                if r.value_len > 0 {
                    return String::from_utf8_lossy(&r.value).into_owned();
                }
            }
        }
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

    /// Spawn a command with arguments (like dwm's {.v = (const char*[]){...}})
    pub fn spawn_args(&self, cmd: &str, args: &[&str]) {
        use std::process::Command;
        let _ = Command::new(cmd).args(args).spawn();
    }

    /// Run a shell command string (like dwm's SHCMD macro)
    pub fn spawn_sh(&self, cmd: &str) {
        use std::process::Command;
        let _ = Command::new("sh").args(["-c", cmd]).spawn();
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
