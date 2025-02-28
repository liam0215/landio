use bevy::prelude::*;
mod components;
mod events;
mod resources;
mod systems;

use components::*;
use events::PlayerDeathEvent;
use resources::*;
use systems::collision::*;
use systems::input::*;
use systems::movement::*;
use systems::player::handle_player_death;
use systems::trails::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Land.io Clone".into(),
                resolution: (800., 600.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_event::<PlayerDeathEvent>()
        .insert_resource(GameState::default())
        .add_systems(Startup, setup_game)
        .add_systems(
            Update,
            (
                player_input_system,
                start_trail_system,
                player_movement_system,
                update_trail_system,
                render_trail_system,
                collision_detection_system,
                handle_player_death,
                claim_territory_system,
                game_timer_system,
                init_player_territory.run_if(run_once()),
            ),
        )
        .run();
}

fn setup_game(mut commands: Commands) {
    // Spawn camera
    commands.spawn(Camera2d::default());

    // Add grid settings resource
    let grid_settings = GridSettings::default();
    commands.insert_resource(grid_settings.clone());

    // Create grid of tiles
    let tile_size = grid_settings.tile_size;
    let half_width = (grid_settings.grid_width as f32 * tile_size) / 2.0;
    let half_height = (grid_settings.grid_height as f32 * tile_size) / 2.0;

    for y in 0..grid_settings.grid_height {
        for x in 0..grid_settings.grid_width {
            // Calculate position (centered in window)
            let pos_x = (x as f32 * tile_size) - half_width + (tile_size / 2.0);
            let pos_y = (y as f32 * tile_size) - half_height + (tile_size / 2.0);

            // Checkerboard pattern for visibility
            let is_dark = (x + y) % 2 == 0;
            let tile_color = if is_dark {
                Color::srgb(0.8, 0.8, 0.8) // Light gray
            } else {
                Color::srgb(0.9, 0.9, 0.9) // Lighter gray
            };

            commands.spawn((
                Sprite {
                    color: tile_color,
                    custom_size: Some(Vec2::new(tile_size, tile_size)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(pos_x, pos_y, -0.1)),
                GlobalTransform::default(),
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                Tile {
                    x,
                    y,
                    owner: None,
                    is_trail: false,
                },
            ));
        }
    }

    // Spawn player centered on a tile
    let player_color = Color::srgb(0.2, 0.7, 0.9);

    // Calculate center tile coordinates (this ensures we're on an actual tile)
    let center_tile_x = grid_settings.grid_width / 2;
    let center_tile_y = grid_settings.grid_height / 2;

    // Calculate the exact pixel position of the center tile
    let player_start_x = (center_tile_x as f32 * tile_size) - half_width + (tile_size / 2.0);
    let player_start_y = (center_tile_y as f32 * tile_size) - half_height + (tile_size / 2.0);

    // Spawn the player entity
    commands.spawn((
        Sprite {
            color: player_color,
            custom_size: Some(Vec2::new(tile_size * 0.8, tile_size * 0.8)), // Slightly smaller than tile
            ..default()
        },
        Transform::from_translation(Vec3::new(player_start_x, player_start_y, 0.0)),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
        Player {
            speed: 5.0, // Speed in tiles per second
            direction: Vec2::ZERO,
            buffered_direction: None,
            score: 0,
            color: player_color,
            is_drawing_trail: false,
            last_tile_pos: (center_tile_x, center_tile_y), // Set to the exact tile position
            is_moving_to_next_tile: false,
        },
    ));
}

fn game_timer_system(
    time: Res<Time>,
    mut game_state: ResMut<GameState>,
    player_query: Query<(Entity, &Player)>,
) {
    if game_state.game_running {
        game_state.timer.tick(time.delta());

        if game_state.timer.finished() {
            game_state.game_running = false;

            // Determine winner
            let mut highest_score = 0;
            let mut _winner = None;

            for (entity, player) in player_query.iter() {
                if player.score > highest_score {
                    highest_score = player.score;
                    _winner = Some(entity);
                }
            }

            // Here you would display the winner
            println!("Game over! Winner determined.");
        }
    }
}

fn init_player_territory(
    grid_settings: Res<GridSettings>,
    mut player_query: Query<(Entity, &mut Player)>,
    mut tile_query: Query<(&mut Tile, &mut Sprite)>,
) {
    // Get the player entity
    if let Ok((player_entity, player)) = player_query.get_single() {
        // Calculate center tile coordinates
        let center_tile_x = grid_settings.grid_width / 2;
        let center_tile_y = grid_settings.grid_height / 2;

        // Claim starting territory for the player
        let territory_radius = 2; // Claim a 5x5 area

        for (mut tile, mut sprite) in tile_query.iter_mut() {
            let dx = (tile.x - center_tile_x).abs();
            let dy = (tile.y - center_tile_y).abs();

            if dx <= territory_radius && dy <= territory_radius {
                // Mark as player territory
                tile.owner = Some(player_entity);
                sprite.color = player.color.with_alpha(0.5);
            }
        }

        // Give player initial score based on territory
        let territory_size = (territory_radius * 2 + 1).pow(2);
        if let Ok((_, mut player)) = player_query.get_single_mut() {
            player.score = territory_size as u32;
        }

        println!("Player starting with {} territory tiles", territory_size);
    }
}

// Add this helper for running a system only once
fn run_once() -> impl FnMut() -> bool {
    let mut has_run = false;
    move || {
        if !has_run {
            has_run = true;
            true
        } else {
            false
        }
    }
}
