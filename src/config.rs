// ── User configuration ──────────────────────────────────────────────────────
// Configs i am currently using you can create your own const values and add  configs to them to
// customise them as you please 
pub const MODKEY: u16       = 1 << 6;  // Mod4 (Super)
pub const TERMINAL: &str    = "alacritty"; // add whatever terminal you like here i use alacritty  
pub const LAUNCHER: &str    = "dmenu_run"; // app launcher (Super+d) probably going to change to
                                           // rmenu when i get a job and start tsoding lol 
pub const BROWSER: &str     = "librewolf"; //web browser (Super+w)
pub const WALLPAPER: &str   = "/home/reitard/Projects/mp/rwm/miku.png"; //change this to the path
                                                                        //of your wallpaperimage
pub const EDITOR: &str      = "emacs";
pub const GAP: u32          = 5; //Gap between windows
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
