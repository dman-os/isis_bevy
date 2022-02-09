use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_rapier3d::prelude::*;

use super::{BoidStrategy, BoidStrategyDuoComponent, BoidStrategyOutput, SteeringRoutineComposer};
use crate::{
    craft::mind::{boid::steering_systems::*, sensors::*},
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
    pub seek_routine: Option<Entity>,
    pub avoid_collision_routine: Option<Entity>,
}

pub type RunCircuitBundle = BoidStrategyDuoComponent<RunCircuit, RunCircuitState>;

pub fn run_circuit_butler(
    mut commands: Commands,
    mut added_strategies: Query<
        (
            &RunCircuit,
            &BoidStrategy,
            &mut RunCircuitState,
            &mut BoidStrategyOutput,
        ),
        Added<RunCircuit>,
    >,
    crafts: Query<&CraftRoutinesIndex>,
) {
    for (params, strategy, mut state, mut out) in added_strategies.iter_mut() {
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
        let seek = commands
            .spawn()
            .insert_bundle(SeekRoutineBundle::new(
                Seek {
                    target: SeekTarget::Position {
                        pos: params.initial_location,
                    },
                },
                strategy.craft_entt(),
            ))
            .id();
        commands
            .entity(strategy.craft_entt())
            .push_children(&[avoid_collision, seek]);
        state.seek_routine = Some(seek);
        state.avoid_collision_routine = Some(avoid_collision);
        *out = BoidStrategyOutput {
            routine_usage: SteeringRoutineComposer::PriorityOverride {
                routines: smallvec::smallvec![avoid_collision, seek],
            },
            fire_weapons: false,
        }
    }
}

pub fn run_circuit(
    strategies: Query<&RunCircuitState>,
    checkpoints: Query<(Entity, &CircuitCheckpoint, &GlobalTransform)>,
    narrow_phase: Res<NarrowPhase>,
    parents: Query<&ColliderParentComponent>,
    mut seek_routines: Query<&mut Seek>,
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
                        let mut seek_params = seek_routines
                            .get_mut(state.seek_routine.unwrap())
                            .expect("Seek routine not found for RunCircuitState");

                        if let SeekTarget::Position { pos: prev_pos } = seek_params.target {
                            if prev_pos.distance_squared(checkopoint_xform.translation) < 1. {
                                tracing::info!("craft arrived at checkpoint {prev_pos:?}",);
                                seek_params.target = SeekTarget::Position {
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
