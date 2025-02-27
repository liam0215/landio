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
            // Get current grid position
            let current_x = ((transform.translation.x + half_width) / tile_size).floor() as i32;
            let current_y = ((transform.translation.y + half_height) / tile_size).floor() as i32;

            // Calculate movement (grid-based)
            let normalized_dir = player.direction.normalize();
            let movement = normalized_dir * player.speed * time.delta_secs();

            // Apply movement
            transform.translation.x += movement.x * tile_size;
            transform.translation.y += movement.y * tile_size;

            // Calculate new grid position
            let new_x = ((transform.translation.x + half_width) / tile_size).floor() as i32;
            let new_y = ((transform.translation.y + half_height) / tile_size).floor() as i32;

            // Constrain to grid boundaries
            let constrained_x = new_x.clamp(0, grid_settings.grid_width - 1);
            let constrained_y = new_y.clamp(0, grid_settings.grid_height - 1);

            // If position changed, update trails
            if constrained_x != current_x || constrained_y != current_y {
                // Determine current tile state
                let mut current_is_territory = false;
                let mut current_is_trail = false;
                let mut current_is_owned = false;

                for (_tile_entity, tile, _) in tile_query.iter() {
                    if tile.x == current_x && tile.y == current_y {
                        if tile.owner == Some(entity) {
                            current_is_owned = true;
                            if tile.is_trail {
                                current_is_trail = true;
                            } else {
                                current_is_territory = true;
                            }
                        }
                        break;
                    }
                }

                // Determine new tile state
                let mut new_is_territory = false;
                let mut new_is_owned = false;

                for (_tile_entity, tile, _) in tile_query.iter() {
                    if tile.x == constrained_x && tile.y == constrained_y {
                        if tile.owner == Some(entity) && !tile.is_trail {
                            new_is_territory = true;
                            new_is_owned = true;
                        }
                        break;
                    }
                }

                // Update the tile at the new position
                for (_tile_entity, mut tile, mut sprite) in tile_query.iter_mut() {
                    if tile.x == constrained_x && tile.y == constrained_y {
                        // Case 1: Coming from outside territory (either trail or unowned) and entering territory
                        if !current_is_territory && new_is_territory && player.is_drawing_trail {
                            // Only claim territory if we have a valid trail
                            let mut trail_count = 0;
                            for (_, trail_tile, _) in tile_query.iter() {
                                if trail_tile.is_trail && trail_tile.owner == Some(entity) {
                                    trail_count += 1;
                                    if trail_count >= 5 {
                                        // Complete the trail and claim area
                                        player.is_drawing_trail = false;
                                        println!("Player returned to their territory - claiming enclosed area!");

                                        commands.insert_resource(CompleteTrail {
                                            player: Some(entity),
                                            complete: true,
                                        });
                                        break;
                                    }
                                }
                            }

                            if trail_count < 5 {
                                // Not enough trail tiles to claim, just stop drawing
                                player.is_drawing_trail = false;
                            }
                        }
                        // Case 2: Leaving owned territory and entering unowned area
                        else if current_is_territory && !new_is_owned {
                            player.is_drawing_trail = true;
                            tile.is_trail = true;
                            tile.owner = Some(entity);

                            // Make trail a brighter, more distinct color
                            sprite.color = player.color.with_alpha(0.8);
                            println!("Player left their territory - starting a trail!");
                        }
                        // Case 3: Already drawing a trail and continuing on unowned area
                        else if player.is_drawing_trail && !new_is_owned {
                            tile.is_trail = true;
                            tile.owner = Some(entity);

                            // Keep consistent trail color
                            sprite.color = player.color.with_alpha(0.8);
                        }
                        break;
                    }
                }

                // Snap position to center of tile
                transform.translation.x =
                    (constrained_x as f32 * tile_size) - half_width + (tile_size / 2.0);
                transform.translation.y =
                    (constrained_y as f32 * tile_size) - half_height + (tile_size / 2.0);
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
                });
            }
        }
    }
}
