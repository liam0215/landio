use crate::components::{GridSettings, Player, Tile, Trail};
use crate::events::{PlayerDeathEvent, PlayerDeathReason};
use crate::resources::CompleteTrail;
use bevy::prelude::*;

pub fn start_trail_system(
    grid_settings: Res<GridSettings>,
    mut player_query: Query<(Entity, &Transform, &mut Player)>,
    mut tile_query: Query<(Entity, &mut Tile, &mut Sprite)>,
) {
    let tile_size = grid_settings.tile_size;
    let half_width = (grid_settings.grid_width as f32 * tile_size) / 2.0;
    let half_height = (grid_settings.grid_height as f32 * tile_size) / 2.0;

    for (player_entity, transform, mut player) in player_query.iter_mut() {
        // Only process players who are moving but not yet drawing a trail
        if player.direction.length_squared() > 0.0 && !player.is_drawing_trail {
            // Calculate current grid position
            let current_x = ((transform.translation.x + half_width) / tile_size).floor() as i32;
            let current_y = ((transform.translation.y + half_height) / tile_size).floor() as i32;

            // Check if the current tile is part of the player's territory
            let mut on_own_territory = false;

            for (tile_entity, mut tile, mut sprite) in tile_query.iter_mut() {
                if tile.x == current_x && tile.y == current_y {
                    // If tile is owned by this player but is not a trail
                    if tile.owner == Some(player_entity) && !tile.is_trail {
                        on_own_territory = true;
                    } else if !on_own_territory {
                        // If we're not on our territory, immediately mark this tile as part of the trail
                        // This fixes the off-by-one error when starting a trail
                        player.is_drawing_trail = true;
                        tile.is_trail = true;
                        tile.owner = Some(player_entity);

                        // Trail is more visible (higher alpha)
                        sprite.color = player.color.with_alpha(0.8);
                    }
                    break;
                }
            }

            // Only start drawing a trail if outside own territory
            if !on_own_territory && !player.is_drawing_trail {
                player.is_drawing_trail = true;
                println!("Player left their territory - starting a trail!");
            }
        }
    }
}

// Add points to the trail as player moves
pub fn update_trail_system(
    query: Query<(Entity, &Transform, &Player)>,
    mut trail_query: Query<&mut Trail>,
) {
    for (entity, transform, player) in query.iter() {
        if player.is_drawing_trail {
            let player_pos = Vec2::new(transform.translation.x, transform.translation.y);

            // Find the active trail belonging to this player
            for mut trail in trail_query.iter_mut() {
                if trail.owner == entity && trail.is_active {
                    let last_point = trail.points.last().unwrap_or(&Vec2::ZERO);

                    // Only add points if we've moved far enough (prevents too many points)
                    if last_point.distance(player_pos) > 5.0 {
                        trail.points.push(player_pos);
                    }

                    break;
                }
            }
        }
    }
}

// Render the trails
pub fn render_trail_system(
    mut commands: Commands,
    trail_query: Query<(Entity, &Trail)>,
    player_query: Query<&Player>,
) {
    for (trail_entity, trail) in trail_query.iter() {
        // If trail has at least 2 points (to form a line)
        if trail.points.len() >= 2 {
            // Get the trail owner's color
            let player_color = if let Ok(player) = player_query.get(trail.owner) {
                player.color
            } else {
                // Default color if player not found
                Color::srgb(1.0, 0.0, 0.0)
            };

            // First clear any existing children
            commands.entity(trail_entity).clear_children();

            // Then add new children using with_children
            commands.entity(trail_entity).with_children(|parent| {
                // Create line segments for the trail
                for i in 1..trail.points.len() {
                    let start = trail.points[i - 1];
                    let end = trail.points[i];

                    // Calculate segment properties
                    let segment_dir = (end - start).normalize_or_zero();
                    let segment_length = start.distance(end);
                    let segment_center = start + segment_dir * (segment_length / 2.0);
                    let angle = segment_dir.y.atan2(segment_dir.x);

                    // Spawn line segment directly as a child
                    parent.spawn((
                        Sprite {
                            color: player_color,
                            custom_size: Some(Vec2::new(segment_length, 3.0)), // 3 pixels wide
                            ..default()
                        },
                        Transform {
                            translation: Vec3::new(segment_center.x, segment_center.y, 0.1),
                            rotation: Quat::from_rotation_z(angle),
                            ..default()
                        },
                        GlobalTransform::default(),
                        Visibility::default(),
                        InheritedVisibility::default(),
                        ViewVisibility::default(),
                    ));
                }
            });
        }
    }
}

