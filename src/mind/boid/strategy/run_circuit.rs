use deps::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use super::{
    super::SteeringRoutineComposer, ActiveBoidStrategy, BoidStrategy, BoidStrategyBundleExtra,
    BoidStrategyOutput,
};
use crate::{
    craft::*,
    mind::{boid::*, *},
};

#[derive(Debug, Clone, Component)]
pub struct RunCircuit {
    pub initial_point: Entity,
}

#[derive(Debug, Clone, Component)]
pub struct CircuitWaypoint {
    pub next_point: Entity,
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
    waypoints: Query<(&GlobalTransform,)>,
    crafts: Query<(&sensors::SteeringRoutinesIndex,)>,
) {
    for (entt, param, strategy, mut state, mut out) in added_strategies.iter_mut() {
        let (routines,) = crafts
            .get(strategy.craft_entt())
            .expect_or_log("craft not found for BoidStrategy");
        let (waypoint1_xform,) = waypoints
            .get(param.initial_point)
            .expect_or_log("initial CircuitWaypoint not found");
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
                        pos: waypoint1_xform.translation,
                        speed: 80.,
                        /* lin_vel: (waypoint2_xform.translation - waypoint1_xform.translation)
                        .normalize()
                        * engine_config.linvel_limit, */
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
    // mut commands: Commands,
    strategies: Query<&RunCircuitState, With<ActiveBoidStrategy>>,
    waypoints: Query<(Entity, &CircuitWaypoint, &GlobalTransform)>,
    narrow_phase: Res<NarrowPhase>,
    parents: Query<&ColliderParentComponent>,
    mut arrive_routines: Query<&mut arrive::Arrive>,
    crafts: Query<(
        // &CurrentBoidStrategy,
        &sensors::BoidStrategyIndex,
        &engine::EngineConfig,
        &RigidBodyVelocityComponent,
    )>,
) {
    for (checkpt_entt, waypoint, checkopoint_xform) in waypoints.iter() {
        // if something triggered the waypoint
        for (collider1, collider2) in narrow_phase
            .intersections_with(checkpt_entt.handle())
            .filter(|(_, _, ixing)| *ixing)
            .map(|(c1, c2, _)| (c1, c2))
        {
            let other_collider = if collider1.entity() == checkpt_entt {
                collider2.entity()
            } else {
                collider1.entity()
            };
            if let Ok(Ok((index, engine_config, vel))) = parents
                // if other_collider has a rigd body
                .get(other_collider)
                // and that rigd body belongs to a craft
                .map(|parent| crafts.get(parent.handle.entity()))
            {
                // for any acttive RunCircuit strategies on the craft
                if let Some(entts) = index.kind::<RunCircuit>() {
                    for entt in entts {
                        let state = strategies.get(*entt).expect_or_log(
                            "RunCircuitState not found for indexed RunCircuit strategy",
                        );
                        let mut arrive_param = arrive_routines
                            .get_mut(state.arrive_routine.unwrap_or_log())
                            .expect_or_log("Arrive routine not found for RunCircuitState");
                        match arrive_param.target {
                            arrive::Target::Position {
                                pos: prev_pos,
                                speed,
                            } => {
                                if prev_pos.distance_squared(checkopoint_xform.translation)
                                    - (engine_config.extents.max_element().powi(2))
                                    < 1.
                                {
                                    let cur_spd = vel.linvel.magnitude();
                                    // commands.entity(other_collider).despawn_recursive();
                                    tracing::info!(
                                        ?cur_spd,
                                        "craft arrived at waypoint {prev_pos:?}",
                                    );
                                    let (_, _, next_waypoint_xform) = waypoints
                                        .get(waypoint.next_point)
                                        .expect_or_log("next CircuitWaypoint not found");
                                    /*
                                    let (_, _, next_next_waypoint_xform) = waypoints
                                        .get(next_waypoint.next_point)
                                        .expect_or_log("next next CircuitWaypoint not found"); */
                                    arrive_param.target = arrive::Target::Position {
                                        pos: next_waypoint_xform.translation,
                                        /* lin_vel: (next_next_waypoint_xform.translation
                                        - next_waypoint_xform.translation)
                                        .normalize()
                                        * engine_config.linvel_limit, */
                                        speed,
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
