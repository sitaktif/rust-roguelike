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
const MAP_HEIGHT: i32 = 43;

// Panel constants.
const BAR_WIDTH: i32 = 20;
const PANEL_HEIGHT: i32 = 7;
const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;
const MSG_X: i32 = BAR_WIDTH + 2;
const MSG_WIDTH: i32 = SCREEN_WIDTH - MSG_X;
const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;

const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;
const MAX_ROOM_MONSTERS: i32 = 3;

const PLAYER_ID: usize = 0;

const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOR_LIGHT_WALL: Color = Color { r: 130, g: 110, b: 50 };
const COLOR_DARK_GROUND: Color = Color { r: 50, g: 50, b: 150 };
const COLOR_LIGHT_GROUND: Color = Color { r: 200, g: 180, b: 50 };
const COLOR_ORC: Color = colors::DESATURATED_GREEN;
const COLOR_TROLL: Color = colors::DARKER_GREEN;

const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
const FOV_LIGHT_WALLS: bool = true;
const TORCH_RADIUS: i32 = 10;


// Common functions

/// Mutably borrow two different elements from the given slice.
/// Panics if the two indices have the same value, or if they are out of bounds.
pub fn mut_two<T>(items: &mut [T], i: usize, j: usize) -> (&mut T, &mut T) {
    assert!(i != j, "The two indices should be different");
    let (left, right) = items.split_at_mut(std::cmp::max(i, j));
    if i < j {
        (&mut left[i], &mut right[0])
    } else {
        (&mut right[0], &mut left[j])
    }
}


// Specific code

#[derive(Copy, Clone, Debug, PartialEq)]
struct Fighter {
    max_hp: i32,
    hp: i32,
    defence: i32,
    power: i32,
    on_death: DeathCallback,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum DeathCallback {
    Player,
    Monster,
}

impl DeathCallback {
    fn callback(self, object: &mut Object, messages: &mut Messages) {
        use self::DeathCallback::*;
        let callback: fn(&mut Object, &mut Messages) = match self {
            Player => player_death,
            Monster => monster_death,
        };
        callback(object, messages);
    }
}

fn player_death(player: &mut Object, messages: &mut Messages) {
    // The game ends!
    log_message(messages, "You died!", colors::DARK_RED);

    // Transform the player into a corpse
    player.alive = false;
    player.char = '%';
    player.color = colors::DARK_RED;
    player.fighter = None;
}

fn monster_death(monster: &mut Object, messages: &mut Messages) {
    // Transform into a traversable, unattackable, immobile corpse
    log_message(messages, format!("{} is dead!", monster.name), colors::ORANGE);
    monster.char = '%';
    monster.color = colors::DARK_RED;
    monster.traversable = true;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct Ai;

struct Object {
    x: i32,
    y: i32,
    char: char,
    name: String,
    color: Color,
    traversable: bool,
    alive: bool,
    fighter: Option<Fighter>,
    ai: Option<Ai>,

}

impl Object {
    pub fn new(x: i32, y: i32, char: char, name: &str, color: Color, traversable: bool) -> Self {
        Object {
            x,
            y,
            char,
            name: name.to_string(),
            color,
            traversable,
            alive: false,
            fighter: None,
            ai: None,
        }
    }

    // Movement
    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }
    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }
    pub fn distance_to(&self, other: &Object) -> f32 {
        (((self.x - other.x).pow(2) + (self.y - other.y).pow(2)) as f32).sqrt()
    }

    // Fight
    pub fn take_damage(&mut self, damage: i32, messages: &mut Messages) {
        // Apply damage if possible
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
            let fighter = &*fighter;  // Change into an immutable reference.
            if fighter.hp <= 0 {
                self.alive = false;
                fighter.on_death.callback(self, messages);
            }
        }
    }
    pub fn attack(&mut self, target: &mut Object, messages: &mut Messages) {
        let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defence);
        if damage > 0 {
            log_message(messages, format!("{} attacks {} for {} hit points!", self.name, target.name, damage), colors::WHITE);
            target.take_damage(damage, messages);
        } else {
            log_message(messages, format!("{} attacks {} but it has no effect!", self.name, target.name), colors::WHITE);
        }
    }

    // Graphics
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
fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
    let (x, y) = objects[id].pos();
    if is_traversable(x + dx, y + dy, map, objects) {
        objects[id].set_pos(x + dx, y + dy);
    }
}
fn move_by_or_attack(id: usize, dx: i32, dy: i32, map: &Map,
                     objects: &mut Vec<Object>, messages: &mut Messages) {
    // The coordinates the player is moving to/attacking.
    let x = objects[id].x + dx;
    let y = objects[id].y + dy;

    // Try to find an attackable object there.
    let target_id = objects.iter().position(|o| {
        o.fighter.is_some() && o.pos() == (x, y)
    });

    // Attack if such an object is found.
    match target_id {
        Some(target_id) => {
            let (player, target) = mut_two(objects, PLAYER_ID, target_id);
            player.attack(target, messages);
        },
        None => move_by(id, dx, dy, map, objects),
    }
}

fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [Object]) {
    // Vector from object to target.
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let dist = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    // Normalise the vector.
    let dx = (dx as f32 / dist).round() as i32;
    let dy = (dy as f32 / dist).round() as i32;

    move_by(id, dx, dy, map, objects);
}

fn ai_take_turn(monster_id: usize, map: &Map, objects: &mut [Object], messages: &mut Messages,
                fov_map: &FovMap) {
    // Basic monster takes its turn; if you can see it, it can see you.
    let (monster_x, monster_y) = objects[monster_id].pos();
    if fov_map.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[PLAYER_ID]) >= 2.0 {
            // Move towards the player.
            move_towards(monster_id, objects[PLAYER_ID].x, objects[PLAYER_ID].y, map, objects);
        } else {
            let (monster, player) = mut_two(objects, monster_id, PLAYER_ID);
            monster.attack(player, messages);
        }
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

type Messages = Vec<(String, Color)>;

fn main() {
    println!("Hello, world!");

    let mut root = Root::initializer()
    .font("arial10x10.png", FontLayout::Tcod)
    .font_type(FontType::Greyscale)
    .size(SCREEN_WIDTH, SCREEN_HEIGHT)
    .title("Rust/libtcod tutorial")
    .init();

    let mut con = Offscreen::new(MAP_WIDTH, MAP_HEIGHT);
    let mut panel = Offscreen::new(MAP_WIDTH, PANEL_HEIGHT);
    let mut messages = vec![];

    tcod::system::set_fps(LIMIT_FPS);

    let mut objects = Vec::new();
    let (mut map, (player_x, player_y)) = make_map(&mut objects);

    let mut player = Object::new(player_x, player_y, '@', "player", colors::WHITE, false);
    player.alive = true;
    player.fighter = Some(Fighter { max_hp: 30, hp: 30, defence: 2, power: 5, on_death: DeathCallback::Player });

    // let npc = Object::new(player.x - 1, player.y -3, '@', colors::YELLOW);
    objects.insert(PLAYER_ID, player);

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

    // A warm welcoming message!
    log_message(
        &mut messages,
        "Welcome stranger! Prepare to perish in the Tombs of the Ancient Kings.",
        colors::RED,
    );

    while !root.window_closed() {
        // clear the screen of the previous frame
        con.clear();

        con.set_default_foreground(colors::WHITE);
        let player = &mut objects[PLAYER_ID];

        let fov_recompute = prev_player_position != (player.x, player.y);
        render_all(&mut root, &mut con, &mut panel, &objects, &messages,
                   &mut map, &mut fov_map, fov_recompute);

        root.flush();

        let player = &mut objects[PLAYER_ID];
        prev_player_position = (player.x, player.y);

        // Handle keys and exit if needed
        let player_action = handle_keys(&mut root, &map, &mut objects, &mut messages);
        if player_action == PlayerAction::Exit {
            break;
        }
        if objects[PLAYER_ID].alive && player_action != PlayerAction::DidntTakeTurn {
            for o in objects.iter().filter(
                |x| (x.name) != (objects[PLAYER_ID].name) &&
                x.distance_to(&objects[PLAYER_ID]) < 5_f32 &&
                x.fighter.is_some()
                ) {
                log_message(&mut messages, format!("The {} growls!", o.name), colors::DARK_RED);
            }
        }
        for id in 0..objects.len() {
            if objects[id].ai.is_some() {
                ai_take_turn(id, &map, &mut objects, &mut messages, &fov_map);
            }
        }
    }

}

