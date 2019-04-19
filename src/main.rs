extern crate tcod;

use tcod::console::*;
use tcod::colors;
use tcod::input::Key;
use tcod::input::KeyCode::*;

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;
const LIMIT_FPS: i32 = 20;

fn main() {
    println!("Hello, world!");

    let mut root = Root::initializer()
    .font("arial10x10.png", FontLayout::Tcod)
    .font_type(FontType::Greyscale)
    .size(SCREEN_WIDTH, SCREEN_HEIGHT)
    .title("Rust/libtcod tutorial")
    .init();

    tcod::system::set_fps(LIMIT_FPS);

    let mut player_x = SCREEN_WIDTH / 2;
    let mut player_y = SCREEN_HEIGHT / 2;

    while !root.window_closed() {
        root.set_default_foreground(colors::WHITE);
        root.put_char(player_x, player_y, '@', BackgroundFlag::None);
        root.flush();
        root.put_char(player_x, player_y, '0', BackgroundFlag::None);

        // Handle keys and exit if needed
        let exit = handle_keys(&mut root, &mut player_x, &mut player_y);
        if exit {
            break;
        }
    }

}

/// Handle a key press event
///
/// # Return value
///
/// A value of true means that the caller should exit.
fn handle_keys(root: &mut Root, player_x: &mut i32, player_y: &mut i32) -> bool {
    // TODO
    let key = root.wait_for_keypress(true);

    match key {
        // Player movement
        Key { code: Up, .. } => *player_y -= 1,
        Key { code: Down, .. } => *player_y += 1,
        Key { code: Left, .. } => *player_x -= 1,
        Key { code: Right, .. } => *player_x += 1,

        // Alt-enter: toggle fullscreen
        Key { code: Enter, alt: true, .. } => {
            root.set_fullscreen(!root.is_fullscreen());
        },

        // Exit the game
        Key { code: Escape, .. } => return true,

        // Ignore other keys
        _ => {},
    }

    false
}
