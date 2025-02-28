// components.rs
use bevy::prelude::*;

#[derive(Component)]
pub struct Player {
    pub speed: f32,
    pub direction: Vec2,
    pub buffered_direction: Option<Vec2>,
    pub score: u32,
    pub color: Color,
    pub is_drawing_trail: bool,
    pub last_tile_pos: (i32, i32),
    pub is_moving_to_next_tile: bool,
}

#[derive(Component)]
pub struct Trail {
    pub owner: Entity,
    pub points: Vec<Vec2>,
    pub is_active: bool,
}

#[derive(Component)]
pub struct Tile {
    pub x: i32,
    pub y: i32,
    pub owner: Option<Entity>,
    pub is_trail: bool,
}

#[derive(Resource, Clone)]
pub struct GridSettings {
    pub tile_size: f32,
    pub grid_width: i32,
    pub grid_height: i32,
}

impl Default for GridSettings {
    fn default() -> Self {
        Self {
            tile_size: 20.0, // Each tile is 20x20 pixels
            grid_width: 40,  // 40 tiles across (800 pixels)
            grid_height: 30, // 30 tiles high (600 pixels)
        }
    }
}
