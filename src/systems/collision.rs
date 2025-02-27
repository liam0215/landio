use crate::components::{GridSettings, Player, Tile, Trail};
use crate::events::{PlayerDeathEvent, PlayerDeathReason};
use bevy::prelude::*;

pub fn collision_detection_system(
    mut player_query: Query<(Entity, &Transform, &Player)>,
    tile_query: Query<(Entity, &Tile, &Sprite)>,
    grid_settings: Res<GridSettings>,
    mut death_events: EventWriter<PlayerDeathEvent>,
) {
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

        // Create a grid representation
        let grid_width = grid_settings.grid_width as usize;
        let grid_height = grid_settings.grid_height as usize;

        // Create a grid to mark trail tiles
        let mut trail_grid = vec![vec![false; grid_width]; grid_height];

        // Collect trail tiles and mark them in the grid
        let mut trail_positions = Vec::new();

        // Fill the grid and collect trail positions
        for (_, tile, _) in tile_query.iter() {
            if tile.owner == Some(player_entity) && tile.is_trail {
                // Make sure tile coordinates are in bounds
                if tile.x >= 0
                    && tile.x < grid_settings.grid_width
                    && tile.y >= 0
                    && tile.y < grid_settings.grid_height
                {
                    trail_grid[tile.y as usize][tile.x as usize] = true;
                    trail_positions.push((tile.x, tile.y));
                }
            }
        }

        // Only check for collisions if the player has a substantial trail
        if trail_positions.len() < 10 {
            continue;
        }

        // Mark the "safe zone" around the player's current position
        // This is a larger area to prevent false positives when making tight turns
        let safe_radius = 2; // Increased from 1 to 2

        for dy in -safe_radius..=safe_radius {
            for dx in -safe_radius..=safe_radius {
                let check_x = current_x + dx;
                let check_y = current_y + dy;

                // Skip out of bounds
                if check_x < 0
                    || check_x >= grid_settings.grid_width
                    || check_y < 0
                    || check_y >= grid_settings.grid_height
                {
                    continue;
                }

                // Mark tiles close to the player as 'safe' from collision
                trail_grid[check_y as usize][check_x as usize] = false;
            }
        }

        // Check if player is directly on a trail tile (except the safe zone)
        // We need to check player's exact position, not just the grid cell
        let mut collision_detected = false;

        for &(tx, ty) in &trail_positions {
            // Skip if this position is within the safe zone
            let dx = (tx - current_x).abs();
            let dy = (ty - current_y).abs();
            if dx <= safe_radius && dy <= safe_radius {
                continue;
            }

            // Calculate distance to this trail tile
            let trail_center_x = (tx as f32 * tile_size) - half_width + (tile_size / 2.0);
            let trail_center_y = (ty as f32 * tile_size) - half_height + (tile_size / 2.0);
            let trail_pos = Vec2::new(trail_center_x, trail_center_y);

            // Check distance - making this slightly smaller to require more obvious crossings
            let collision_threshold = tile_size * 0.6; // Reduced from 0.8
            if player_pos.distance(trail_pos) < collision_threshold {
                collision_detected = true;

                // Debug info
                println!(
                    "Collision detected at trail pos ({},{}) - distance: {}",
                    tx,
                    ty,
                    player_pos.distance(trail_pos)
                );
                break;
            }
        }

        if collision_detected {
            // Send death event
            death_events.send(PlayerDeathEvent {
                player_entity,
                reason: PlayerDeathReason::TrailCollision,
            });
        }
    }
}
