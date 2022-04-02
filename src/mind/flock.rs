use deps::*;

use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;
use educe::Educe;

use crate::mind::*;

pub mod strategy;
use strategy::*;
pub mod formation;

#[derive(Bundle)]
pub struct FlockMindBundle {
    pub members: FlockMembers,
    pub change_events: FlockChangeEvents,
    // smarts layer coordination
    pub active_strategy: CurrentFlockStrategy,
    pub active_formation: CurrentFlockFormation,
    pub directive: FlockMindDirective,
}

impl FlockMindBundle {
    pub fn new(members: FlockMembers, formation_entt: Entity) -> Self {
        Self {
            members,
            active_strategy: Default::default(),
            active_formation: CurrentFlockFormation {
                formation: formation_entt,
            },
            directive: Default::default(),
            change_events: Default::default(),
        }
    }
}

#[derive(Debug, Default, Component, Clone, Educe)]
#[educe(Deref)]
pub struct FlockMembers {
    #[educe(Deref)]
    members: smallvec::SmallVec<[Entity; 8]>,
    added: smallvec::SmallVec<[Entity; 4]>,
    removed: smallvec::SmallVec<[Entity; 4]>,
}

impl FlockMembers {
    #[inline]
    pub fn push(&mut self, value: Entity) {
        self.members.push(value);
        self.added.push(value);
    }

    #[inline]
    pub fn remove(&mut self, entt_to_remove: Entity) -> bool {
        for (ii, entt) in self.members.iter().enumerate() {
            if *entt == entt_to_remove {
                self.removed.push(entt_to_remove);
                self.members.swap_remove(ii);
                return true;
            }
        }
        false
    }

    #[inline]
    fn added(&mut self) -> smallvec::Drain<[Entity; 4]> {
        self.added.drain(0..self.added.len())
    }

    #[inline]
    fn removed(&mut self) -> smallvec::Drain<[Entity; 4]> {
        self.removed.drain(0..self.removed.len())
    }
}

#[derive(Debug, Clone, Component, Reflect, Inspectable)]
pub struct CurrentFlockFormation {
    pub formation: Entity,
}

#[derive(Debug, Clone, Component, Educe)]
#[educe(Default)]
pub enum FlockMindDirective {
    #[educe(Default)]
    None,
    CAS,
    FormUp {
        leader_directive: Option<boid::BoidMindDirective>,
    },
    JoinFomation {
        formation: Entity,
    },
}

pub fn flock_mind(
    mut commands: Commands,
    mut minds: Query<
        (Entity, &FlockMindDirective, &mut CurrentFlockStrategy),
        (Changed<FlockMindDirective>, Added<FlockMindDirective>),
    >,
) {
    for (flock_entt, directive, mut cur_stg) in minds.iter_mut() {
        if let Some(cur_stg) = cur_stg.strategy.take() {
            commands.entity(cur_stg).despawn_recursive();
        }
        cur_stg.strategy = match directive {
            FlockMindDirective::None => None,
            FlockMindDirective::FormUp { leader_directive } => Some(
                commands
                    .spawn()
                    .insert_bundle(strategy::form_up::Bundle::new(
                        strategy::form_up::FormUp {
                            leader_directive: leader_directive.clone(),
                        },
                        flock_entt,
                        Default::default(),
                    ))
                    .id(),
            ),
            FlockMindDirective::CAS => Some(
                commands
                    .spawn()
                    .insert_bundle(strategy::cas::Bundle::new(
                        strategy::cas::CAS {},
                        flock_entt,
                        Default::default(),
                    ))
                    .id(),
            ),
            FlockMindDirective::JoinFomation { .. } => {
                todo!()
            }
        };
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

pub fn flock_change_event_emitter(
    mut flocks: Query<&mut FlockChangeEvents>
) */

// pub type FlockCrossRefIndex = sensors::CrossReferenceIndex<FlockMembers>;

#[derive(Debug)]
pub enum FlockChangeEvent {
    MemberAdded { entt: Entity },
    MemberRemoved { entt: Entity },
}

#[derive(Debug, Default, Component, Educe)]
#[educe(Deref, DerefMut)]
pub struct FlockChangeEvents(bevy::ecs::event::Events<FlockChangeEvent>);

#[derive(Default, Component, Educe)]
#[educe(Deref, DerefMut)]
pub struct FlockChangeEventsReader(bevy::ecs::event::ManualEventReader<FlockChangeEvent>);

pub fn flock_members_change_listener(
    // new: Query<(Entity, &FlockMembers), Added<FlockMembers>>,
    mut queries: QuerySet<(
        // all
        QueryState<(&mut FlockChangeEvents,)>,
        // changed
        QueryState<(&mut FlockMembers, &mut FlockChangeEvents), Changed<FlockMembers>>,
    )>,
    // mut crafts: Query<(&mut boid::BoidMindDirective,)>,
    // mut cross_ref_index: ResMut<FlockCrossRefIndex>,
    // removed: RemovedComponents<FlockMembers>,
) {
    /* for (entt, members) in new.iter() {
        // add them to the global index
        cross_ref_index.insert(entt, members.clone());
    } */
    for (mut members, mut events) in queries.q1().iter_mut() {
        for removed in members.removed() {
            events.send(FlockChangeEvent::MemberRemoved { entt: removed });
        }
        for added in members.added() {
            events.send(FlockChangeEvent::MemberAdded { entt: added });
        }
    }
    for (mut events,) in queries.q0().iter_mut() {
        events.update()
    }
    /* for entt in removed.iter() {
        cross_ref_index.remove(&entt);
    } */
}
