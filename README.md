# rwm

A minimal, dwm-inspired tiling window manager written in Rust using `x11rb`.

No bloat. No config parsers. Just a clean master-stack layout with sane defaults.

## Features

- **Master-stack tiling** — first window gets left half, rest stack on the right
- **Focus follows mouse** — EnterNotify driven, no click-to-focus
- **Kill focused window** — graceful `WM_DELETE_WINDOW`, falls back to `XKillClient`
- **Launch terminal** — one keybinding, configurable command
- **Rotate stack** — cycle any window into master position
- **Quit cleanly** — `Mod4+Shift+q`

## Keybindings

| Binding | Action |
|---------|--------|
| `Mod4 + Return` | Spawn terminal |
| `Mod4 + Shift + c` | Kill focused window |
| `Mod4 + j` | Rotate forward (next window → master) |
| `Mod4 + k` | Rotate backward (last window → master) |
| `Mod4 + Shift + q` | Quit rwm |

`Mod4` is the Super/Windows key.

## Build

```bash
cargo build --release
```

Binary lands at `target/release/rwm` (~719K).

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

Edit the constants at the top of `src/main.rs`:

```rust
const TERMINAL: &str = "st";  // "alacritty", "kitty", "xterm", etc.
```

Modifier key, keybindings, and layout are all hardcoded in the same file — easy to find, easy to change.

## Dependencies

- Rust 2024 edition
- [`x11rb`](https://crates.io/crates/x11rb) 0.13 — pure Rust X11 bindings (no C deps)

## Roadmap

- [x] Mouse-follows-focus
- [x] Kill focused window
- [x] Launch terminal
- [x] Rotate/cycle master
- [ ] Gaps between windows
- [ ] Multiple workspaces
- [ ] Status bar
- [ ] Floating window support
- [ ] Fullscreen toggle
- [ ] Config struct

## License

This project is licensed under the GNU General Public License v3.0 — see [LICENSE](LICENSE) for details.
