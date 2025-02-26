// resources.rs
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource)]
pub struct GameState {
    pub timer: Timer,
    pub player_scores: HashMap<Entity, u32>,
    pub game_running: bool,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(300.0, TimerMode::Once), // 5 minutes
            player_scores: HashMap::new(),
            game_running: false,
        }
    }
}

#[derive(Resource, Default)]
pub struct CompleteTrail {
    pub player: Option<Entity>,
    pub complete: bool,
}