// The main territory claiming system - uses flood fill to accurately determine
// which tiles are inside the enclosed area
pub fn claim_territory_system(
    grid_settings: Res<GridSettings>,
    complete_trail: Option<ResMut<CompleteTrail>>,
    mut death_events: EventWriter<PlayerDeathEvent>,
    mut player_query: Query<(Entity, &mut Player)>,
    mut tile_query: Query<(Entity, &mut Tile, &mut Sprite)>,
) {
    // Only process if we have a completed trail
    if let Some(mut trail_info) = complete_trail {
        if !trail_info.complete || trail_info.player.is_none() {
            return;
        }

        let player_entity = trail_info.player.unwrap();
        let entry_point = trail_info.entry_point; // Get the entry point
        trail_info.complete = false; // Reset flag

        println!("============ TERRITORY CLAIMING STARTED ============");

        // If no entry point, it's a pure loop by crossing own trail - PLAYER DIES
        if entry_point.is_none() {
            // Send a player death event instead of directly handling it here
            death_events.send(PlayerDeathEvent {
                player_entity,
                reason: PlayerDeathReason::CrossedTrail,
            });

            return;
        }

        // We have an entry point, so the player completed loop by returning to territory
        let (entry_x, entry_y) = entry_point.unwrap();
        println!(
            "Player completed loop by returning to territory at ({}, {})",
            entry_x, entry_y
        );

        let grid_width = grid_settings.grid_width as usize;
        let grid_height = grid_settings.grid_height as usize;

        // Step 1: Create grid representation
        let mut grid = vec![vec![0; grid_width]; grid_height]; // 0=empty, 1=player territory, 2=trail, 3=other player
        let mut tile_entities = vec![vec![None; grid_width]; grid_height];

        // Collect all trail and territory tiles
        let mut trail_coords = Vec::new();
        let mut territory_coords = Vec::new();

        for (tile_entity, tile, _) in tile_query.iter() {
            if tile.x >= 0
                && tile.x < grid_width as i32
                && tile.y >= 0
                && tile.y < grid_height as i32
            {
                let x = tile.x as usize;
                let y = tile.y as usize;

                tile_entities[y][x] = Some(tile_entity);

                if tile.owner == Some(player_entity) {
                    if tile.is_trail {
                        grid[y][x] = 2; // Trail
                        trail_coords.push((x, y));
                    } else {
                        grid[y][x] = 1; // Player territory
                        territory_coords.push((x, y));
                    }
                } else if tile.owner.is_some() {
                    grid[y][x] = 3; // Other player territory
                }
            }
        }

        println!(
            "Found {} trail tiles and {} territory tiles",
            trail_coords.len(),
            territory_coords.len()
        );

        // Step 2: Find all potentially enclosed areas
        let mut fill_grid = vec![vec![false; grid_width]; grid_height];

        // Mark all non-empty cells as visited
        for y in 0..grid_height {
            for x in 0..grid_width {
                if grid[y][x] > 0 {
                    fill_grid[y][x] = true;
                }
            }
        }

        // Flood fill from the edges to mark outside areas
        let mut queue = Vec::new();

        // Start from edges
        for x in 0..grid_width {
            if !fill_grid[0][x] {
                queue.push((x, 0));
                fill_grid[0][x] = true;
            }
            if !fill_grid[grid_height - 1][x] {
                queue.push((x, grid_height - 1));
                fill_grid[grid_height - 1][x] = true;
            }
        }

        for y in 1..grid_height - 1 {
            if !fill_grid[y][0] {
                queue.push((0, y));
                fill_grid[y][0] = true;
            }
            if !fill_grid[y][grid_width - 1] {
                queue.push((grid_width - 1, y));
                fill_grid[y][grid_width - 1] = true;
            }
        }

        // Perform flood fill
        while let Some((x, y)) = queue.pop() {
            let neighbors = [
                (x + 1, y),
                (x.wrapping_sub(1), y),
                (x, y + 1),
                (x, y.wrapping_sub(1)),
            ];

            for (nx, ny) in neighbors {
                if nx < grid_width && ny < grid_height && !fill_grid[ny][nx] {
                    fill_grid[ny][nx] = true;
                    queue.push((nx, ny));
                }
            }
        }

        // Step 3: Collect all enclosed tiles
        let mut enclosed_tiles = Vec::new();

        for y in 0..grid_height {
            for x in 0..grid_width {
                if grid[y][x] == 0 && !fill_grid[y][x] {
                    enclosed_tiles.push((x, y));
                }
            }
        }

        println!("Found {} enclosed tiles", enclosed_tiles.len());

        // Always convert trail to territory (happens regardless of enclosed area)
        let player_color = player_query
            .get(player_entity)
            .map_or(Color::srgba(0.5, 0.5, 0.5, 1.0), |(_, p)| p.color);
        let territory_color = player_color.with_alpha(0.5);

        for (_, mut tile, mut sprite) in tile_query.iter_mut() {
            if tile.is_trail && tile.owner == Some(player_entity) {
                tile.is_trail = false;
                sprite.color = territory_color;
            }
        }

        // If there are no enclosed tiles, stop here
        if enclosed_tiles.is_empty() {
            println!("No enclosed tiles found. Claiming process ended.");
            println!("============ TERRITORY CLAIMING ENDED ============");
            return;
        }

        // Step 5: Claim all enclosed tiles (since the loop was completed by returning to territory)
        let mut claimed_count = 0;

        for &(x, y) in &enclosed_tiles {
            if let Some(entity) = tile_entities[y][x] {
                if let Ok((_, mut tile, mut sprite)) = tile_query.get_mut(entity) {
                    tile.owner = Some(player_entity);
                    tile.is_trail = false;
                    sprite.color = territory_color;
                    claimed_count += 1;
                }
            }
        }

        // Update player score
        if claimed_count > 0 {
            if let Ok((_, mut player)) = player_query.get_mut(player_entity) {
                player.score += claimed_count;
                println!(
                    "Player claimed {} tiles. Total score: {}",
                    claimed_count, player.score
                );
            }
            println!(
                "Territory claimed successfully by returning to territory at ({}, {})",
                entry_x, entry_y
            );
        }

        println!("============ TERRITORY CLAIMING ENDED ============");
    }
}
