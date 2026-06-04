use x11rb::connection::Connection;
use x11rb::rust_connection::RustConnection;
use x11rb::protocol::xproto::*;
use crate::config::*;

/// Simple X11 core-font status bar (drawing context struct)
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

    /// Redraw the bar: workspace indicators + window title
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
        conn.poly_fill_rectangle(self.window, self.gc_norm, &[
            Rectangle { x: 0, y: 0, width: self.width, height: h },
        ]).unwrap();

        let mut x: i16 = 0;
        let cell = (h as i16) + 4; //width per workspace 

        // workspace indicators
        for i in 0..NUM_WORKSPACES {
            let label = format!(" {} ", i + 1);
            if i == current_ws {
                // selected: draw highlight bg then text
                conn.poly_fill_rectangle(self.window, self.gc_sel, &[
                    Rectangle { x, y: 0, width: cell as u16, height: h },
                ]).unwrap();
                conn.image_text8(self.window, self.gc_sel, x + 2, y, label.as_bytes()).unwrap();
            } else if occupied[i] {
                // has windows but not selected
                conn.image_text8(self.window, self.gc_norm, x + 2, y, label.as_bytes()).unwrap();
            } else {
                // empty workspace, dimmer
                conn.image_text8(self.window, self.gc_norm, x + 2, y, label.as_bytes()).unwrap();
            }
            x += cell;
        }

        // separator
        x += 8;

        // window title (truncate to fit)
        let max_chars = ((self.width as i16 - x) / 7).max(0) as usize; // ~7px per char in fixed
        let title_trunc: String = title.chars().take(max_chars).collect();
        if !title_trunc.is_empty() {
            conn.image_text8(
                self.window, self.gc_norm, x, y,
                title_trunc.as_bytes(),
            ).unwrap();
        }

        conn.flush().unwrap();
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
