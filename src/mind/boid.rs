use deps::*;

use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;
use educe::Educe;

use crate::{craft::*, math::*, mind::sensors::*};

use steering::*;
use strategy::*;

pub mod steering;
pub mod strategy;

#[derive(Debug, Clone, Inspectable, Component)]
pub struct BoidMindConfig {
    pub angular_input_multiplier: TReal,
}

impl Default for BoidMindConfig {
    fn default() -> Self {
        Self {
            angular_input_multiplier: 10.,
        }
    }
}

#[derive(Bundle, Default)]
pub struct BoidMindBundle {
    pub config: BoidMindConfig,
    // smarts layer coordination
    pub active_strategy: CurrentBoidStrategy,
    pub cur_routine: CurrentSteeringRoutine,
    pub directive: BoidMindDirective,

    // indices
    pub routine_index: SteeringRoutinesIndex,
    pub wpn_index: CraftWeaponsIndex,
    pub strategy_index: BoidStrategyIndex,
}

#[derive(Debug, Clone, Component, Educe)]
#[educe(Default)]
pub enum BoidMindDirective {
    #[educe(Default)]
    None,
    SlaveToPlayerControl,
    HoldPosition {
        pos: TVec3,
    },
    JoinFomation {
        formation: Entity,
    },
    FlyWithFlockCAS {
        param: steering::fly_with_flock::FlyWithFlock,
    },
    RunCircuit {
        param: strategy::run_circuit::RunCircuit,
    },
}

pub fn boid_mind(
    mut commands: Commands,
    mut boids: Query<
        (
            Entity,
            &BoidMindDirective,
            &mut CurrentBoidStrategy,
            &engine::EngineConfig,
            &CraftDimensions,
        ),
        Changed<BoidMindDirective>,
    >,
) {
    for (boid_entt, directive, mut cur_stg, engine_config, dim) in boids.iter_mut() {
        if let Some(cur_stg) = cur_stg.strategy.take() {
            commands.entity(cur_stg).despawn_recursive();
        }
        cur_stg.strategy = match directive {
            BoidMindDirective::None => None,
            BoidMindDirective::SlaveToPlayerControl => {
                let player: Box<strategy::custom::RoutineSpawner> = Box::new(move |commands, _| {
                    commands
                        .spawn()
                        .insert_bundle(steering::player::Bundle::new(
                            steering::player::Player,
                            boid_entt,
                        ))
                        .id()
                });
                Some(
                    commands
                        .spawn()
                        .insert_bundle(strategy::custom::Bundle::new(
                            strategy::custom::Custom::new(strategy::custom::Composition::Single {
                                routine_spawner: player,
                            }),
                            boid_entt,
                        ))
                        .id(),
                )
            }
            BoidMindDirective::HoldPosition { pos } => {
                let pos = *pos;
                let linvel_limit = engine_config.linvel_limit;
                let accel_limit = engine_config.actual_acceleration_limit();
                let raycast_toi_modifier = dim.max_element();
                let cast_shape_radius = raycast_toi_modifier * 0.5;
                let avoid_collision: Box<strategy::custom::RoutineSpawner> =
                    Box::new(move |commands, _| {
                        commands
                            .spawn()
                            .insert_bundle(steering::avoid_collision::Bundle::new(
                                steering::avoid_collision::AvoidCollision::new(
                                    cast_shape_radius,
                                    raycast_toi_modifier,
                                ),
                                boid_entt,
                                Default::default(),
                            ))
                            .id()
                    });
                let arrive: Box<strategy::custom::RoutineSpawner> = Box::new(move |commands, _| {
                    commands
                        .spawn()
                        .insert_bundle(steering::arrive::Bundle::new(
                            steering::arrive::Arrive {
                                target: arrive::Target::Position { pos, speed: 0. },
                                arrival_tolerance: 5.,
                                deceleration_radius: None,
                                linvel_limit,
                                accel_limit,
                            },
                            boid_entt,
                        ))
                        .id()
                });

                Some(
                    commands
                        .spawn()
                        .insert_bundle(strategy::custom::Bundle::new(
                            strategy::custom::Custom::new(
                                strategy::custom::Composition::PriorityOverride {
                                    routines: smallvec::smallvec![avoid_collision, arrive],
                                },
                            ),
                            boid_entt,
                        ))
                        .id(),
                )
            }
            BoidMindDirective::JoinFomation { formation } => {
                let formation = *formation;
                Some(
                    commands
                        .spawn()
                        .insert_bundle(strategy::form::Bundle::new(
                            strategy::form::Form { formation },
                            boid_entt,
                            Default::default(),
                        ))
                        .id(),
                )
            }
            BoidMindDirective::FlyWithFlockCAS { param } => {
                let param = param.clone();
                let raycast_toi_modifier = dim.max_element();
                let cast_shape_radius = raycast_toi_modifier * 0.5;
                let avoid_collision: Box<strategy::custom::RoutineSpawner> =
                    Box::new(move |commands, _| {
                        commands
                            .spawn()
                            .insert_bundle(steering::avoid_collision::Bundle::new(
                                steering::avoid_collision::AvoidCollision::new(
                                    cast_shape_radius,
                                    raycast_toi_modifier,
                                ),
                                boid_entt,
                                Default::default(),
                            ))
                            .id()
                    });
                let fly_with_flock: Box<strategy::custom::RoutineSpawner> =
                    Box::new(move |commands, _| {
                        commands
                            .spawn()
                            .insert_bundle(steering::fly_with_flock::Bundle::new(param, boid_entt))
                            .id()
                    });

                Some(
                    commands
                        .spawn()
                        .insert_bundle(strategy::custom::Bundle::new(
                            strategy::custom::Custom::new(
                                strategy::custom::Composition::PriorityOverride {
                                    routines: smallvec::smallvec![avoid_collision, fly_with_flock],
                                },
                            ),
                            boid_entt,
                        ))
                        .id(),
                )
            }
            BoidMindDirective::RunCircuit { param } => Some(
                commands
                    .spawn()
                    .insert_bundle(strategy::run_circuit::Bundle::new(
                        param.clone(),
                        boid_entt,
                        Default::default(),
                    ))
                    .id(),
            ),
        }
    }
}
