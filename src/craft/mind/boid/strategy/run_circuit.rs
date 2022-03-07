use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_rapier3d::prelude::*;

use super::{
    super::SteeringRoutineComposer, ActiveBoidStrategy, BoidStrategy, BoidStrategyBundleExtra,
    BoidStrategyOutput,
};
use crate::{
    craft::mind::{boid::steering::*, sensors::*},
    math::*,
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

pub type RunCircuitBundle = BoidStrategyBundleExtra<RunCircuit, RunCircuitState>;

pub fn run_circuit_butler(
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
    for (entt, params, strategy, mut state, mut out) in added_strategies.iter_mut() {
        let routines = crafts
            .get(strategy.craft_entt())
            .expect("craft not found for BoidStrategy");
        let avoid_collision = routines
            .kind::<AvoidCollision>()
            .map(|v| v[0])
            .unwrap_or_else(|| {
                commands
                    .spawn()
                    .insert_bundle(AvoidCollisionRoutineBundle::new(
                        AvoidCollision::default(),
                        strategy.craft_entt(),
                    ))
                    .id()
            });
        let arrive = commands
            .spawn()
            .insert_bundle(ArriveRoutineBundle::new(
                Arrive {
                    target: ArriveTarget::Position {
                        pos: params.initial_location,
                    },
                    arrival_tolerance: 5.,
                    deceleration_radius: None,
                },
                strategy.craft_entt(),
            ))
            .id();

        commands
            .entity(strategy.craft_entt())
            .push_children(&[avoid_collision, arrive]);
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

pub fn run_circuit(
    strategies: Query<&RunCircuitState, With<ActiveBoidStrategy>>,
    checkpoints: Query<(Entity, &CircuitCheckpoint, &GlobalTransform)>,
    narrow_phase: Res<NarrowPhase>,
    parents: Query<&ColliderParentComponent>,
    mut arrive_routines: Query<&mut Arrive>,
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
                        let mut arrive_params = arrive_routines
                            .get_mut(state.arrive_routine.unwrap())
                            .expect("Arrive routine not found for RunCircuitState");

                        if let ArriveTarget::Position { pos: prev_pos } = arrive_params.target {
                            if prev_pos.distance_squared(checkopoint_xform.translation) < 1. {
                                tracing::info!("craft arrived at checkpoint {prev_pos:?}",);
                                arrive_params.target = ArriveTarget::Position {
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
