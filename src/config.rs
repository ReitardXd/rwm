// ── User configuration ──────────────────────────────────────────────────────
// Configs i am going to use you can edit it if you want just know the hex value for the input
pub const MODKEY: u16       = 1 << 6;  // Mod4 (Super)
pub const TERMINAL: &str    = "alacritty"; // add whatever terminal you like here i use alacritty  
pub const GAP: u32          = 8; //Gap between windows
pub const BAR_HEIGHT: u16   = 20; 
pub const NUM_WORKSPACES: usize = 9;

// colors add the hexvalue to customise 
pub const BORDER_FOCUSED: u32   = 0xbbbbbb;
pub const BORDER_UNFOCUSED: u32 = 0x444444;
pub const BAR_BG: u32      = 0x222222;
pub const BAR_FG: u32      = 0xbbbbbb;
pub const BAR_SEL_BG: u32  = 0x005577;
pub const BAR_SEL_FG: u32  = 0xeeeeee;
pub const FONT_NAME: &str  = "fixed";
