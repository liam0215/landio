use crate::components::{GridSettings, Player, Tile, Trail};
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
        // Skip if player is not moving
        if player.direction.length_squared() == 0.0 {
            continue;
        }

        // Calculate current grid position
        let current_x = ((transform.translation.x + half_width) / tile_size).floor() as i32;
        let current_y = ((transform.translation.y + half_height) / tile_size).floor() as i32;

        // Calculate the next tile based on player direction
        let next_dir = player.direction.normalize();
        let next_x = current_x + next_dir.x.round() as i32;
        let next_y = current_y + next_dir.y.round() as i32;

        // Check if current tile is territory (owned by player, not a trail)
        let mut current_is_territory = false;
        let mut next_is_territory = false;

        // Check current tile
        for (_, tile, _) in tile_query.iter() {
            if tile.x == current_x && tile.y == current_y {
                if tile.owner == Some(player_entity) && !tile.is_trail {
                    current_is_territory = true;
                }
                break;
            }
        }

        // Check next tile
        if next_x >= 0
            && next_x < grid_settings.grid_width
            && next_y >= 0
            && next_y < grid_settings.grid_height
        {
            for (_, tile, _) in tile_query.iter() {
                if tile.x == next_x && tile.y == next_y {
                    if tile.owner == Some(player_entity) && !tile.is_trail {
                        next_is_territory = true;
                    }
                    break;
                }
            }
        }

        // CASE 1: Player is on territory and about to leave territory
        if current_is_territory && !next_is_territory && !player.is_drawing_trail {
            // Set the flag to start drawing trail on the NEXT tile
            player.is_drawing_trail = true;
            println!(
                "Player is leaving territory - will start trail on next tile at ({}, {})",
                next_x, next_y
            );
        }
        // CASE 2: Player is not on territory and not drawing trail yet
        // This handles the case where they might have teleported or spawned outside territory
        else if !current_is_territory && !player.is_drawing_trail {
            player.is_drawing_trail = true;

            // Immediately mark the current tile as a trail
            for (_, mut tile, mut sprite) in tile_query.iter_mut() {
                if tile.x == current_x && tile.y == current_y {
                    tile.is_trail = true;
                    tile.owner = Some(player_entity);
                    sprite.color = player.color.with_alpha(0.8);

                    println!(
                        "Started trail at current position ({}, {})",
                        current_x, current_y
                    );
                    break;
                }
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
    mut player_query: Query<(Entity, &mut Player)>,
    mut tile_query: Query<(Entity, &mut Tile, &mut Sprite)>,
) {
    // Only process if we have a completed trail
    if let Some(mut trail_info) = complete_trail {
        if !trail_info.complete || trail_info.player.is_none() {
            return;
        }

        let player_entity = trail_info.player.unwrap();
        let entry_point = trail_info.entry_point;

        // Reset the flag to prevent processing multiple times
        trail_info.complete = false;
        trail_info.player = None;
        trail_info.entry_point = None;

        // We must have an entry point for territory claiming
        if entry_point.is_none() {
            println!("No entry point specified for territory claiming, aborting.");
            return;
        }

        let (entry_x, entry_y) = entry_point.unwrap();
        println!("============ TERRITORY CLAIMING STARTED ============");
        println!(
            "Player completed loop by returning to territory at ({}, {})",
            entry_x, entry_y
        );

        let grid_width = grid_settings.grid_width as usize;
        let grid_height = grid_settings.grid_height as usize;

        // Step 1: Create grid representation
        #[derive(Clone, Copy, PartialEq)]
        enum CellType {
            Empty,
            PlayerTerritory,
            PlayerTrail,
            Other,
        }

        let mut grid = vec![vec![CellType::Empty; grid_width]; grid_height];
        let mut tile_entities = vec![vec![None; grid_width]; grid_height];

        // Fill the grid with current tile state
        for (tile_entity, tile, _) in tile_query.iter() {
            if tile.x >= 0
                && tile.x < grid_settings.grid_width
                && tile.y >= 0
                && tile.y < grid_settings.grid_height
            {
                let x = tile.x as usize;
                let y = tile.y as usize;

                tile_entities[y][x] = Some(tile_entity);

                if tile.owner == Some(player_entity) {
                    if tile.is_trail {
                        grid[y][x] = CellType::PlayerTrail;
                    } else {
                        grid[y][x] = CellType::PlayerTerritory;
                    }
                } else if tile.owner.is_some() {
                    grid[y][x] = CellType::Other;
                }
            }
        }

        // Step 2: Convert all trail tiles to territory
        let mut trail_count = 0;

        for y in 0..grid_height {
            for x in 0..grid_width {
                if grid[y][x] == CellType::PlayerTrail {
                    grid[y][x] = CellType::PlayerTerritory;
                    trail_count += 1;
                }
            }
        }

        println!("Converting {} trail tiles to territory", trail_count);

        // Step 3: Find all potentially enclosed areas
        let mut fill_grid = vec![vec![false; grid_width]; grid_height];

        // Mark all non-empty cells as visited
        for y in 0..grid_height {
            for x in 0..grid_width {
                if grid[y][x] != CellType::Empty {
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
                (x.wrapping_add(1), y),
                (x.wrapping_sub(1), y),
                (x, y.wrapping_add(1)),
                (x, y.wrapping_sub(1)),
            ];

            for (nx, ny) in neighbors {
                if nx < grid_width && ny < grid_height && !fill_grid[ny][nx] {
                    fill_grid[ny][nx] = true;
                    queue.push((nx, ny));
                }
            }
        }

        // Step 4: Collect all enclosed tiles
        let mut enclosed_tiles = Vec::new();

        for y in 0..grid_height {
            for x in 0..grid_width {
                if grid[y][x] == CellType::Empty && !fill_grid[y][x] {
                    enclosed_tiles.push((x, y));
                }
            }
        }

        println!("Found {} enclosed tiles", enclosed_tiles.len());

        // Step 5: Claim enclosed tiles and update trail tiles
        let player_color = player_query
            .get(player_entity)
            .map_or(Color::srgba(0.5, 0.5, 0.5, 1.0), |(_, p)| p.color);

        let territory_color = player_color.with_alpha(0.5);
        let mut claimed_count = 0;

        for (_, mut tile, mut sprite) in tile_query.iter_mut() {
            // First, convert all trail tiles to territory
            if tile.owner == Some(player_entity) && tile.is_trail {
                tile.is_trail = false;
                sprite.color = territory_color;
            }

            // Then claim enclosed tiles
            let tile_pos = (tile.x as usize, tile.y as usize);
            if enclosed_tiles.contains(&tile_pos) {
                tile.owner = Some(player_entity);
                tile.is_trail = false;
                sprite.color = territory_color;
                claimed_count += 1;
            }
        }

        // Update player score
        if let Ok((_, mut player)) = player_query.get_mut(player_entity) {
            player.score += claimed_count;
            println!(
                "Player claimed {} tiles. Total score: {}",
                claimed_count, player.score
            );
        }

        println!("============ TERRITORY CLAIMING ENDED ============");
    }
}
