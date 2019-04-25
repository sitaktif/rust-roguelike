extern crate tcod;

use tcod::console::*;
use tcod::colors::{self,Color};
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

    let mut con = Offscreen::new(SCREEN_WIDTH, SCREEN_HEIGHT);

    tcod::system::set_fps(LIMIT_FPS);

    let player = Object::new(SCREEN_WIDTH / 2, SCREEN_HEIGHT / 2, '@', colors::WHITE);
    let npc = Object::new(player.x - 1, player.y -3, '@', colors::YELLOW);

    let mut objects = [player, npc];

    while !root.window_closed() {
        con.set_default_foreground(colors::WHITE);
        for obj in objects.iter() {
            obj.draw(&mut con);
        }
        blit(&mut con, (0, 0), (SCREEN_WIDTH, SCREEN_HEIGHT), &mut root, (0, 0), 1.0, 1.0);

        root.flush();
        for obj in objects.iter() {
            obj.clear(&mut con);
        }

        // Handle keys and exit if needed
        let player = &mut objects[0];
        let exit = handle_keys(&mut root, player);
        if exit {
            break;
        }
    }

}

struct Object {
    x: i32,
    y: i32,
    char: char,
    color: Color,
}

impl Object {
    pub fn new(x: i32, y: i32, char: char, color: Color) -> Self {
        Object {
            x, y, char, color,
        }
    }
    /// Move object by the given amount
    pub fn move_by(&mut self, dx: i32, dy: i32) {
        self.x += dx;
        self.y += dy;
    }
    pub fn draw(&self, con: &mut Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }
    pub fn clear(&self, con: &mut Console) {
        con.put_char(self.x, self.y, ' ', BackgroundFlag::None);
    }
}

/// Handle a key press event
///
/// # Return value
///
/// A value of true means that the caller should exit.
fn handle_keys(root: &mut Root, player: &mut Object) -> bool {
    // TODO
    let key = root.wait_for_keypress(true);

    match key {
        // Player movement
        Key { code: Up, .. } | Key { printable: 'k', .. } => player.move_by(0, -1),
        Key { code: Down, .. } | Key { printable: 'j', .. }  => player.move_by(0, 1),
        Key { code: Left, .. } | Key { printable: 'h', .. }  => player.move_by(-1, 0),
        Key { code: Right, .. } | Key { printable: 'l', .. }  => player.move_by(1, 0),

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
