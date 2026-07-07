use x11rb::connection::Connection;
use x11rb::rust_connection::RustConnection;
use x11rb::protocol::xproto::*;
use crate::config::*;

/// Simple X11 core-font status bar (drawing context struct)
// Allows you to view what is the newest activity in the current workspace
pub struct StatusBar {
    pub window: Window,
    gc_norm: Gcontext,
    gc_sel: Gcontext,
    width: u16,
    font_ascent: i16,
}

impl StatusBar {
    pub fn new(
        conn: &RustConnection,
        root: Window,
        screen_width: u16,
        depth: u8,
        visual: Visualid,
    ) -> Self {
        // open font
        let font = conn.generate_id().unwrap();
        conn.open_font(font, FONT_NAME.as_bytes()).unwrap();
        // query font metrics for text baseline
        let font_info = conn.query_font(font).unwrap().reply().unwrap();
        let font_ascent = font_info.font_ascent as i16;

        // create bar window (override_redirect so Window manager doesnt manage it)
        let win = conn.generate_id().unwrap();
        conn.create_window(
            depth, win, root,
            0, 0, screen_width, BAR_HEIGHT, 0,
            WindowClass::INPUT_OUTPUT, visual,
            &CreateWindowAux::new()
                .background_pixel(BAR_BG)
                .override_redirect(1u32)
                .event_mask(EventMask::EXPOSURE | EventMask::BUTTON_PRESS),
        ).unwrap();
        // GC for normal text (fg on bg)
        let gc_norm = conn.generate_id().unwrap();
        conn.create_gc(gc_norm, win, &CreateGCAux::new()
            .foreground(BAR_FG).background(BAR_BG).font(font)
        ).unwrap();
        // GC for selected workspace (sel_fg on sel_bg)
        let gc_sel = conn.generate_id().unwrap();
        conn.create_gc(gc_sel, win, &CreateGCAux::new()
            .foreground(BAR_SEL_FG).background(BAR_SEL_BG).font(font)
        ).unwrap();

        conn.close_font(font).unwrap();
        conn.map_window(win).unwrap();
        conn.flush().unwrap();

        Self { window: win, gc_norm, gc_sel, width: screen_width, font_ascent }
    }

    /// Redraw the bar: workspace indicators + window title + clock
    pub fn draw(
        &self,
        conn: &RustConnection,
        current_ws: usize,
        title: &str,
        occupied: &[bool],
    ) {
        let h = BAR_HEIGHT;
        let y = self.font_ascent + (h as i16 - self.font_ascent) / 2;

        // clears the entire bar
        let _ = conn.poly_fill_rectangle(self.window, self.gc_norm, &[
            Rectangle { x: 0, y: 0, width: self.width, height: h },
        ]);

        let mut x: i16 = 0;
        let cell = (h as i16) + 4; //width per workspace

        // workspace numbering
        for i in 0..NUM_WORKSPACES {
            let label = format!(" {} ", i + 1);
            if i == current_ws {
                // selected: draw highlight bg then text
                let _ = conn.poly_fill_rectangle(self.window, self.gc_sel, &[
                    Rectangle { x, y: 0, width: cell as u16, height: h },
                ]);
                let _ = conn.image_text8(self.window, self.gc_sel, x + 2, y, label.as_bytes());
            } else if occupied[i] {
                // has windows but not selected
                let _ = conn.image_text8(self.window, self.gc_norm, x + 2, y, label.as_bytes());
            } else {
                // empty workspace, dimmer
                let _ = conn.image_text8(self.window, self.gc_norm, x + 2, y, label.as_bytes());
            }
            x += cell;
        }

        // separator
        x += 8;

        // ── Clock on the right ──────────────────────────────────────
        let time_str = Self::current_time();
        let time_bytes = time_str.as_bytes();
        let time_width = (time_bytes.len() as i16) * 7; // ~7px per char in fixed font
        let time_x = self.width as i16 - time_width - 8; // 8px padding from right edge
        let _ = conn.image_text8(self.window, self.gc_norm, time_x, y, time_bytes);

        // ── Window title (truncate to fit between workspaces and clock) ──
        let available = (time_x - x - 16).max(0) as usize; // 16px gap before clock
        let max_chars = available / 7;
        let title_trunc: String = title.chars().take(max_chars).collect();
        if !title_trunc.is_empty() {
            let _ = conn.image_text8(
                self.window, self.gc_norm, x, y,
                title_trunc.as_bytes(),
            );
        }

        let _ = conn.flush();
    }

    /// Get current time as "HH:MM  Day DD Mon" string
    fn current_time() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};

        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // apply IST offset (+5:30 = 19800 seconds)
        let local_secs = secs + 19800;

        let days_since_epoch = local_secs / 86400;
        let time_of_day = local_secs % 86400;
        let hours = time_of_day / 3600;
        let minutes = (time_of_day % 3600) / 60;

        // day of week (epoch was Thursday = 4)
        let dow = ((days_since_epoch + 4) % 7) as usize;
        let dow_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

        // date from days since epoch
        let (year, month, day) = days_to_ymd(days_since_epoch);
        let mon_names = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];
        let _ = year; // we don't show year

        format!("{:02}:{:02}  {} {:02} {}",
            hours, minutes,
            dow_names[dow], day, mon_names[(month - 1) as usize])
    }

    /// Map a click x-coordinate on the bar to a workspace index (if any)
    pub fn ws_from_click(&self, click_x: i16) -> Option<usize> {
        let cell = BAR_HEIGHT as i16 + 4;
        let total = cell * NUM_WORKSPACES as i16;
        if click_x < 0 || click_x >= total {
            return None;
        }
        Some((click_x / cell) as usize)
    }
}

/// Convert days since Unix epoch to (year, month, day)
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
