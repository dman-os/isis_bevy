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
    pub consts: steering::CraftControllerConsts,
    // smarts layer coordination
    pub active_strategy: CurrentBoidStrategy,
    pub cur_routine: CurrentSteeringRoutine,
    pub directive: BoidMindDirective,

    // indices
    pub routine_index: SteeringRoutinesIndex,
    pub wpn_index: CraftWeaponsIndex,
    pub strategy_index: BoidStrategyIndex,
}

#[derive(Clone, Component, Educe)]
#[educe(Debug, Default)]
pub enum BoidMindDirective {
    #[educe(Default)]
    None,
    KeepGoingForward,
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
    AttackPresue {
        param: strategy::attack_persue::AttackPersue,
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
            BoidMindDirective::KeepGoingForward => {
                let raycast_toi_modifier = dim.max_element();
                let cast_shape_radius = raycast_toi_modifier * 0.5;
                let avoid_collision: Box<strategy::custom::RoutineSpawner> =
                    Box::new(move |commands, _| {
                        commands.entity(boid_entt).add_children(|p| {
                            p.spawn()
                                .insert_bundle(steering::avoid_collision::Bundle::new(
                                    steering::avoid_collision::AvoidCollision::new(
                                        cast_shape_radius,
                                        raycast_toi_modifier,
                                    ),
                                    boid_entt,
                                    default(),
                                ))
                                .id()
                        })
                    });
                let closure: Box<strategy::custom::RoutineSpawner> =
                    Box::new(move |commands, strategy| {
                        commands.entity(boid_entt).add_children(|p| {
                            p.spawn()
                                .insert_bundle(steering::closure::Bundle::new(
                                    steering::closure::Closure {
                                        closure: Box::new(|xform, _, _| {
                                            (
                                                LinearRoutineOutput::Dir(xform.forward()),
                                                // LinearRoutineOutput::Dir(TVec3::Z),
                                                steering::look_to(-TVec3::Z).into(),
                                            )
                                        }),
                                    },
                                    strategy.boid_entt(),
                                ))
                                .id()
                        })
                    });
                Some(commands.entity(boid_entt).add_children(|p| {
                    p.spawn()
                        .insert_bundle(strategy::custom::Bundle::new(
                            strategy::custom::Custom::new(
                                strategy::custom::Composition::PriorityOverride {
                                    routines: smallvec::smallvec![avoid_collision, closure],
                                },
                            ),
                            boid_entt,
                        ))
                        .id()
                }))
            }
            /* BoidMindDirective::Custom { composition } => Some(
                commands.entity(boid_entt).add_children(|p|
                    p
                    .spawn()
                    .insert_bundle(strategy::custom::Bundle::new(
                        strategy::custom::Custom::new(composition.take().unwrap_or_log()),
                        boid_entt,
                    ))
                    .id(),
            )
            ), */
            BoidMindDirective::SlaveToPlayerControl => {
                let player: Box<strategy::custom::RoutineSpawner> = Box::new(move |commands, _| {
                    commands.entity(boid_entt).add_children(|p| {
                        p.spawn()
                            .insert_bundle(steering::player::Bundle::new(
                                steering::player::Player,
                                boid_entt,
                            ))
                            .id()
                    })
                });
                Some(commands.entity(boid_entt).add_children(|p| {
                    p.spawn()
                        .insert_bundle(strategy::custom::Bundle::new(
                            strategy::custom::Custom::new(strategy::custom::Composition::Single {
                                routine_spawner: player,
                            }),
                            boid_entt,
                        ))
                        .id()
                }))
            }
            BoidMindDirective::HoldPosition { pos } => {
                let pos = *pos;
                let accel_limit = engine_config.actual_accel_limit();
                let raycast_toi_modifier = dim.max_element();
                let cast_shape_radius = raycast_toi_modifier * 0.5;
                let avoid_collision: Box<strategy::custom::RoutineSpawner> =
                    Box::new(move |commands, _| {
                        commands.entity(boid_entt).add_children(|p| {
                            p.spawn()
                                .insert_bundle(steering::avoid_collision::Bundle::new(
                                    steering::avoid_collision::AvoidCollision::new(
                                        cast_shape_radius,
                                        raycast_toi_modifier,
                                    ),
                                    boid_entt,
                                    default(),
                                ))
                                .id()
                        })
                    });
                let arrive: Box<strategy::custom::RoutineSpawner> = Box::new(move |commands, _| {
                    commands.entity(boid_entt).add_children(|p| {
                        p.spawn()
                            .insert_bundle(steering::arrive::Bundle::new(
                                steering::arrive::Arrive {
                                    target: arrive::Target::Vector {
                                        at_pos: pos,
                                        pos_linvel: default(),
                                        // with_linvel: default(),
                                        with_speed: 0.,
                                    },
                                    arrival_tolerance: 5.,
                                    deceleration_radius: None,
                                    avail_accel: accel_limit,
                                },
                                boid_entt,
                            ))
                            .id()
                    })
                });

                Some(commands.entity(boid_entt).add_children(|p| {
                    p.spawn()
                        .insert_bundle(strategy::custom::Bundle::new(
                            strategy::custom::Custom::new(
                                strategy::custom::Composition::PriorityOverride {
                                    routines: smallvec::smallvec![avoid_collision, arrive],
                                },
                            ),
                            boid_entt,
                        ))
                        .id()
                }))
            }
            BoidMindDirective::JoinFomation { formation } => {
                let formation = *formation;
                Some(commands.entity(boid_entt).add_children(|p| {
                    p.spawn()
                        .insert_bundle(strategy::form::Bundle::new(
                            strategy::form::Form { formation },
                            boid_entt,
                            default(),
                        ))
                        .id()
                }))
            }
            BoidMindDirective::FlyWithFlockCAS { param } => {
                let param = param.clone();
                let raycast_toi_modifier = dim.max_element();
                let cast_shape_radius = raycast_toi_modifier * 0.5;
                let avoid_collision: Box<strategy::custom::RoutineSpawner> =
                    Box::new(move |commands, _| {
                        commands.entity(boid_entt).add_children(|p| {
                            p.spawn()
                                .insert_bundle(steering::avoid_collision::Bundle::new(
                                    steering::avoid_collision::AvoidCollision::new(
                                        cast_shape_radius,
                                        raycast_toi_modifier,
                                    ),
                                    boid_entt,
                                    default(),
                                ))
                                .id()
                        })
                    });
                let fly_with_flock: Box<strategy::custom::RoutineSpawner> =
                    Box::new(move |commands, _| {
                        commands.entity(boid_entt).add_children(|p| {
                            p.spawn()
                                .insert_bundle(steering::fly_with_flock::Bundle::new(
                                    param, boid_entt,
                                ))
                                .id()
                        })
                    });

                Some(commands.entity(boid_entt).add_children(|p| {
                    p.spawn()
                        .insert_bundle(strategy::custom::Bundle::new(
                            strategy::custom::Custom::new(
                                strategy::custom::Composition::PriorityOverride {
                                    routines: smallvec::smallvec![avoid_collision, fly_with_flock],
                                },
                            ),
                            boid_entt,
                        ))
                        .id()
                }))
            }
            BoidMindDirective::RunCircuit { param } => {
                Some(commands.entity(boid_entt).add_children(|p| {
                    p.spawn()
                        .insert_bundle(strategy::run_circuit::Bundle::new(
                            param.clone(),
                            boid_entt,
                            default(),
                        ))
                        .id()
                }))
            }
            BoidMindDirective::AttackPresue { param } => {
                Some(commands.entity(boid_entt).add_children(|p| {
                    p.spawn()
                        .insert_bundle(strategy::attack_persue::Bundle::new(
                            param.clone(),
                            boid_entt,
                            default(),
                        ))
                        .id()
                }))
            }
        }
    }
}
