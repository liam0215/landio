use crate::components::{GridSettings, Player, Territory, Tile, Trail};
use crate::resources::CompleteTrail;
use bevy::prelude::*;

// Start drawing a trail when player moves out of their territory
// In start_trail_system:
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
                        sprite.color = player.color.with_alpha(0.5);
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

pub fn check_loop_system(
    grid_settings: Res<GridSettings>,
    mut commands: Commands,
    mut player_query: Query<(Entity, &mut Player)>,
    mut tile_query: Query<(Entity, &mut Tile, &mut Sprite)>,
) {
    let grid_width = grid_settings.grid_width;
    let grid_height = grid_settings.grid_height;

    // First, create a grid representation of all tiles and their states
    let mut tile_grid = vec![vec![None; grid_width as usize]; grid_height as usize];
    let mut tile_entities = vec![vec![None; grid_width as usize]; grid_height as usize];

    // Fill the grid with current tile state
    for (tile_entity, tile, _) in tile_query.iter() {
        if tile.x >= 0 && tile.x < grid_width && tile.y >= 0 && tile.y < grid_height {
            tile_grid[tile.y as usize][tile.x as usize] = Some((tile.owner, tile.is_trail));
            tile_entities[tile.y as usize][tile.x as usize] = Some(tile_entity);
        }
    }

    // For each player
    for (player_entity, mut player) in player_query.iter_mut() {
        if !player.is_drawing_trail {
            continue;
        }

        // Find all trail tiles for this player
        let mut trail_points = Vec::new();
        for y in 0..grid_height as usize {
            for x in 0..grid_width as usize {
                if let Some((owner, is_trail)) = tile_grid[y][x] {
                    if is_trail && owner == Some(player_entity) {
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
                        if is_trail && owner == Some(player_entity) {
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
            println!("Loop detected! Claiming territory...");
            player.is_drawing_trail = false;

            // Find tiles adjacent to trail
            let mut tiles_to_claim = Vec::new();

            for &(tx, ty) in &trail_points {
                let neighbors = [(tx + 1, ty), (tx - 1, ty), (tx, ty + 1), (tx, ty - 1)];

                for &(nx, ny) in &neighbors {
                    if nx >= 0 && nx < grid_width && ny >= 0 && ny < grid_height {
                        // If tile exists and is not already owned by this player
                        if let Some((owner, is_trail)) = tile_grid[ny as usize][nx as usize] {
                            if !is_trail && owner != Some(player_entity) {
                                if let Some(entity) = tile_entities[ny as usize][nx as usize] {
                                    tiles_to_claim.push(entity);
                                }
                            }
                        }
                    }
                }
            }

            // Now convert trails to borders
            let mut trail_tiles_to_convert = Vec::new();
            for &(tx, ty) in &trail_points {
                if let Some(entity) = tile_entities[ty as usize][tx as usize] {
                    trail_tiles_to_convert.push(entity);
                }
            }

            // Now apply all changes
            let claim_count = tiles_to_claim.len();

            // Claim adjacent tiles
            for tile_entity in tiles_to_claim {
                if let Ok((_, mut tile, mut sprite)) = tile_query.get_mut(tile_entity) {
                    tile.owner = Some(player_entity);
                    tile.is_trail = false;
                    sprite.color = player.color.with_alpha(0.3);
                }
            }

            // Convert trail to territory border
            for tile_entity in trail_tiles_to_convert {
                if let Ok((_, mut tile, mut sprite)) = tile_query.get_mut(tile_entity) {
                    tile.is_trail = false;
                    sprite.color = player.color.with_alpha(0.7);
                }
            }

            // Update player score
            player.score += claim_count as u32;
            println!(
                "Player claimed {} tiles. Total score: {}",
                claim_count, player.score
            );
        }
    }
}

pub fn claim_territory_system(
    _commands: Commands,
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
        trail_info.complete = false; // Reset flag

        let grid_width = grid_settings.grid_width as usize;
        let grid_height = grid_settings.grid_height as usize;

        // Step 1: Create grid representation
        let mut grid = vec![vec![0; grid_width]; grid_height]; // 0=empty, 1=player territory, 2=trail, 3=other player
        let mut tile_entities = vec![vec![None; grid_width]; grid_height];

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
                    } else {
                        grid[y][x] = 1; // Player territory
                    }
                } else if tile.owner.is_some() {
                    grid[y][x] = 3; // Other player territory
                }
            }
        }

        // Step 2: Create a new grid for the flood fill
        let mut fill_grid = vec![vec![false; grid_width]; grid_height];

        // Mark all territory and trail as unavailable for flood fill
        for y in 0..grid_height {
            for x in 0..grid_width {
                if grid[y][x] > 0 {
                    // Any non-empty cell
                    fill_grid[y][x] = true;
                }
            }
        }

        // Step 3: Flood fill from all edges to mark "outside" area
        let mut queue = Vec::new();

        // Start from all edge cells
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

        // Step 4 debug
        let mut outside_count = 0;
        let mut inside_count = 0;
        let mut trail_count = 0;
        let mut territory_count = 0;

        for y in 0..grid_height {
            for x in 0..grid_width {
                if grid[y][x] == 0 {
                    if fill_grid[y][x] {
                        outside_count += 1;
                    } else {
                        inside_count += 1;
                    }
                } else if grid[y][x] == 1 {
                    territory_count += 1;
                } else if grid[y][x] == 2 {
                    trail_count += 1;
                }
            }
        }

        println!(
            "Grid stats: {} outside, {} inside, {} trail, {} territory",
            outside_count, inside_count, trail_count, territory_count
        );
        // Step 4: Claim all cells not reached by the flood fill (they're inside the territory)
        let player_color = if let Ok((_, player)) = player_query.get(player_entity) {
            player.color
        } else {
            Color::srgba(0.5, 0.5, 0.5, 1.0)
        };

        let mut claimed_count = 0;

        for y in 0..grid_height {
            for x in 0..grid_width {
                // If cell is empty (0) and not marked by flood fill, it's inside
                if grid[y][x] == 0 && !fill_grid[y][x] {
                    if let Some(entity) = tile_entities[y][x] {
                        if let Ok((_, mut tile, mut sprite)) = tile_query.get_mut(entity) {
                            tile.owner = Some(player_entity);
                            tile.is_trail = false;
                            sprite.color = player_color.with_alpha(0.3);
                            claimed_count += 1;
                        }
                    }
                }
            }
        }

        // Step 5: Convert all trail tiles to territory
        for (_, mut tile, mut sprite) in tile_query.iter_mut() {
            if tile.is_trail && tile.owner == Some(player_entity) {
                tile.is_trail = false;
                sprite.color = player_color.with_alpha(0.7);
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
    }
}

// This is a simplified version of territory claiming
// In a real implementation, this would use proper geometry calculations
pub fn check_territory_claim_system(
    mut commands: Commands,
    trail_query: Query<(Entity, &Trail)>,
    mut player_query: Query<(Entity, &mut Player)>,
) {
    for (trail_entity, trail) in trail_query.iter() {
        if !trail.is_active {
            continue;
        }

        // Check if trail forms a loop (simplified)
        if trail.points.len() > 10 {
            let first_point = trail.points.first().unwrap();
            let last_point = trail.points.last().unwrap();

            // If start and end points are close, consider it a loop
            if first_point.distance(*last_point) < 5.0 {
                // Get the player who owns this trail
                if let Ok((player_entity, mut player)) = player_query.get_mut(trail.owner) {
                    // Calculate territory area (simplified)
                    let approx_area = calculate_polygon_area(&trail.points);

                    // Create territory
                    commands.spawn(Territory {
                        owner: player_entity,
                        polygon: trail.points.clone(),
                        area: approx_area,
                    });

                    // Update player score
                    player.score += approx_area as u32;
                    player.is_drawing_trail = false;

                    // Remove the trail
                    commands.entity(trail_entity).despawn();
                }
            }
        }
    }
}

// Helper function to calculate polygon area using Shoelace formula
fn calculate_polygon_area(points: &[Vec2]) -> f32 {
    if points.len() < 3 {
        return 0.0;
    }

    let mut area = 0.0;
    let n = points.len();

    for i in 0..n {
        let j = (i + 1) % n;
        area += points[i].x * points[j].y;
        area -= points[j].x * points[i].y;
    }

    (area / 2.0).abs()
}
