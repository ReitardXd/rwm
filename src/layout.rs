use crate::client::Client;
use crate::config::*;

/// Compute tiled geometry for clients. Pure math — no X11 calls.
/// Skips fullscreen clients (they get handled separately).
pub fn tile(clients: &mut [&mut Client], screen_w: u32, screen_h: u32) {
    let tiled: Vec<usize> = clients.iter().enumerate()
        .filter(|(_, c)| !c.fullscreen)
        .map(|(i, _)| i)
        .collect();

    let n = tiled.len();
    if n == 0 { return; }

    let g = GAP;
    let bar = BAR_HEIGHT as u32;
    let sy = bar as i32 + g as i32;
    let sh = screen_h.saturating_sub(bar + 2 * g);

    if n == 1 {
        let idx = tiled[0];
        clients[idx].x = g as i32;
        clients[idx].y = sy;
        clients[idx].w = screen_w.saturating_sub(2 * g);
        clients[idx].h = sh;
        return;
    }

    // master: left half
    let master_w = screen_w / 2 - g - g / 2;
    let idx = tiled[0];
    clients[idx].x = g as i32;
    clients[idx].y = sy;
    clients[idx].w = master_w;
    clients[idx].h = sh;

    // stack: right half
    let stack_x = (screen_w / 2 + g / 2) as i32;
    let stack_w = screen_w - screen_w / 2 - g - g / 2;
    let stack_count = (n - 1) as u32;
    let total_h = sh.saturating_sub((stack_count - 1) * g);
    let each_h = total_h / stack_count;

    for (si, &idx) in tiled[1..].iter().enumerate() {
        let si = si as u32;
        let y = sy + (si * (each_h + g)) as i32;
        let h = if si == stack_count - 1 {
            (sy as u32 + sh) - y as u32
        } else {
            each_h
        };
        clients[idx].x = stack_x;
        clients[idx].y = y;
        clients[idx].w = stack_w;
        clients[idx].h = h;
    }
}
