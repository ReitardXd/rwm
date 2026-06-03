use x11rb::protocol::xproto::Window;

/// Per-window state, like dwm's Client struct
pub struct Client {
    pub window: Window,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub floating: bool,
    pub workspace: usize,
}

impl Client {
    pub fn new(window: Window, workspace: usize) -> Self {
        Self {
            window, x: 0, y: 0, w: 640, h: 480,
            floating: false, workspace,
        }
    }
}
