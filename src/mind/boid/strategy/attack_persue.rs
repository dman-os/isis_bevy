use deps::*;

use bevy::{ prelude::*};
use bevy_rapier3d::prelude::*;

use super::{ActiveBoidStrategy, BoidStrategy, BoidStrategyBundleExtra, BoidStrategyOutput};
use crate::{
    math::*,
    mind::{
        boid::{steering::*, SteeringRoutineComposer},
        sensors::*,
    },
};

#[derive(Debug, Clone, Component)]
pub struct AttackPersue {
    pub quarry_rb: RigidBodyHandle,
    pub attacking_range: TReal,
}

#[derive(Debug, Clone, Default, Component)]
pub struct AttackPersueState {
    pub intercept_routine: Option<Entity>,
    pub intercept_wpn_speed: Option<Entity>,
    pub avoid_collision: Option<Entity>,
}

pub type Bundle = BoidStrategyBundleExtra<AttackPersue, AttackPersueState>;

pub fn butler(
    mut commands: Commands,
    mut added_strategies: Query<
        (Entity, &AttackPersue, &BoidStrategy, &mut AttackPersueState),
        Added<AttackPersue>,
    >,
    crafts: Query<(
        &SteeringRoutinesIndex,
        &CraftWeaponsIndex,
        &BoidStrategyIndex,
        Changed<CraftWeaponsIndex>,
    )>,
    mut routines: Query<&mut intercept::Intercept>,
) {
    for (entt, param, strategy, mut state) in added_strategies.iter_mut() {
        let (routines, wpns, ..) = crafts
            .get(strategy.craft_entt())
            .expect_or_log("craft not found for BoidStrategy");
        state.intercept_routine = Some(
            commands
                .spawn()
                .insert_bundle(intercept::Bundle::new(
                    intercept::Intercept {
                        quarry_rb: param.quarry_rb,
                        speed: None,
                    },
                    strategy.craft_entt(),
                ))
                .id(),
        );
        state.intercept_wpn_speed = Some(
            commands
                .spawn()
                .insert_bundle(intercept::Bundle::new(
                    intercept::Intercept {
                        quarry_rb: param.quarry_rb,
                        speed: if wpns.avg_projectile_speed > 0. {
                            Some(wpns.avg_projectile_speed)
                        } else {
                            None
                        },
                    },
                    strategy.craft_entt(),
                ))
                .id(),
        );
        state.avoid_collision = Some(
            routines
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
                }),
        );

        commands.entity(entt).insert(ActiveBoidStrategy);
    }
    for (_, weapons, strategy_index, changed) in crafts.iter() {
        if !changed {
            continue;
        }
        if let Some(entts) = strategy_index.kind::<AttackPersue>() {
            for strategy in entts {
                if let Ok((_, _, _, state)) = added_strategies.get(*strategy) {
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
    crafts: Query<&GlobalTransform>, // crafts
) {
    for (param, strategy, state, mut out) in strategies.iter_mut() {
        let xform = crafts
            .get(strategy.craft_entt())
            .expect_or_log("craft xform not found for CraftStrategy craft_entt");
        let quarry_xform = crafts
            .get(param.quarry_rb.entity())
            .expect_or_log("quarry_xform not found for on AttackPersue strategy");

        let target_distance_squared =
            (quarry_xform.translation - xform.translation).length_squared();
        let target_direction = (quarry_xform.translation - xform.translation).normalize();
        // if beyond range

        *out = if target_distance_squared > (param.attacking_range * param.attacking_range) {
            // intercept
            BoidStrategyOutput {
                routine_usage: SteeringRoutineComposer::Single {
                    entt: state.intercept_routine.unwrap_or_log(),
                },
                fire_weapons: false,
            }
        } else {
            // take action based on relative direction of quarry

            const DIRECTION_DETERMINATION_COS_THRESHOLD: TReal = 0.707;

            let fwdness = xform.forward().dot(target_direction);
            // ahead
            if fwdness > DIRECTION_DETERMINATION_COS_THRESHOLD {
                BoidStrategyOutput {
                    routine_usage: SteeringRoutineComposer::PriorityOverride {
                        routines: smallvec::smallvec![
                            state.avoid_collision.unwrap_or_log(),
                            state.intercept_wpn_speed.unwrap_or_log(),
                        ],
                    },
                    // fire_weapons: true,
                    fire_weapons: 1. - fwdness < crate::math::real::EPSILON * 10_000.,
                }
                // aside
            } else if fwdness < -DIRECTION_DETERMINATION_COS_THRESHOLD {
                BoidStrategyOutput {
                    routine_usage: SteeringRoutineComposer::PriorityOverride {
                        routines: smallvec::smallvec![
                            state.avoid_collision.unwrap_or_log(),
                            state.intercept_routine.unwrap_or_log(),
                        ],
                    },
                    fire_weapons: false,
                }
                // behind
            } else {
                BoidStrategyOutput {
                    routine_usage: SteeringRoutineComposer::PriorityOverride {
                        routines: smallvec::smallvec![
                            state.avoid_collision.unwrap_or_log(),
                            state.intercept_routine.unwrap_or_log(),
                        ],
                    },
                    fire_weapons: false,
                }
            }
        };
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
