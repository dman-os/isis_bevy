use deps::*;

use bevy::prelude::*;

use super::{ActiveBoidStrategy, BoidStrategy, BoidStrategyBundleExtra, BoidStrategyOutput};
use crate::{
    craft::*,
    math::*,
    mind::{boid::steering::*, sensors::*},
};

#[derive(Debug, Clone, Component)]
pub struct AttackPersue {
    pub quarry_rb: Entity,
    pub attacking_range: TReal,
}

#[derive(Debug, Clone, Default, Component)]
pub struct AttackPersueState {
    pub composer_routine: Option<Entity>,
    pub intercept_routine: Option<Entity>,
    pub intercept_wpn_speed: Option<Entity>,
    pub avoid_collision: Option<Entity>,
}

pub type Bundle = BoidStrategyBundleExtra<AttackPersue, AttackPersueState>;

pub fn butler(
    mut commands: Commands,
    mut added_strategies: Query<
        (
            Entity,
            &AttackPersue,
            &BoidStrategy,
            &mut AttackPersueState,
            &mut BoidStrategyOutput,
        ),
        Added<AttackPersue>,
    >,
    mut crafts: ParamSet<(
        Query<(
            &engine::EngineConfig,
            &CraftDimensions,
            &SteeringRoutinesIndex,
            &CraftWeaponsIndex,
        )>,
        Query<(&CraftWeaponsIndex, &BoidStrategyIndex), Changed<CraftWeaponsIndex>>,
    )>,
    mut routines: Query<&mut intercept::Intercept>,
) {
    for (entt, param, strategy, mut state, mut out) in added_strategies.iter_mut() {
        let p0 = crafts.p0();
        let (engine_config, dim, routines_idx, wpns, ..) = p0
            .get(strategy.boid_entt())
            .expect_or_log("craft not found for BoidStrategy");

        let raycast_toi_modifier = dim.max_element();
        let cast_shape_radius = raycast_toi_modifier * 0.5;
        let (avoid_collision, intercept_routine, intercept_wpn_speed) =
            commands.entity(strategy.boid_entt()).add_children(|par| {
                (
                    routines_idx
                        .kind::<avoid_collision::AvoidCollision>()
                        .map(|v| v[0])
                        .unwrap_or_else(|| {
                            par.spawn()
                                .insert_bundle(avoid_collision::Bundle::new(
                                    avoid_collision::AvoidCollision::new(
                                        cast_shape_radius,
                                        raycast_toi_modifier,
                                    ),
                                    strategy.boid_entt(),
                                    default(),
                                ))
                                .id()
                        }),
                    par.spawn()
                        .insert_bundle(intercept::Bundle::new(
                            intercept::Intercept {
                                quarry_rb: param.quarry_rb,
                                linvel_limit: engine_config.linvel_limit,
                                speed: None,
                            },
                            strategy.boid_entt(),
                        ))
                        .id(),
                    par.spawn()
                        .insert_bundle(intercept::Bundle::new(
                            intercept::Intercept {
                                quarry_rb: param.quarry_rb,
                                linvel_limit: engine_config.linvel_limit,
                                speed: if wpns.avg_projectile_speed > 0. {
                                    Some(wpns.avg_projectile_speed)
                                } else {
                                    None
                                },
                            },
                            strategy.boid_entt(),
                        ))
                        .id(),
                )
            });
        let compose = commands.entity(strategy.boid_entt()).add_children(|p| {
            p.spawn()
                .insert_bundle(compose::Bundle::new(
                    compose::Compose {
                        composer: compose::SteeringRoutineComposer::PriorityOverride {
                            routines: smallvec::smallvec![avoid_collision, intercept_routine,],
                        },
                    },
                    strategy.boid_entt(),
                ))
                .id()
        });

        state.intercept_routine = Some(intercept_routine);
        state.intercept_wpn_speed = Some(intercept_wpn_speed);
        state.avoid_collision = Some(avoid_collision);
        state.composer_routine = Some(compose);

        *out = BoidStrategyOutput {
            steering_routine: Some(compose),
            fire_weapons: false,
        };
        commands.entity(entt).insert(ActiveBoidStrategy);
    }
    for (weapons, strategy_index) in crafts.p1().iter() {
        if let Some(entts) = strategy_index.kind::<AttackPersue>() {
            for strategy in entts {
                if let Ok((_, _, _, state, ..)) = added_strategies.get(*strategy) {
                    if let Some(entt) = state.intercept_wpn_speed {
                        if let Ok(mut routine) = routines.get_mut(entt) {
                            routine.speed = Some(weapons.avg_projectile_speed)
                        }
                    }
                }
            }
        }
    }
}

