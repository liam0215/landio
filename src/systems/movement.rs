// In src/systems/movement.rs
use crate::components::{GridSettings, Player, Tile};
use crate::resources::CompleteTrail;
use bevy::prelude::*;

pub fn player_movement_system(
    time: Res<Time>,
    grid_settings: Res<GridSettings>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut Player)>,
    mut tile_query: Query<(Entity, &mut Tile, &mut Sprite)>,
) {
    let tile_size = grid_settings.tile_size;
    let half_width = (grid_settings.grid_width as f32 * tile_size) / 2.0;
    let half_height = (grid_settings.grid_height as f32 * tile_size) / 2.0;

    for (entity, mut transform, mut player) in query.iter_mut() {
        if player.direction.length_squared() > 0.0 {
            // Calculate current grid position
            let current_x = ((transform.translation.x + half_width) / tile_size).floor() as i32;
            let current_y = ((transform.translation.y + half_height) / tile_size).floor() as i32;
            let current_pos = (current_x, current_y);

            // Calculate tile center position
            let tile_center_x = (current_x as f32 * tile_size) - half_width + (tile_size / 2.0);
            let tile_center_y = (current_y as f32 * tile_size) - half_height + (tile_size / 2.0);
            let tile_center = Vec2::new(tile_center_x, tile_center_y);

            // Calculate distance to tile center
            let distance_to_center =
                Vec2::new(transform.translation.x, transform.translation.y).distance(tile_center);

            // If we're at a tile center or just starting movement
            if distance_to_center < 0.5
                || (!player.is_moving_to_next_tile && current_pos != player.last_tile_pos)
            {
                // We've reached a new tile center
                player.is_moving_to_next_tile = false;
                player.last_tile_pos = current_pos;

                // Apply any buffered direction change now that we're at a tile center
                if let Some(new_dir) = player.buffered_direction {
                    player.direction = new_dir;
                    player.buffered_direction = None;
                    println!("Applied buffered direction: {:?}", player.direction);
                }

                // Mark that we're starting movement to the next tile
                player.is_moving_to_next_tile = true;

                // Determine current tile state - is this the player's territory?
                let mut current_is_territory = false;

                for (_, tile, _) in tile_query.iter() {
                    if tile.x == current_x && tile.y == current_y {
                        if tile.owner == Some(entity) {
                            if !tile.is_trail {
                                current_is_territory = true;
                            }
                        }
                        break;
                    }
                }

                // Determine next tile state based on current direction
                let next_dir = player.direction.normalize();
                let next_x = current_x + next_dir.x.round() as i32;
                let next_y = current_y + next_dir.y.round() as i32;

                // Check if next tile is in bounds
                if next_x >= 0
                    && next_x < grid_settings.grid_width
                    && next_y >= 0
                    && next_y < grid_settings.grid_height
                {
                    // Check if next tile is player's territory
                    let mut next_is_territory = false;

                    for (_, tile, _) in tile_query.iter() {
                        if tile.x == next_x && tile.y == next_y {
                            if tile.owner == Some(entity) {
                                if !tile.is_trail {
                                    next_is_territory = true;
                                }
                            }
                            break;
                        }
                    }

                    // Case 1: Currently on territory, about to leave territory
                    // Mark that we'll start drawing trail at the NEXT tile, not this one
                    if current_is_territory && !next_is_territory && !player.is_drawing_trail {
                        player.is_drawing_trail = true;
                        println!("Leaving territory - will start drawing trail on next tile");
                    }
                    // Case 2: Coming back to own territory while drawing a trail
                    // Complete the loop and claim territory
                    else if next_is_territory && player.is_drawing_trail {
                        println!("Returning to territory - will claim enclosed area");
                    }
                }

                // Process current tile (not the next one)
                for (_, mut tile, mut sprite) in tile_query.iter_mut() {
                    if tile.x == current_x && tile.y == current_y {
                        // If we're on our own territory and we're drawing a trail
                        // and it's not the tile we just started drawing from
                        if tile.owner == Some(entity) && !tile.is_trail && player.is_drawing_trail {
                            // Player returned to their territory - complete the trail
                            player.is_drawing_trail = false;
                            println!(
                                "Player returned to their territory - claiming enclosed area!"
                            );

                            commands.insert_resource(CompleteTrail {
                                player: Some(entity),
                                complete: true,
                                entry_point: Some((current_x, current_y)),
                            });
                        }
                        // Mark as part of trail if drawing and NOT the player's territory
                        else if player.is_drawing_trail && !current_is_territory {
                            tile.is_trail = true;
                            tile.owner = Some(entity);

                            // Keep consistent trail color
                            sprite.color = player.color.with_alpha(0.8);
                        }
                        break;
                    }
                }
            }

            // Apply movement (smooth)
            let normalized_dir = player.direction.normalize();
            let movement = normalized_dir * player.speed * time.delta_secs();
            transform.translation.x += movement.x * tile_size;
            transform.translation.y += movement.y * tile_size;

            // Calculate new grid position
            let new_x = ((transform.translation.x + half_width) / tile_size).floor() as i32;
            let new_y = ((transform.translation.y + half_height) / tile_size).floor() as i32;

            // Constrain to grid boundaries
            let constrained_x = new_x.clamp(0, grid_settings.grid_width - 1);
            let constrained_y = new_y.clamp(0, grid_settings.grid_height - 1);

            // If we've gone beyond the grid boundaries, snap back
            if constrained_x != new_x || constrained_y != new_y {
                transform.translation.x =
                    (constrained_x as f32 * tile_size) - half_width + (tile_size / 2.0);
                transform.translation.y =
                    (constrained_y as f32 * tile_size) - half_height + (tile_size / 2.0);
                player.is_moving_to_next_tile = false; // We've snapped to a tile center
            }
        }

        // Check for loops in trail
        if player.is_drawing_trail {
            // Create a grid representation of all tiles
            let grid_width = grid_settings.grid_width;
            let grid_height = grid_settings.grid_height;
            let mut tile_grid = vec![vec![None; grid_width as usize]; grid_height as usize];

            // Fill the grid with current tile state
            for (_, tile, _) in tile_query.iter() {
                if tile.x >= 0 && tile.x < grid_width && tile.y >= 0 && tile.y < grid_height {
                    tile_grid[tile.y as usize][tile.x as usize] = Some((tile.owner, tile.is_trail));
                }
            }

            // Find all trail tiles for this player
            let mut trail_points = Vec::new();
            for y in 0..grid_height as usize {
                for x in 0..grid_width as usize {
                    if let Some((owner, is_trail)) = tile_grid[y][x] {
                        if is_trail && owner == Some(entity) {
                            trail_points.push((x as i32, y as i32));
                        }
                    }
                }
            }

            // Simple loop detection: check if any trail tile has more than 2 trail neighbors
            let mut has_loop = false;
            for &(tx, ty) in &trail_points {
                let neighbors = [(tx + 1, ty), (tx - 1, ty), (tx, ty + 1), (tx, ty - 1)];

                let mut trail_neighbor_count = 0;
                for &(nx, ny) in &neighbors {
                    if nx >= 0 && nx < grid_width && ny >= 0 && ny < grid_height {
                        if let Some((owner, is_trail)) = tile_grid[ny as usize][nx as usize] {
                            if is_trail && owner == Some(entity) {
                                trail_neighbor_count += 1;
                            }
                        }
                    }
                }

                if trail_neighbor_count > 2 {
                    has_loop = true;
                    break;
                }
            }

            if has_loop {
                println!("Loop detected! Triggering territory claim...");
                player.is_drawing_trail = false;

                // Trigger claim_territory_system to accurately determine enclosed areas
                commands.insert_resource(CompleteTrail {
                    player: Some(entity),
                    complete: true,
                    entry_point: None,
                });
            }
        }
    }
}
