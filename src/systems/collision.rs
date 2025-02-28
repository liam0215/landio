use crate::components::{GridSettings, Player, Tile};
use crate::events::{PlayerDeathEvent, PlayerDeathReason};
use bevy::prelude::*;

pub fn collision_detection_system(
    player_query: Query<(Entity, &Transform, &Player)>,
    tile_query: Query<(Entity, &Tile, &Sprite)>,
    grid_settings: Res<GridSettings>,
    mut death_events: EventWriter<PlayerDeathEvent>,
) {
    // This system will handle mid-movement collisions
    // The tile-level collisions are now handled by the movement system

    for (player_entity, player_transform, player) in player_query.iter() {
        // If the player is not drawing a trail, they can't collide with anything
        if !player.is_drawing_trail {
            continue;
        }

        let player_pos = Vec2::new(
            player_transform.translation.x,
            player_transform.translation.y,
        );

        // Get the grid coordinates
        let tile_size = grid_settings.tile_size;
        let half_width = (grid_settings.grid_width as f32 * tile_size) / 2.0;
        let half_height = (grid_settings.grid_height as f32 * tile_size) / 2.0;
        let current_x = ((player_transform.translation.x + half_width) / tile_size).floor() as i32;
        let current_y = ((player_transform.translation.y + half_height) / tile_size).floor() as i32;

        // Collect all trail tiles that could be collided with
        let mut trail_positions = Vec::new();

        for (_, tile, _) in tile_query.iter() {
            // Only consider collisions with the player's own trail
            if tile.owner == Some(player_entity) && tile.is_trail {
                // Skip the current tile and immediate neighbors (safe zone)
                let dx = (tile.x - current_x).abs();
                let dy = (tile.y - current_y).abs();

                if dx <= 1 && dy <= 1 {
                    continue;
                }

                trail_positions.push((tile.x, tile.y));
            }
        }

        // Check for collisions with trail tiles
        let mut collision_detected = false;

        for &(tx, ty) in &trail_positions {
            // Calculate distance to this trail tile's center
            let trail_center_x = (tx as f32 * tile_size) - half_width + (tile_size / 2.0);
            let trail_center_y = (ty as f32 * tile_size) - half_height + (tile_size / 2.0);
            let trail_pos = Vec2::new(trail_center_x, trail_center_y);

            // Original collision threshold
            let collision_threshold = tile_size * 0.7; // Slightly more forgiving

            if player_pos.distance(trail_pos) < collision_threshold {
                collision_detected = true;
                println!(
                    "⚠️ Mid-movement collision detected with trail at ({},{})",
                    tx, ty
                );
                break;
            }
        }

        if collision_detected {
            death_events.send(PlayerDeathEvent {
                player_entity,
                reason: PlayerDeathReason::TrailCollision,
            });
        }
    }
}
