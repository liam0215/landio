use crate::components::{GridSettings, Player, Tile, Trail};
use bevy::prelude::*;

pub fn collision_detection_system(
    mut commands: Commands,
    mut player_query: Query<(Entity, &Transform, &mut Player)>,
    _trail_query: Query<(Entity, &Trail)>,
    mut tile_query: Query<(Entity, &mut Tile, &mut Sprite)>,
    grid_settings: Res<GridSettings>,
) {
    let mut player_to_kill = None;

    // First, check for collisions
    for (player_entity, player_transform, player) in player_query.iter() {
        let player_pos = Vec2::new(
            player_transform.translation.x,
            player_transform.translation.y,
        );

        // If the player is not drawing a trail, they can't collide with anything
        if !player.is_drawing_trail {
            continue;
        }

        // Get the grid coordinates
        let tile_size = grid_settings.tile_size;
        let half_width = (grid_settings.grid_width as f32 * tile_size) / 2.0;
        let half_height = (grid_settings.grid_height as f32 * tile_size) / 2.0;
        let current_x = ((player_transform.translation.x + half_width) / tile_size).floor() as i32;
        let current_y = ((player_transform.translation.y + half_height) / tile_size).floor() as i32;

        // Count how many trail tiles this player already has
        // Only check for collisions if the player has a substantial trail
        let mut trail_count = 0;
        for (_, tile, _) in tile_query.iter() {
            if tile.is_trail && tile.owner == Some(player_entity) {
                trail_count += 1;
            }
        }

        // Only check for collisions if the player has a substantial trail
        // This prevents death right after starting to draw a trail
        if trail_count < 20 {
            continue;
        }

        // Check for overlapping trail tiles, but ignore the current position and immediate previous positions
        let mut recent_positions = Vec::new();

        // Track the last few positions to avoid false collisions with recently drawn trail
        for (_, tile, _) in tile_query.iter() {
            if tile.owner == Some(player_entity) && tile.is_trail {
                if tile.x == current_x && tile.y == current_y {
                    // Current position, always ignore
                    recent_positions.push((tile.x, tile.y));
                } else if (tile.x - current_x).abs() <= 1 && (tile.y - current_y).abs() <= 1 {
                    // Immediate neighbors, likely recent
                    recent_positions.push((tile.x, tile.y));
                }
            }
        }

        // Only check for non-recent trail tiles
        for (_, tile, _) in tile_query.iter() {
            // Only check the player's own trail tiles for collision
            if tile.owner == Some(player_entity) && tile.is_trail {
                // Skip if this is a recent position
                if recent_positions.contains(&(tile.x, tile.y)) {
                    continue;
                }

                // Only consider tiles that are close to the player's current position
                let tile_center_x = (tile.x as f32 * tile_size) - half_width + (tile_size / 2.0);
                let tile_center_y = (tile.y as f32 * tile_size) - half_height + (tile_size / 2.0);
                let tile_pos = Vec2::new(tile_center_x, tile_center_y);

                // Calculate distance from player to tile center
                let distance = player_pos.distance(tile_pos);

                // If player is very close to a non-recent trail tile, it's a collision
                if distance < tile_size * 0.75 {
                    // Smaller than a full tile size for better accuracy
                    println!(
                        "Player hit their own trail! Game over! (Distance: {})",
                        distance
                    );
                    player_to_kill = Some(player_entity);
                    break;
                }
            }
        }
    }

    // If a player needs to be killed, do it outside the original query borrowing
    if let Some(player_entity) = player_to_kill {
        kill_player(
            player_entity,
            &mut commands,
            &mut player_query,
            &mut tile_query,
            &grid_settings,
        );
    }
}

// Function to reset player when they die - no longer takes player as parameter
fn kill_player(
    player_entity: Entity,
    commands: &mut Commands,
    player_query: &mut Query<(Entity, &Transform, &mut Player)>,
    tile_query: &mut Query<(Entity, &mut Tile, &mut Sprite)>,
    _grid_settings: &Res<GridSettings>,
) {
    // Reset player
    if let Ok((_, _, mut player)) = player_query.get_mut(player_entity) {
        // Stop drawing trail
        player.is_drawing_trail = false;

        // Reset score
        player.score = 0;
    }

    // Reset player position to center
    commands
        .entity(player_entity)
        .insert(Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)));

    // Remove all player territories and trails
    for (_, mut tile, mut sprite) in tile_query.iter_mut() {
        if tile.owner == Some(player_entity) {
            tile.owner = None;
            tile.is_trail = false;

            // Reset to original color (checkerboard pattern)
            let is_dark = (tile.x + tile.y) % 2 == 0;
            sprite.color = if is_dark {
                Color::srgb(0.8, 0.8, 0.8) // Light gray
            } else {
                Color::srgb(0.9, 0.9, 0.9) // Lighter gray
            };
        }
    }

    println!("Player respawned. Score reset to 0.");
}
