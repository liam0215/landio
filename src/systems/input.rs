use crate::components::Player;
use bevy::prelude::*;

pub fn player_input_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Player>,
) {
    if let Ok(mut player) = query.get_single_mut() {
        // Start with no direction
        let mut new_direction = Vec2::ZERO;

        // Process only cardinal directions - no diagonals allowed
        // Priority order: right > left > down > up (later ones override earlier ones)
        // You can change this order if you prefer a different priority

        if keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp) {
            new_direction = Vec2::new(0.0, 1.0);
        }

        if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown) {
            new_direction = Vec2::new(0.0, -1.0);
        }

        if keyboard_input.pressed(KeyCode::KeyA) || keyboard_input.pressed(KeyCode::ArrowLeft) {
            new_direction = Vec2::new(-1.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight) {
            new_direction = Vec2::new(1.0, 0.0);
        }

        // Only update direction if there's input
        if new_direction != Vec2::ZERO {
            // Check if the new direction is opposite to the current direction
            let current_dir = player.direction;
            let is_opposite = (current_dir.x != 0.0 && new_direction.x == -current_dir.x)
                || (current_dir.y != 0.0 && new_direction.y == -current_dir.y);

            // Don't allow direct reversals
            if is_opposite {
                // Ignore the reversal attempt
                return;
            }

            // If the player is currently moving to the next tile, buffer the direction change
            if player.is_moving_to_next_tile && current_dir != Vec2::ZERO {
                player.buffered_direction = Some(new_direction);
            } else {
                // Otherwise, apply the direction immediately
                player.direction = new_direction;
                player.buffered_direction = None;
            }
        }
    }
}
