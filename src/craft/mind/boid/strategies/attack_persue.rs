use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_rapier3d::prelude::*;

use super::{BoidStrategy, BoidStrategyDuoComponent, BoidStrategyOutput};
use crate::{
    craft::mind::{
        boid::{steering_systems::*, SteeringRoutineComposer},
        sensors::*,
    },
    math::*,
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

pub type AttackPersueBundle = BoidStrategyDuoComponent<AttackPersue, AttackPersueState>;

pub fn attack_persue_butler(
    mut commands: Commands,
    mut added_strategies: Query<
        (&AttackPersue, &BoidStrategy, &mut AttackPersueState),
        Added<AttackPersue>,
    >,
    crafts: Query<(
        &CraftRoutinesIndex,
        &CraftWeaponsIndex,
        &CraftStrategyIndex,
        Changed<CraftWeaponsIndex>,
    )>,
    mut routines: Query<&mut Intercept>,
) {
    for (params, strategy, mut state) in added_strategies.iter_mut() {
        let (routines, wpns, ..) = crafts
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
        let intercept = commands
            .spawn()
            .insert_bundle(InterceptRoutineBundle::new(
                Intercept {
                    quarry_rb: params.quarry_rb,
                    speed: None,
                },
                strategy.craft_entt(),
            ))
            .id();

        let intercept_wpn_speed = commands
            .spawn()
            .insert_bundle(InterceptRoutineBundle::new(
                Intercept {
                    quarry_rb: params.quarry_rb,
                    speed: if wpns.avg_projectile_speed > 0. {
                        Some(wpns.avg_projectile_speed)
                    } else {
                        None
                    },
                },
                strategy.craft_entt(),
            ))
            .id();
        commands.entity(strategy.craft_entt()).push_children(&[
            avoid_collision,
            intercept,
            intercept_wpn_speed,
        ]);
        state.intercept_routine = Some(intercept);
        state.intercept_wpn_speed = Some(intercept_wpn_speed);
        state.avoid_collision = Some(avoid_collision);
    }
    for (_, weapons, strategy_index, changed) in crafts.iter() {
        if !changed {
            continue;
        }
        if let Some(entts) = strategy_index.kind::<AttackPersue>() {
            for strategy in entts {
                if let Ok((_, _, state)) = added_strategies.get(*strategy) {
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

pub fn attack_persue(
    mut strategies: Query<
        (
            &AttackPersue,
            &BoidStrategy,
            &AttackPersueState,
            &mut BoidStrategyOutput,
        ),
        // With<ActiveRoutine>,
        // TODO: active routine filtering
    >,
    crafts: Query<&GlobalTransform>, // crafts
) {
    for (params, strategy, state, mut out) in strategies.iter_mut() {
        let xform = crafts
            .get(strategy.craft_entt())
            .expect("craft xform not found for CraftStrategy craft_entt");
        let quarry_xform = crafts
            .get(params.quarry_rb.entity())
            .expect("quarry_xform not found for on AttackPersue strategy");

        let target_distance_squared =
            (quarry_xform.translation - xform.translation).length_squared();
        let target_direction = (quarry_xform.translation - xform.translation).normalize();
        // if beyond range
        *out = if target_distance_squared > (params.attacking_range * params.attacking_range) {
            // intercept
            BoidStrategyOutput {
                routine_usage: SteeringRoutineComposer::Single {
                    entt: state.intercept_routine.unwrap(),
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
                            state.avoid_collision.unwrap(),
                            state.intercept_wpn_speed.unwrap(),
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
                            state.avoid_collision.unwrap(),
                            state.intercept_routine.unwrap(),
                        ],
                    },
                    fire_weapons: false,
                }
                // behind
            } else {
                BoidStrategyOutput {
                    routine_usage: SteeringRoutineComposer::PriorityOverride {
                        routines: smallvec::smallvec![
                            state.avoid_collision.unwrap(),
                            state.intercept_routine.unwrap(),
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
