use bevy::prelude::*;
mod components;
mod resources;
mod systems;

use components::*;
use resources::*;
use systems::collision::*;
use systems::input::*;
use systems::movement::*;
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
        .insert_resource(GameState::default())
        .add_systems(Startup, setup_game)
        .add_systems(
            Update,
            (
                player_input_system,
                player_movement_system,
                start_trail_system,
                update_trail_system,
                render_trail_system,
                claim_territory_system,
                collision_detection_system,
                game_timer_system,
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
    let player_start_x = 0.0; // Center of grid
    let player_start_y = 0.0;

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
            score: 0,
            color: player_color,
            is_drawing_trail: false,
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
