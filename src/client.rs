use x11rb::protocol::xproto::Window;

/// Per-window state
pub struct Client {
    pub window: Window,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub fullscreen: bool,
    pub workspace: usize,
    // saved geometry before fullscreen
    pub pre_fs: Option<(i32, i32, u32, u32)>,
}

impl Client {
    pub fn new(window: Window, workspace: usize) -> Self {
        Self {
            window, x: 0, y: 0, w: 640, h: 480,
            fullscreen: false, workspace, pre_fs: None,
        }
    }
}
