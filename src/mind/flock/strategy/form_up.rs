use deps::*;

use bevy::prelude::*;

use crate::mind::{
    flock::{
        formation,
        strategy::{FlockStrategy, FlockStrategyBundleJumbo},
        ActiveFlockStrategy, CurrentFlockFormation, FlockChangeEvent, FlockChangeEvents,
        FlockChangeEventsReader, FlockMembers,
    },
    *,
};

#[derive(Debug, Clone, Component)]
pub struct FormUp {
    pub leader_directive: Option<boid::BoidMindDirective>,
}

#[derive(Debug, Clone, Component, Default)]
pub struct FormUpState {
    leader: Option<Entity>,
}

pub type Bundle = FlockStrategyBundleJumbo<FormUp, (FormUpState, FlockChangeEventsReader)>;

// FIXME: this assumes the center pivot is shadowing a leader craft
pub fn butler(
    mut commands: Commands,
    mut strategies: ParamSet<(
        // new
        Query<
            (
                Entity,
                &FlockStrategy,
                &FormUp,
                &mut FlockChangeEventsReader,
                &mut FormUpState,
            ),
            (Added<FormUp>, With<FormUp>),
        >,
        Query<
            (
                &FlockStrategy,
                &mut FlockChangeEventsReader,
                &FormUp,
                &mut FormUpState,
            ),
            With<FormUp>,
        >, // all
    )>,
    flocks: Query<(&FlockMembers, &FlockChangeEvents, &CurrentFlockFormation)>,
    formations: Query<(&formation::FormationCenterPivot,)>,
    mut crafts: Query<(&mut boid::BoidMindDirective,)>,
    // member_changes: Query<(Entity, &FlockMembers), Changed<FlockMembers>>
) {
    for (strategy_entt, strategy, param, mut reader, mut state) in strategies.p0().iter_mut() {
        let (members, events, cur_formation) = flocks.get(strategy.flock_entt()).unwrap_or_log();
        // we're not interested in any events before activation
        let _ = reader.iter(events).skip_while(|_| true);

        let (center_pivot,) = formations.get(cur_formation.formation).unwrap_or_log();
        for member in members.iter().filter(|e| **e != center_pivot.boid_entt()) {
            let (mut directive,) = crafts.get_mut(*member).unwrap_or_log();
            *directive = boid::BoidMindDirective::JoinFomation {
                formation: cur_formation.formation,
            };
        }
        if let Some(leader_directive) = &param.leader_directive {
            state.leader = Some(center_pivot.boid_entt());
            let (mut directive,) = crafts.get_mut(center_pivot.boid_entt()).unwrap_or_log();
            *directive = leader_directive.clone();
        }

        commands.entity(strategy_entt).insert(ActiveFlockStrategy);
    }

    for (strategy, mut reader, param, mut state) in strategies.p1().iter_mut() {
        let (_, events, cur_formation) = flocks.get(strategy.flock_entt()).unwrap_or_log();
        let (center_pivot,) = formations.get(cur_formation.formation).unwrap_or_log();
        for event in reader.iter(events) {
            match event {
                FlockChangeEvent::MemberAdded { entt } => {
                    let entt = *entt;
                    // if the new member is the leader
                    if entt == center_pivot.boid_entt() {
                        if let Some(leader_directive) = &param.leader_directive {
                            let old_leader = state.leader.take().unwrap_or_log();
                            let (mut directive,) = crafts.get_mut(old_leader).unwrap_or_log();
                            *directive = boid::BoidMindDirective::JoinFomation {
                                formation: cur_formation.formation,
                            };
                            let (mut directive,) =
                                crafts.get_mut(center_pivot.boid_entt()).unwrap_or_log();
                            *directive = leader_directive.clone();
                        }
                        state.leader = Some(entt);
                    } else {
                        let (mut directive,) = crafts.get_mut(entt).unwrap_or_log();
                        *directive = boid::BoidMindDirective::JoinFomation {
                            formation: cur_formation.formation,
                        };
                    }
                    /*  let (mut directive,) = crafts.get_mut(entt).unwrap_or_log();
                    match directive.as_ref() {
                        boid::BoidMindDirective::JoinFomation { formation } => {
                            if *formation != param.formation {
                                *directive = boid::BoidMindDirective::JoinFomation {
                                    formation: param.formation,
                                };
                            }
                        }
                        _ => {
                            *directive = boid::BoidMindDirective::JoinFomation {
                                formation: param.formation,
                            };
                        }
                    } */
                }
                FlockChangeEvent::MemberRemoved { entt } => {
                    let entt = *entt;
                    // if the leader was changed
                    if entt == state.leader.unwrap_or_log() {
                        if let Some(leader_directive) = &param.leader_directive {
                            let (mut directive,) =
                                crafts.get_mut(center_pivot.boid_entt()).unwrap_or_log();
                            *directive = leader_directive.clone();
                        }
                    }
                }
            }
        }
    }
}

/* pub fn update(
    mut strategies: Query<(&FlockStrategy, &mut FormUpState)>,
    flocks: Query<&FlockMembers>,
    crafts: Query<(&GlobalTransform, &RigidBodyVelocityComponent)>,
) {
    for (strategy, mut state) in strategies.iter_mut() {}
}
 */
