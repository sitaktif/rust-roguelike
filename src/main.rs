extern crate tcod;

use std::cmp::*;

use rand::Rng;

use tcod::console::*;
use tcod::colors::{self,Color};
use tcod::input::Key;
use tcod::input::KeyCode::*;
use tcod::map::{Map as FovMap, FovAlgorithm};

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;
const LIMIT_FPS: i32 = 40;

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;

const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;
const MAX_ROOM_MONSTERS: i32 = 3;

const PLAYER_IDX: usize = 0;

const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOR_LIGHT_WALL: Color = Color { r: 130, g: 110, b: 50 };
const COLOR_DARK_GROUND: Color = Color { r: 50, g: 50, b: 150 };
const COLOR_LIGHT_GROUND: Color = Color { r: 200, g: 180, b: 50 };
const COLOR_ORC: Color = colors::DESATURATED_GREEN;
const COLOR_TROLL: Color = colors::DARKER_GREEN;

const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
const FOV_LIGHT_WALLS: bool = true;
const TORCH_RADIUS: i32 = 10;

struct Object {
    x: i32,
    y: i32,
    char: char,
    name: String,
    color: Color,
    traversable: bool,
    alive: bool,
}

impl Object {
    pub fn new(x: i32, y: i32, char: char, name: &str, color: Color, traversable: bool) -> Self {
        Object {
            x, y, char, name: name.to_string(), color, traversable, alive: false
        }
    }
    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }
    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }
    pub fn draw(&self, con: &mut Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }
    pub fn clear(&self, con: &mut Console) {
        con.put_char(self.x, self.y, ' ', BackgroundFlag::None);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

/// Move object by the given amount
/// Note: because we need to pass the object vec, we have a borrow issue if we write this as a
///     method: self (of type Object) would be borrowed as mutable but the vector of objects would
///     contain a ref to self and the borrow checked wouldn't allow that.
fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut Vec<Object>) {
    let (x, y) = objects[id].pos();
    if is_traversable(x + dx, y + dy, map, objects) {
        objects[id].set_pos(x + dx, y + dy);
    }
}

#[derive(Clone, Copy, Debug)]
struct Tile {
    explored: bool,
    traversable: bool,
    transparent: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile { explored: false, traversable: true, transparent: true }
    }

    pub fn wall() -> Self {
        Tile { explored: false, traversable: false, transparent: false }
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

    let mut objects = Vec::new();
    let (mut map, (player_x, player_y)) = make_map(&mut objects);

    let mut player = Object::new(player_x, player_y, '@', "player", colors::WHITE, false);
    player.alive = true;

    // let npc = Object::new(player.x - 1, player.y -3, '@', colors::YELLOW);
    objects.insert(PLAYER_IDX, player);

    // Fill the field-of-view map
    let mut fov_map = FovMap::new(MAP_WIDTH, MAP_HEIGHT);
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            fov_map.set(x, y,
                        map[x as usize][y as usize].transparent,
                        map[x as usize][y as usize].traversable);
        }
    }

    let mut prev_player_position = (-1, -1);

    while !root.window_closed() {
        // clear the screen of the previous frame
        con.clear();

        con.set_default_foreground(colors::WHITE);
        let player = &mut objects[PLAYER_IDX];

        let fov_recompute = prev_player_position != (player.x, player.y);
        render_all(&mut root, &mut con, &objects, &mut map, &mut fov_map, fov_recompute);

        root.flush();

        let player = &mut objects[PLAYER_IDX];
        prev_player_position = (player.x, player.y);

        // Handle keys and exit if needed
        match handle_keys(&mut root, &map, &mut objects) {
            PlayerAction::TookTurn => (),
            PlayerAction::DidntTakeTurn => (),
            PlayerAction::Exit => break,
        }
    }

}