#[allow(clippy::if_same_then_else)]
pub fn update(
    mut strategies: Query<
        (
            &AttackPersue,
            &BoidStrategy,
            &AttackPersueState,
            &mut BoidStrategyOutput,
        ),
        With<ActiveBoidStrategy>,
    >,
    crafts: Query<&Transform>, // crafts
    mut composers: Query<(&mut compose::Compose,)>,
) {
    for (param, strategy, state, mut out) in strategies.iter_mut() {
        let xform = crafts
            .get(strategy.boid_entt())
            .expect_or_log("craft xform not found for CraftStrategy boid_entt");
        let quarry_xform = crafts
            .get(param.quarry_rb)
            .expect_or_log("quarry_xform not found for on AttackPersue strategy");

        let target_distance_squared =
            (quarry_xform.translation - xform.translation).length_squared();
        let target_direction = (quarry_xform.translation - xform.translation).normalize();

        let (mut composer,) = composers
            .get_mut(state.composer_routine.unwrap_or_log())
            .unwrap_or_log();

        // if beyond range
        let (fire_wpns, second_routine) =
            if target_distance_squared > (param.attacking_range * param.attacking_range) {
                // intercept
                (false, state.intercept_routine.unwrap_or_log())
            } else {
                // take action based on relative direction of quarry
                const DIRECTION_DETERMINATION_COS_THRESHOLD: TReal = 0.707;
                let fwdness = xform.forward().dot(target_direction);
                // ahead
                if fwdness > DIRECTION_DETERMINATION_COS_THRESHOLD {
                    (
                        1. - fwdness < crate::math::real::EPSILON * 10_000.,
                        state.intercept_wpn_speed.unwrap_or_log(),
                    )
                }
                // aside
                else if fwdness < -DIRECTION_DETERMINATION_COS_THRESHOLD {
                    (false, state.intercept_routine.unwrap_or_log())
                }
                // behind
                else {
                    (false, state.intercept_routine.unwrap_or_log())
                }
            };
        out.fire_weapons = fire_wpns;
        match &mut composer.composer {
            compose::SteeringRoutineComposer::PriorityOverride { routines } => {
                routines[1] = second_routine;
            }
            _ => unreachable!(),
        }
    }
}
/*
pub enum GeneralRelativeDirection {
    Ahead,
    Aside,
    Behind,
}


pub trait RelativeDirection {
    fn rel_dir(&self, other: &Self, cos_threshold: TReal) -> GeneralRelativeDirection;

    fn rel_dir_default(&self, other: &Self) -> GeneralRelativeDirection {
        self.rel_dir(other, DIRECTION_DETERMINATION_COS_THRESHOLD)
    }
}

impl RelativeDirection for TVec3 {
    fn rel_dir(&self, other: &TVec3, cos_threshold: TReal) -> GeneralRelativeDirection {
        let fwdness = self.dot(*other);
        if fwdness > cos_threshold {
            GeneralRelativeDirection::Ahead
        } else if fwdness < -cos_threshold {
            GeneralRelativeDirection::Behind
        } else {
            GeneralRelativeDirection::Aside
        }
    }
}

impl RelativeDirection for Transform {
    fn rel_dir(&self, other: &Transform, cos_threshold: TReal) -> GeneralRelativeDirection {
        self.forward()
            .rel_dir(&(self.translation - other.translation), cos_threshold)
    }
}

impl RelativeDirection for GlobalTransform {
    fn rel_dir(&self, other: &GlobalTransform, cos_threshold: TReal) -> GeneralRelativeDirection {
        self.forward()
            .rel_dir(&(self.translation - other.translation), cos_threshold)
    }
}
 */
