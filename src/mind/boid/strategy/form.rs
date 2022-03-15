// FIXME: this layer's just overhead, get rid of it
//        - well, flock strategies shouldn't be allowed to touch boid strategies
//          for some reason. I guess it's staying?

use deps::*;

use bevy::prelude::*;

use crate::craft::*;
use crate::mind::{
    boid::{steering::*, strategy::*},
    flock::formation::*,
    sensors::*,
};

#[derive(Debug, Clone, Component)]
pub struct Form {
    pub formation: Entity,
}

#[derive(Debug, Clone, Component, Default)]
pub struct FormState {
    pub composer_routine: Option<Entity>,
    pub avoid_collision_routine: Option<Entity>,
    pub arrive_routine: Option<Entity>,
    pub face_routine: Option<Entity>,
}

pub type Bundle = BoidStrategyBundleExtra<Form, FormState>;

pub fn butler(
    mut commands: Commands,
    mut formations: Query<(&mut FormationState, &FormationOutputs)>,
    mut added_strategies: Query<
        (
            Entity,
            &Form,
            &BoidStrategy,
            &mut FormState,
            &mut BoidStrategyOutput,
        ),
        Added<Form>,
    >,
    crafts: Query<(
        &SteeringRoutinesIndex,
        &engine::EngineConfig,
        &CraftDimensions,
    )>,
) {
    for (entt, param, strategy, mut state, mut out) in added_strategies.iter_mut() {
        let (routines, engine_config, dim) = crafts
            .get(strategy.boid_entt())
            .expect_or_log("craft not found for BoidStrategy");

        let raycast_toi_modifier = dim.max_element();
        let cast_shape_radius = raycast_toi_modifier * 0.5;
        let avoid_collision = routines
            .kind::<avoid_collision::AvoidCollision>()
            .map(|v| v[0])
            .unwrap_or_else(|| {
                commands
                    .spawn()
                    .insert_bundle(avoid_collision::Bundle::new(
                        avoid_collision::AvoidCollision::new(
                            cast_shape_radius,
                            raycast_toi_modifier,
                        ),
                        strategy.boid_entt(),
                        Default::default(),
                    ))
                    .id()
            });

        let (mut formation_state, fomation_output) =
            formations.get_mut(param.formation).unwrap_or_log();
        formation_state
            .boid_strategies
            .insert(strategy.boid_entt(), entt);
        let form_out = fomation_output
            .index
            .get(&strategy.boid_entt())
            .unwrap_or_log();
        let arrive = commands
            .spawn()
            .insert_bundle(arrive::Bundle::new(
                arrive::Arrive {
                    target: arrive::Target::Vector {
                        at_pos: form_out.pos,
                        // with_linvel: form_out.linvel,
                        pos_linvel: form_out.pos_linvel,
                        with_speed: form_out.linvel.length(),
                    },
                    arrival_tolerance: 5.,
                    deceleration_radius: None,
                    linvel_limit: engine_config.linvel_limit,
                    avail_accel: engine_config.actual_acceleration_limit(),
                },
                strategy.boid_entt(),
            ))
            .id();
        let face = commands
            .spawn()
            .insert_bundle(face::Bundle::new(
                face::Face {
                    target: face::Target::Direction {
                        dir: form_out.facing,
                    },
                },
                strategy.boid_entt(),
            ))
            .id();
        let compose = commands
            .spawn()
            .insert_bundle(compose::Bundle::new(
                compose::Compose {
                    composer: compose::SteeringRoutineComposer::AvoidCollisionHelper {
                        avoid_collision,
                        routines: smallvec::smallvec![
                            ((1., 0.).into(), arrive),
                            ((0., 1.).into(), face),
                        ],
                    },
                },
                strategy.boid_entt(),
            ))
            .id();

        state.composer_routine = Some(compose);
        state.avoid_collision_routine = Some(avoid_collision);
        state.arrive_routine = Some(arrive);
        state.face_routine = Some(face);

        *out = BoidStrategyOutput {
            steering_routine: Some(compose),
            fire_weapons: false,
        };

        commands.entity(entt).insert(ActiveBoidStrategy);
    }
}

pub fn update(
    strategies: Query<(&FormState, Option<&ActiveBoidStrategy>)>,
    formations: Query<(&FormationOutputs, &FormationState)>,
    mut arrive_routines: Query<&mut arrive::Arrive>,
    mut face_routines: Query<&mut face::Face>,
) {
    // return;
    for (out, formation_state) in formations.iter() {
        let mut _skip_count = 0;

        for (boid_entt, strategy) in formation_state.boid_strategies.iter() {
            let (state, is_active) = strategies.get(*strategy).unwrap_or_log();
            if is_active.is_none() {
                // skip if boid_strategy is not active yet
                _skip_count += 1;
                continue;
            }
            let form_out = out.index.get(boid_entt).unwrap_or_log();

            let mut arrive_param = arrive_routines
                .get_mut(state.arrive_routine.unwrap_or_log())
                .unwrap_or_log();
            arrive_param.target = arrive::Target::Vector {
                at_pos: form_out.pos,
                // with_linvel: form_out.linvel,
                pos_linvel: form_out.pos_linvel,
                with_speed: form_out.linvel.length(),
            };
            let mut face_param = face_routines
                .get_mut(state.face_routine.unwrap_or_log())
                .unwrap_or_log();
            face_param.target = face::Target::Direction {
                dir: form_out.facing,
            };
        }
        // FIXME: 3 frame gap unless I divvy up the damn PreUpdate stage
        /* if _skip_count == 0 && out.positions.len() != formation_state.boid_strategies.len() {
            let expected = out.positions.len();
            let particpating = formation_state.boid_strategies.len();
            tracing::error!(
                ?expected,
                ?particpating,
                "expected formant count to particpating formant count disrepancies detected"
            );
        } */
    }
}
