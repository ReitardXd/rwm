# rwm

A minimal, dwm-inspired tiling window manager written in Rust using `x11rb`.


## Features

- **Master-stack tiling** — first window gets left half, rest stack on the right
- **Focus follows mouse** — EnterNotify driven, no click-to-focus
- **Focus cycling** — `Super+j/k` cycles focus without rearranging windows
- **Kill focused window** — graceful `WM_DELETE_WINDOW`, falls back to `XKillClient`
- **Gaps** — configurable pixel gaps between windows and screen edges
- **9 workspaces** — switch with `Super+1..9`, move windows with `Super+Shift+1..9`, or click the bar
- **Clickable status bar** — workspace indicators + focused window title
- **Fullscreen toggle** — `Super+f`, covers entire screen including bar
- **App launchers** — dmenu, browser, file manager, htop, ncmpcpp, etc.
- **Media keys** — volume, brightness, mpc playback, mute, screenshots
- **Last workspace** — `Super+Tab` toggles between current and previous workspace

## Keybindings

### Core

| Binding | Action |
|---------|--------|
| `Super + Return` | Spawn terminal (alacritty) |
| `Super + d` | dmenu_run |
| `Super + q` | Kill focused window |
| `Super + Shift + q` | Quit rwm |
| `Super + j` | Focus next window |
| `Super + k` | Focus previous window |
| `Super + f` | Toggle fullscreen |
| `Super + Tab` | Switch to last workspace |
| `Super + Backspace` | sysact |
| `Super + 1..9` | Switch to workspace 1–9 |
| `Super + Shift + 1..9` | Move focused window to workspace 1–9 |
| Click bar number | Switch to that workspace |

### App Launchers

| Binding | Action |
|---------|--------|
| `Super + w` | librewolf |
| `Super + Shift + w` | nmtui (network manager) |
| `Super + r` | lfub (file manager) |
| `Super + Shift + r` | htop |
| `Super + n` | nvim VimwikiIndex |
| `Super + m` | ncmpcpp |
| `Super + p` | mpc toggle |
| `Super + Shift + p` | mpc pause + pauseallmpv |

### Volume & Media

| Binding | Action |
|---------|--------|
| `Super + -` | Volume -5% |
| `Super + Shift + -` | Volume -15% |
| `Super + =` | Volume +5% |
| `Super + Shift + =` | Volume +15% |
| `Super + Shift + m` | Mute toggle |
| `XF86 AudioMute` | Mute toggle |
| `XF86 AudioLower/Raise` | Volume ±3% |
| `XF86 AudioPlay/Prev/Next/Stop` | mpc controls |
| `XF86 BrightnessUp/Down` | xbacklight ±15 |
| `XF86 AudioMicMute` | Mic mute toggle |

### Screenshot

| Binding | Action |
|---------|--------|
| `Print` | Full screenshot (maim) |
| `Shift + Print` | maimpick (selection) |

## Build

```bash
cargo build --release
```

## Usage

### Xephyr (test inside existing session)

```bash
Xephyr :1 -screen 1280x720 &
DISPLAY=:1 ./target/release/rwm
```

### As your actual WM (LightDM)

Install the session file:

```bash
sudo cp rwm.desktop /usr/share/xsessions/rwm.desktop
```

Then select **rwm** from LightDM's session dropdown at login.

### xinitrc

Add to `~/.xinitrc`:

```bash
exec /path/to/rwm
```

Then `startx` from a TTY.

## Configuration

Edit constants in `src/config.rs`:

```rust
pub const TERMINAL: &str    = "alacritty";
pub const LAUNCHER: &str    = "dmenu_run";
pub const BROWSER: &str     = "librewolf";
pub const GAP: u32          = 8;
pub const BAR_HEIGHT: u16   = 20;
pub const NUM_WORKSPACES: usize = 9;
pub const BAR_BG: u32      = 0x222222;
pub const BAR_FG: u32      = 0xbbbbbb;
pub const BAR_SEL_BG: u32  = 0x005577;
```

## Project Structure

```
src/
├── main.rs      entry point, event loop, keybinding dispatch
├── config.rs    all user-tunable constants (like dwm's config.h)
├── keys.rs      keysym constants, XF86 media keys, keycode translation
├── client.rs    per-window state: geometry, fullscreen, workspace
├── layout.rs    pure-geometry tiling math (no X11 calls)
├── bar.rs       status bar: clickable workspace indicators + window title
└── wm.rs        core WM: manage, focus, kill, workspaces, fullscreen
```

## Example Screenshot
<img width="1920" height="1080" alt="pic-full-260605-2237-10" src="https://github.com/user-attachments/assets/a18c2524-6155-407c-8b80-b92751f7b65b" />


## Dependencies

- Rust 2024 edition
- [`x11rb`](https://crates.io/crates/x11rb) 0.13 — pure Rust X11 bindings 

## License

This project is licensed under the GNU General Public License v3.0 — see [LICENSE](LICENSE) for details.
