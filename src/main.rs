extern crate tcod;

use tcod::console::*;
use tcod::colors::{self,Color};
use tcod::input::Key;
use tcod::input::KeyCode::*;

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;
const LIMIT_FPS: i32 = 20;

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;

const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOR_DARK_GROUND: Color = Color { r: 50, g: 50, b: 150 };

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
    pub fn move_by(&mut self, dx: i32, dy: i32, map: &Map) {
        if map[(self.x + dx) as usize][(self.y + dy) as usize].traversable {
            self.x += dx;
            self.y += dy;
        }
    }
    pub fn draw(&self, con: &mut Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }
    pub fn clear(&self, con: &mut Console) {
        con.put_char(self.x, self.y, ' ', BackgroundFlag::None);
    }
}

#[derive(Clone, Copy, Debug)]
struct Tile {
    traversable: bool,
    transparent: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile { traversable: true, transparent: true }
    }

    pub fn wall() -> Self {
        Tile { traversable: false, transparent: false }
    }
}

type Map = Vec<Vec<Tile>>;

#[derive(Clone, Copy, Debug)]
struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect { x1: x, y1: y, x2: x + w, y2: y + h }
    }
}

fn main() {
    println!("Hello, world!");

    let mut root = Root::initializer()
    .font("arial10x10.png", FontLayout::Tcod)
    .font_type(FontType::Greyscale)
    .size(SCREEN_WIDTH, SCREEN_HEIGHT)
    .title("Rust/libtcod tutorial")
    .init();

    let mut con = Offscreen::new(MAP_WIDTH, MAP_HEIGHT);

    tcod::system::set_fps(LIMIT_FPS);

    let player = Object::new(25, 23, '@', colors::WHITE);
    let npc = Object::new(player.x - 1, player.y -3, '@', colors::YELLOW);

    let mut objects = [player, npc];
    let map = make_map();

    while !root.window_closed() {
        con.set_default_foreground(colors::WHITE);
        render_all(&mut root, &mut con, &objects, &map);

        root.flush();
        for obj in objects.iter() {
            obj.clear(&mut con);
        }

        // Handle keys and exit if needed
        let player = &mut objects[0];
        let exit = handle_keys(&mut root, player, &map);
        if exit {
            break;
        }
    }

}

fn render_all(root: &mut Root, con: &mut Offscreen, objects: &[Object], map: &Map) {
    // Draw all the objects in the list
    for obj in objects {
        obj.draw(con)
    }

    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let opaque = !map[x as usize][y as usize].transparent;
            if opaque {
                con.set_char_background(x, y, COLOR_DARK_WALL, BackgroundFlag::Set);
            } else {
                con.set_char_background(x, y, COLOR_DARK_GROUND, BackgroundFlag::Set);
            }
        }
    }
    // Overlay the console over the root
    blit(con, (0, 0), (MAP_WIDTH, MAP_HEIGHT), root, (0, 0), 1.0, 1.0);
}

fn make_map() -> Map {
    // Fill map with untraversable tiles
    // vec![ITEM;NUM] is a macro to create a Vec of size NUM filled with ITEM (where ITEM is
    // evaluated at each iteration).
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];

    let room1 = Rect::new(20, 15, 10, 15);
    let room2 = Rect::new(50, 15, 10, 15);
    create_room(room1, &mut map);
    create_room(room2, &mut map);

    map
}

fn create_room(rect: Rect, map: &mut Map) {
    for x in (rect.x1 + 1)..rect.x2 {
        for y in (rect.y1 + 1)..rect.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

/// Handle a key press event
///
/// # Return value
///
/// A value of true means that the caller should exit.
fn handle_keys(root: &mut Root, player: &mut Object, map: &Map) -> bool {
    // TODO
    let key = root.wait_for_keypress(true);

    match key {
        // Player movement
        Key { code: Up, .. } | Key { printable: 'k', .. } => player.move_by(0, -1, map),
        Key { code: Down, .. } | Key { printable: 'j', .. }  => player.move_by(0, 1, map),
        Key { code: Left, .. } | Key { printable: 'h', .. }  => player.move_by(-1, 0, map),
        Key { code: Right, .. } | Key { printable: 'l', .. }  => player.move_by(1, 0, map),

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
