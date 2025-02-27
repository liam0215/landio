use bevy::prelude::*;

// Event that gets triggered when a player should be killed and respawned
#[derive(Event)]
pub struct PlayerDeathEvent {
    pub player_entity: Entity,
    pub reason: PlayerDeathReason,
}

// Enum to track the reason for player death
#[derive(Debug, Clone, Copy)]
pub enum PlayerDeathReason {
    TrailCollision, // Player hit their own trail
    CrossedTrail,   // Player crossed their trail without returning to territory
    OutOfBounds,    // Player went out of bounds
    HitOtherPlayer, // Player collided with another player
}
