# rwm

A minimal, dwm-inspired tiling window manager written in Rust using `x11rb`.

No bloat. No config parsers. Just a clean master-stack layout with sane defaults.

## Features

- **Master-stack tiling** — first window gets left half, rest stack on the right
- **Focus follows mouse** — EnterNotify driven, no click-to-focus
- **Kill focused window** — graceful `WM_DELETE_WINDOW`, falls back to `XKillClient`
- **Launch terminal** — one keybinding, configurable command
- **Focus cycling** — cycle focus between windows without rearranging layout
- **Gaps** — configurable pixel gaps between windows and screen edges
- **9 workspaces** — switch with Mod4+1..9, move windows with Mod4+Shift+1..9, or click the bar
- **Status bar** — X11 core font bar showing clickable workspace indicators + window title
- **Fullscreen toggle** — Mod4+f to toggle, covers entire screen including bar
- **Quit cleanly** — `Mod4+Shift+q`

## Keybindings

| Binding | Action |
|---------|--------|
| `Mod4 + Return` | Spawn terminal |
| `Mod4 + Shift + c` | Kill focused window |
| `Mod4 + j` | Focus next window |
| `Mod4 + k` | Focus previous window |
| `Mod4 + f` | Toggle fullscreen for focused window |
| `Mod4 + 1..9` | Switch to workspace 1–9 |
| `Mod4 + Shift + 1..9` | Move focused window to workspace 1–9 |
| Click bar workspace | Switch to that workspace |
| `Mod4 + Shift + q` | Quit rwm |

`Mod4` is the Super/Windows key.

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

### As your actual WM

Add to `~/.xinitrc`:

```bash
exec /path/to/rwm
```

Then `startx` from a TTY.

## Configuration

Edit constants in `src/config.rs`:

```rust
pub const TERMINAL: &str    = "alacritty";
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
├── main.rs      entry point + event loop
├── config.rs    all user-tunable constants (like dwm's config.h)
├── keys.rs      keysym constants, keycode translation, grab helpers
├── client.rs    per-window state: geometry, fullscreen, workspace
├── layout.rs    pure-geometry tiling (no X11 calls)
├── bar.rs       status bar: window, font, GC, draw, clickable workspaces
└── wm.rs        core WM: manage, focus, kill, workspaces, fullscreen
```

## Dependencies

- Rust 2024 edition
- [`x11rb`](https://crates.io/crates/x11rb) 0.13 — pure Rust X11 bindings (no C deps)

## License

This project is licensed under the GNU General Public License v3.0 — see [LICENSE](LICENSE) for details.