fn render_all(root: &mut Root, con: &mut Offscreen, panel: &mut Offscreen,
              objects: &[Object], messages: &Messages, map: &mut Map,
              fov_map: &mut FovMap, fov_recompute: bool) {
    if fov_recompute {
        // Recompute FOV if needed (the player moved or something).
        let player = &objects[PLAYER_ID];
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

    // Draw all the objects in the list.
    let mut to_draw: Vec<_> = objects.iter().filter(|o| fov_map.is_in_fov(o.x, o.y)).collect();
    // Sort to put non-blocking objects first.
    to_draw.sort_by(|o1, o2| o2.traversable.cmp(&o1.traversable));
    for obj in to_draw {
        if fov_map.is_in_fov(obj.x, obj.y) {
            obj.draw(con);
        }
    }

    // Overlay the console over the root.
    blit(con, (0, 0), (MAP_WIDTH, MAP_HEIGHT), root, (0, 0), 1.0, 1.0);


    // Show the player stats
    if let Some(fighter) = objects[PLAYER_ID].fighter {
        // Prepare to renter the GUI panel.
        panel.set_default_background(colors::BLACK);
        panel.clear();

        let hp = objects[PLAYER_ID].fighter.map_or(0, |f| f.hp);
        let max_hp = objects[PLAYER_ID].fighter.map_or(0, |f| f.max_hp);
        render_bar(
            panel,
            1,
            1,
            BAR_WIDTH,
            "HP",
            hp,
            max_hp,
            colors::LIGHT_RED,
            colors::DARKER_RED,
            );

        render_messages(messages, panel);

        blit(
            panel,
            (0, 0),
            (SCREEN_WIDTH, SCREEN_HEIGHT),
            root,
            (0, PANEL_Y),
            1.0,
            1.0,
        );
    }
}

fn render_messages(messages: &Messages, panel: &mut Offscreen) {
    let mut y = MSG_HEIGHT as i32;
    for &(ref msg, color) in messages.iter().rev() {
        let msg_height = panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        panel.set_default_foreground(color);
        panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }
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
                let mut orc = Object::new(x, y, 'o', "orc", COLOR_ORC, false);
                orc.fighter = Some(Fighter { max_hp: 10, hp: 10, defence: 0, power: 3, on_death: DeathCallback::Monster});
                orc.ai = Some(Ai);
                orc
            } else {
                let mut troll = Object::new(x, y, 'T', "troll", COLOR_TROLL, false);
                troll.fighter = Some(Fighter { max_hp: 16, hp: 16, defence: 1, power: 4, on_death: DeathCallback::Monster});
                troll.ai = Some(Ai);
                troll
            };
            new_monster.alive = true;
            objects.push(new_monster);
        }
    }
}

// Movement
fn is_traversable(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    // Could be blocked by a tile...
    if ! map[x as usize][y as usize].traversable {
        return false;
    }
    // ...or by an object.
    ! objects.iter().any(|o| {
        ! o.traversable && o.pos() == (x, y)
    })
}

fn render_bar(
    panel: &mut Offscreen,
    x: i32,
    y: i32,
    total_width: i32,
    name: &str,
    value: i32,
    maximum: i32,
    bar_color: Color,
    bg_color: Color,
) {
    // Render a bar (HP, experience, etc). First calculate the width of the bar.
    let bar_width = (total_width as f32 * value as f32 / maximum as f32) as i32;

    // Render the bg first.
    panel.set_default_background(bg_color);
    panel.rect(x, y, total_width, 1, false, BackgroundFlag::Screen);

    // Render the bar on top.
    panel.set_default_background(bar_color);
    if bar_width > 0 {
        panel.rect(x, y, bar_width, 1, false, BackgroundFlag::Screen);
    }

    // Finally add centered text with the values.
    panel.set_default_foreground(colors::WHITE);
    panel.print_ex(
        x + total_width / 2,
        y,
        BackgroundFlag::None,
        TextAlignment::Center,
        &format!("{}: {}/{}", name, value, maximum),
    );
}

fn log_message<T: Into<String>>(messages: &mut Messages, message: T, color: Color) {
    // If the buffer is full, make way.
    if messages.len() == MSG_HEIGHT {
        messages.remove(0);
    }
    messages.push((message.into(), color));
}

/// Handle a key press event
///
/// # Return value
///
/// A value of true means that the caller should exit.
fn handle_keys(root: &mut Root, map: &Map, objects: &mut Vec<Object>, messages: &mut Messages) -> PlayerAction {

    use self::PlayerAction::*;

    let key = root.wait_for_keypress(true);
    let player_alive = objects[PLAYER_ID].alive;

    let mut do_move_by = |dx: i32, dy: i32| {
        move_by_or_attack(PLAYER_ID, dx, dy, map, objects, messages);
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