fn render_all(root: &mut Root, con: &mut Offscreen, objects: &[Object], map: &mut Map,
              fov_map: &mut FovMap, fov_recompute: bool) {
    if fov_recompute {
        // Recompute FOV if needed (the player moved or something)
        let player = &objects[PLAYER_IDX];
        fov_map.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);
    }

    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let visible = fov_map.is_in_fov(x, y);
            let wall = !map[x as usize][y as usize].transparent;
            let explored = &mut map[x as usize][y as usize].explored;

            let color = match(visible, wall) {
                (false, true) => COLOR_DARK_WALL,
                (false, false) => COLOR_DARK_GROUND,
                (true, true) => COLOR_LIGHT_WALL,
                (true, false) => COLOR_LIGHT_GROUND,
            };
            if visible {
                // Since it's visible, we should mark it as explored.
                *explored = true;
            }
            if *explored {
                con.set_char_background(x, y, color, BackgroundFlag::Set);
            }
        }
    }

    // Draw all the objects in the list
    for obj in objects {
        if fov_map.is_in_fov(obj.x, obj.y) {
            obj.draw(con);
        }
    }

    // Overlay the console over the root
    blit(con, (0, 0), (MAP_WIDTH, MAP_HEIGHT), root, (0, 0), 1.0, 1.0);
}

fn make_map(objects: &mut Vec<Object>) -> (Map, (i32, i32)) {
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
            // Place objets (monsters, items, ...).
            place_objects(&new_room, &map, objects);

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

/// Create objects (monsters, items) in a given room.
fn place_objects(room: &Rect, map: &Map, objects: &mut Vec<Object>) {
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);
    let Rect { x1, y1, x2, y2 } = *room;

    for _ in 0..num_monsters {
        let x = rand::thread_rng().gen_range(x1 + 1, x2);
        let y = rand::thread_rng().gen_range(y1 + 1, y2);

        if is_traversable(x, y, map, objects) {
            // 80% chance orc, 20% troll
            let mut new_monster = if rand::random::<f32>() < 0.8 {
                Object::new(x, y, 'o', "orc", COLOR_ORC, false)
            } else {
                Object::new(x, y, 'T', "troll", COLOR_TROLL, false)
            };
            new_monster.alive = true;
            objects.push(new_monster);
        }
    }
}

// Movement
fn is_traversable(x: i32, y: i32, map: &Map, objects: &Vec<Object>) -> bool {
    // Could be blocked by a tile...
    if ! map[x as usize][y as usize].traversable {
        return false;
    }
    // ...or by an object.
    ! objects.iter().any(|o| {
        ! o.traversable && o.pos() == (x, y)
    })
}

/// Handle a key press event
///
/// # Return value
///
/// A value of true means that the caller should exit.
fn handle_keys(root: &mut Root, map: &Map, objects: &mut Vec<Object>) -> PlayerAction {

    use self::PlayerAction::*;

    let key = root.wait_for_keypress(true);
    let player_alive = objects[PLAYER_IDX].alive;

    let mut do_move_by = |dx: i32, dy: i32| {
        move_by(PLAYER_IDX, dx, dy, map, objects);
        TookTurn
    };

    match (key, player_alive) {
        // Player movement
        (Key { printable: 'k', .. }, true) => do_move_by(0, -1),
        (Key { printable: 'j', .. }, true) => do_move_by(0, 1),
        (Key { printable: 'h', .. }, true) => do_move_by(-1, 0),
        (Key { printable: 'l', .. }, true) => do_move_by(1, 0),
        (Key { printable: 'y', .. }, true) => do_move_by(-1, -1),
        (Key { printable: 'u', .. }, true) => do_move_by(1, -1),
        (Key { printable: 'b', .. }, true) => do_move_by(-1, 1),
        (Key { printable: 'n', .. }, true) => do_move_by(1, 1),

        // Alt-enter: toggle fullscreen
        (Key { code: Enter, alt: true, .. }, _) => {
            root.set_fullscreen(!root.is_fullscreen());
            DidntTakeTurn
        },

        // Exit the game
        (Key { code: Escape, .. }, _) => Exit,

        // Ignore other keys
        _ => DidntTakeTurn,
    }
}
