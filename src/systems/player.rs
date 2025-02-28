use crate::components::{GridSettings, Player, Tile};
use crate::events::{PlayerDeathEvent, PlayerDeathReason};
use crate::CompleteTrail;
use bevy::prelude::*;

// System that handles player death events
pub fn handle_player_death(
    mut commands: Commands,
    mut death_events: EventReader<PlayerDeathEvent>,
    mut player_query: Query<&mut Player>,
    mut tile_query: Query<(Entity, &mut Tile, &mut Sprite)>,
    grid_settings: Res<GridSettings>,
    // Add this to cancel any pending territory claiming
    mut complete_trail: Option<ResMut<CompleteTrail>>,
) {
    // Skip if no death events
    if death_events.is_empty() {
        return;
    }

    // First, explicitly cancel any pending territory claiming operations
    if let Some(mut trail_info) = complete_trail {
        // Clear the complete trail resource to cancel any territory claiming
        trail_info.complete = false;
        trail_info.player = None;
        trail_info.entry_point = None;
        println!("Cancelled any pending territory claims due to player death");
    }

    for event in death_events.read() {
        let player_entity = event.player_entity;

        match event.reason {
            PlayerDeathReason::TrailCollision => {
                println!("⚠️ PLAYER HIT THEIR OWN TRAIL! GAME OVER! ⚠️");
            }
            PlayerDeathReason::CrossedTrail => {
                println!("PLAYER CROSSED OWN TRAIL - PLAYER DIES!");
            }
            PlayerDeathReason::OutOfBounds => {
                println!("PLAYER WENT OUT OF BOUNDS - PLAYER DIES!");
            }
            PlayerDeathReason::HitOtherPlayer => {
                println!("PLAYER HIT ANOTHER PLAYER - PLAYER DIES!");
            }
        }

        // Reset player
        let player_color = if let Ok(player) = player_query.get(player_entity) {
            player.color
        } else {
            Color::srgba(0.2, 0.7, 0.9, 1.0) // Default color
        };

        if let Ok(mut player) = player_query.get_mut(player_entity) {
            // Stop drawing trail immediately
            player.is_drawing_trail = false;
            player.buffered_direction = None;

            // Set direction to zero to stop movement
            player.direction = Vec2::ZERO;

            // Reset score to ZERO - lose all points!
            player.score = 0;
        }

        // Reset player position to center of grid
        let center_tile_x = grid_settings.grid_width / 2;
        let center_tile_y = grid_settings.grid_height / 2;
        let tile_size = grid_settings.tile_size;
        let half_width = (grid_settings.grid_width as f32 * tile_size) / 2.0;
        let half_height = (grid_settings.grid_height as f32 * tile_size) / 2.0;

        let center_x = (center_tile_x as f32 * tile_size) - half_width + (tile_size / 2.0);
        let center_y = (center_tile_y as f32 * tile_size) - half_height + (tile_size / 2.0);

        // Update player transform and position
        commands
            .entity(player_entity)
            .insert(Transform::from_translation(Vec3::new(
                center_x, center_y, 0.0,
            )));

        // Also update player.last_tile_pos to the center tile
        if let Ok(mut player) = player_query.get_mut(player_entity) {
            player.last_tile_pos = (center_tile_x, center_tile_y);
        }

        let mut grid = vec![
            vec![false; grid_settings.grid_width as usize];
            grid_settings.grid_height as usize
        ];

        // First mark all tiles that are owned by this player in the grid
        for (_, tile, _) in tile_query.iter() {
            if tile.x >= 0
                && tile.x < grid_settings.grid_width
                && tile.y >= 0
                && tile.y < grid_settings.grid_height
                && tile.owner == Some(player_entity)
            {
                grid[tile.y as usize][tile.x as usize] = true;
            }
        }

        // Now reset ALL player tiles based on the grid
        let mut territory_count = 0;
        let mut trail_count = 0;

        for (_, mut tile, mut sprite) in tile_query.iter_mut() {
            if tile.x >= 0
                && tile.x < grid_settings.grid_width
                && tile.y >= 0
                && tile.y < grid_settings.grid_height
            {
                let x = tile.x as usize;
                let y = tile.y as usize;

                if grid[y][x] {
                    // Count what we're removing
                    if tile.is_trail {
                        trail_count += 1;
                    } else {
                        territory_count += 1;
                    }

                    // Reset ownership and appearance
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
        }

        println!(
            "Player lost {} territory tiles and {} trail tiles.",
            territory_count, trail_count
        );

        // Pause briefly to ensure all tiles are reset
        // This is just a safety measure and doesn't actually pause execution

        // Give player initial territory just like at first spawn
        let territory_radius = 2; // Creates a 5x5 area (2 tiles in each direction from center)
        let mut initial_territory_count = 0;

        for (_, mut tile, mut sprite) in tile_query.iter_mut() {
            let dx = (tile.x - center_tile_x).abs();
            let dy = (tile.y - center_tile_y).abs();

            if dx <= territory_radius && dy <= territory_radius {
                // Double check that this tile is NOT still owned
                // (This is a sanity check that should never fail if the above code works)
                if tile.owner.is_none() {
                    // Mark as player territory
                    tile.owner = Some(player_entity);
                    tile.is_trail = false;
                    sprite.color = player_color.with_alpha(0.5);
                    initial_territory_count += 1;
                } else {
                    // Print warning if we find a tile still owned by someone
                    println!(
                        "WARNING: Tile at ({}, {}) is still owned during respawn!",
                        tile.x, tile.y
                    );
                }
            }
        }

        // Update player score based on initial territory
        if let Ok(mut player) = player_query.get_mut(player_entity) {
            player.score = initial_territory_count;
        }

        println!(
            "Player respawned at center with {} initial territory tiles.",
            initial_territory_count
        );
    }

    // Clear death events to ensure they don't process again
    death_events.clear();
}
