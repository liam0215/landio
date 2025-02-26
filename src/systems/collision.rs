use crate::components::{Player, Trail};
use crate::resources::GameState;
use bevy::prelude::*;

pub fn collision_detection_system(
    mut commands: Commands,
    player_query: Query<(Entity, &Transform, &Player)>,
    trail_query: Query<(Entity, &Trail)>,
    _game_state: ResMut<GameState>,
) {
    for (player_entity, player_transform, _player) in player_query.iter() {
        let player_pos = Vec2::new(
            player_transform.translation.x,
            player_transform.translation.y,
        );

        for (trail_entity, trail) in trail_query.iter() {
            // Skip collision with own recent trail points (to prevent self-collision at start)
            if trail.owner == player_entity {
                if trail.is_active && trail.points.len() < 10 {
                    continue;
                }
            }

            // Check for collision with trail segments
            for i in 1..trail.points.len() {
                let segment_start = trail.points[i - 1];
                let segment_end = trail.points[i];

                if line_point_distance(segment_start, segment_end, player_pos) < 5.0 {
                    // Collision detected!
                    if trail.owner == player_entity {
                        // Self-collision - check if we can claim territory
                        // This is handled in check_territory_claim_system
                    } else {
                        // Hit another player's trail - reset player
                        commands.entity(trail_entity).despawn();

                        // This would typically reset the player or implement your game's
                        // consequences for hitting another player's trail
                    }
                }
            }
        }
    }
}

// Helper function to calculate distance from a point to a line segment
fn line_point_distance(a: Vec2, b: Vec2, p: Vec2) -> f32 {
    let ab = b - a;
    let ap = p - a;

    if ab.dot(ap) <= 0.0 {
        return ap.length();
    }

    let bp = p - b;
    if ab.dot(bp) >= 0.0 {
        return bp.length();
    }

    let projection = ab.dot(ap) / ab.length_squared();
    let closest = a + ab * projection;
    (p - closest).length()
}
