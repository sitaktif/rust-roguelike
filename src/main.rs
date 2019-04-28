extern crate tcod;

use std::cmp::*;

use rand::Rng;

use tcod::console::*;
use tcod::colors::{self,Color};
use tcod::input::Key;
use tcod::input::KeyCode::*;

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;
const LIMIT_FPS: i32 = 40;

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;

const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;

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
    pub fn center(&self) -> (i32, i32) {
        ((self.x1 + self.x2) / 2, (self.y1 + self.y2) / 2)
    }
    pub fn intersects_with(&self, other: &Rect) -> bool {
        let x_intersects: bool = self.x2 >= other.x1 && self.x1 <= other.x2;
        let y_intersects: bool = self.y2 >= other.y1 && self.y1 <= other.y2;
        x_intersects && y_intersects
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

    let (map, (player_x, player_y)) = make_map();

    let player = Object::new(player_x, player_y, '@', colors::WHITE);
    // let npc = Object::new(player.x - 1, player.y -3, '@', colors::YELLOW);
    let mut objects = [player];


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

fn make_map() -> (Map, (i32, i32)) {
    // Fill map with untraversable tiles
    // vec![ITEM;NUM] is a macro to create a Vec of size NUM filled with ITEM (where ITEM is
    // evaluated at each iteration).
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];

    let mut starting_position = (0, 0);
    let mut rooms: Vec<Rect> = Vec::new();

    for _ in 0..MAX_ROOMS {
        // Random width / height
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);

        // random position without going out of the boundaries of the map.
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

        let new_room = Rect::new(x, y, w, h);

        // If this room intersects with another one, it is considered invalid.
        let invalid = rooms.iter().any(|other_room| new_room.intersects_with(other_room));
        if invalid {
            continue;
        }

        create_room(new_room, &mut map);
        let (new_x, new_y) = new_room.center();

        if rooms.is_empty() {
            // Player starts in first room, no tunnel needed.
            starting_position = (new_x, new_y);
        } else {
            // All other rooms should be connected with the previous one.
            let (prev_x, prev_y) = rooms[rooms.len() - 1].center();

            // We randomly use tunnel_x or tunnel_y first.
            if rand::random() {
                create_h_tunnel(prev_x, new_x, prev_y, &mut map);
                create_v_tunnel(prev_y, new_y, new_x, &mut map);
            } else {
                // first move vertically, then horizontally
                create_v_tunnel(prev_y, new_y, prev_x, &mut map);
                create_h_tunnel(prev_x, new_x, new_y, &mut map);
            }
        }

        rooms.push(new_room);

    }

    (map, starting_position)

}

fn create_room(rect: Rect, map: &mut Map) {
    for x in (rect.x1 + 1)..rect.x2 {
        for y in (rect.y1 + 1)..rect.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    for x in min(x1, x2)..(max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in min(y1, y2)..(max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
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
