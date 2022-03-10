use deps::*;

use bevy::{ prelude::*};
use educe::Educe;

use crate::math::*;

pub mod strategy;
use strategy::*;
pub mod formation;

#[derive(Debug, Default, Component, Educe)]
#[educe(Deref, DerefMut)]
pub struct FlockMembers(pub smallvec::SmallVec<[Entity; 8]>);

#[derive(Bundle, Default)]
pub struct FlockMindBundle {
    pub members: FlockMembers,
    // pub events: FlockChangeEvents,
    // smarts layer coordination
    pub active_strategy: CurrentFlockStrategy,
    pub directive: FlockMindDirective,
}

#[derive(Debug, Clone, Component, Educe)]
#[educe(Default)]
pub enum FlockMindDirective {
    #[educe(Default)]
    None,
    HoldPosition {
        pos: TVec3,
        formation: Entity,
    },
    JoinFomation {
        formation: Entity,
    },
}

pub fn flock_mind(
    mut commands: Commands,
    mut minds: Query<
        (Entity, &FlockMindDirective, &mut CurrentFlockStrategy),
        Changed<FlockMindDirective>,
    >,
) {
    for (flock_entt, directive, mut cur_stg) in minds.iter_mut() {
        if let Some(cur_stg) = cur_stg.strategy.take() {
            commands.entity(cur_stg).despawn_recursive();
        }
        cur_stg.strategy = match directive {
            FlockMindDirective::None => None,
            FlockMindDirective::HoldPosition { pos, formation } => {
                let pos = *pos;
                let formation = *formation;

                Some(
                    commands
                        .spawn()
                        .insert_bundle(strategy::hold::Bundle::new(
                            strategy::hold::Hold { pos, formation },
                            flock_entt,
                            Default::default(),
                        ))
                        .id(),
                )
            }
            FlockMindDirective::JoinFomation { .. } => {
                todo!()
            }
        }
    }
}
/*
#[derive(Debug, Clone, Copy, Component)]
pub struct CraftFlock(pub Entity); */
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
