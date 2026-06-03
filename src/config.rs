// ── User configuration ──────────────────────────────────────────────────────
// All tunable knobs in one place, like dwm's config.h

pub const MODKEY: u16       = 1 << 6;  // Mod4 (Super)
pub const TERMINAL: &str    = "alacritty"; 
pub const GAP: u32          = 8;
pub const BAR_HEIGHT: u16   = 20;
pub const NUM_WORKSPACES: usize = 9;

// colors
pub const BORDER_FOCUSED: u32   = 0xbbbbbb;
pub const BORDER_UNFOCUSED: u32 = 0x444444;
pub const BAR_BG: u32      = 0x222222;
pub const BAR_FG: u32      = 0xbbbbbb;
pub const BAR_SEL_BG: u32  = 0x005577;
pub const BAR_SEL_FG: u32  = 0xeeeeee;
pub const FONT_NAME: &str  = "fixed";
