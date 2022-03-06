use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use educe::Educe;

pub mod strategy;
use strategy::*;

#[derive(Debug, Default, Component, Educe)]
#[educe(Deref, DerefMut)]
pub struct FlockMembers(pub smallvec::SmallVec<[Entity; 8]>);

#[derive(Bundle)]
pub struct FlockMindBundle {
    pub members: FlockMembers,
    // pub events: FlockChangeEvents,
    // smarts layer coordination
    pub active_strategy: ActiveFlockStrategy,
}

impl FlockMindBundle {
    pub fn new(strategy: Entity) -> Self {
        Self {
            members: Default::default(),
            // events: Default::default(),
            active_strategy: ActiveFlockStrategy { strategy },
        }
    }
}

#[derive(Debug, Clone, Copy, Component)]
pub struct CraftFlock(pub Entity);
/*
#[derive(Debug, Default, Component)]
pub struct FlockChangeEvents {
    pub events: smallvec::SmallVec<[FlockChange; 2]>,
}

#[derive(Debug, Component)]
pub enum FlockChange {
    MemberChange {},
}

pub fn flock_change_event_emitter(
    mut flocks: Query<&mut FlockChangeEvents>
) */

/* use formation::*;
pub mod formation {
    use deps::*;

    use bevy::{ecs as bevy_ecs, prelude::*};
}
 */
