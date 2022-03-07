use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_rapier3d::prelude::*;

use super::{
    super::SteeringRoutineComposer, ActiveBoidStrategy, BoidStrategy, BoidStrategyBundleExtra,
    BoidStrategyOutput,
};
use crate::{
    math::*,
    mind::{boid::steering::*, sensors::*},
};

#[derive(Debug, Clone, Component)]
pub struct RunCircuit {
    pub initial_location: TVec3,
}

#[derive(Debug, Clone, Component)]
pub struct CircuitCheckpoint {
    pub next_point_location: TVec3,
}

#[derive(Debug, Clone, Component, Default)]
pub struct RunCircuitState {
    pub arrive_routine: Option<Entity>,
    pub avoid_collision_routine: Option<Entity>,
}

pub type Bundle = BoidStrategyBundleExtra<RunCircuit, RunCircuitState>;

pub fn butler(
    mut commands: Commands,
    mut added_strategies: Query<
        (
            Entity,
            &RunCircuit,
            &BoidStrategy,
            &mut RunCircuitState,
            &mut BoidStrategyOutput,
        ),
        Added<RunCircuit>,
    >,
    crafts: Query<&CraftRoutinesIndex>,
) {
    for (entt, param, strategy, mut state, mut out) in added_strategies.iter_mut() {
        let routines = crafts
            .get(strategy.craft_entt())
            .expect("craft not found for BoidStrategy");
        let avoid_collision = routines
            .kind::<avoid_collision::AvoidCollision>()
            .map(|v| v[0])
            .unwrap_or_else(|| {
                commands
                    .spawn()
                    .insert_bundle(avoid_collision::Bundle::new(
                        avoid_collision::AvoidCollision::default(),
                        strategy.craft_entt(),
                    ))
                    .id()
            });
        let arrive = commands
            .spawn()
            .insert_bundle(arrive::Bundle::new(
                arrive::Arrive {
                    target: arrive::Target::Position {
                        pos: param.initial_location,
                    },
                    arrival_tolerance: 5.,
                    deceleration_radius: None,
                },
                strategy.craft_entt(),
            ))
            .id();

        state.arrive_routine = Some(arrive);
        state.avoid_collision_routine = Some(avoid_collision);
        *out = BoidStrategyOutput {
            routine_usage: SteeringRoutineComposer::PriorityOverride {
                routines: smallvec::smallvec![avoid_collision, arrive],
            },
            fire_weapons: false,
        };

        commands.entity(entt).insert(ActiveBoidStrategy);
    }
}

pub fn update(
    strategies: Query<&RunCircuitState, With<ActiveBoidStrategy>>,
    checkpoints: Query<(Entity, &CircuitCheckpoint, &GlobalTransform)>,
    narrow_phase: Res<NarrowPhase>,
    parents: Query<&ColliderParentComponent>,
    mut arrive_routines: Query<&mut arrive::Arrive>,
    crafts: Query<&CraftStrategyIndex>,
) {
    for (checkpt_entt, checkpoint, checkopoint_xform) in checkpoints.iter() {
        // if something triggered the checkpoint
        for (collider1, collider2) in narrow_phase
            .intersections_with(checkpt_entt.handle())
            .filter(|(_, _, ixing)| *ixing)
            .map(|(c1, c2, _)| (c1, c2))
        {
            // that collider belongs to a craft
            if let Ok(Ok(index)) = parents
                .get(if collider1.entity() == checkpt_entt {
                    collider2.entity()
                } else {
                    collider1.entity()
                })
                .map(|parent| crafts.get(parent.handle.entity()))
            {
                // if the craft is running the circuit
                if let Some(entts) = index.kind::<RunCircuit>() {
                    for entt in entts {
                        let state = strategies
                            .get(*entt)
                            .expect("RunCircuitState not found for indexed strategy");
                        let mut arrive_param = arrive_routines
                            .get_mut(state.arrive_routine.unwrap())
                            .expect("Arrive routine not found for RunCircuitState");

                        if let arrive::Target::Position { pos: prev_pos } = arrive_param.target {
                            if prev_pos.distance_squared(checkopoint_xform.translation) < 1. {
                                tracing::info!("craft arrived at checkpoint {prev_pos:?}",);
                                arrive_param.target = arrive::Target::Position {
                                    pos: checkpoint.next_point_location,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
