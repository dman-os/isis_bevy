use deps::*;

use bevy::prelude::*;
use bevy_inspector_egui::prelude::*;
use bevy_rapier3d::prelude::*;

use super::ActiveFlockStrategy;
use super::{super::FlockMembers, FlockStrategy, FlockStrategyBundleExtra};
use crate::math::*;
use crate::mind::*;

/// Cohesion, Allignment, Separation.
#[derive(Debug, Clone, Component, Default)]
pub struct CAS {}

#[derive(Debug, Clone, Component, Default, Reflect, Inspectable)]
pub struct CASState {
    pub vel_sum: TVec3,
    pub avg_vel: TVec3,
    pub center_sum: TVec3,
    pub center: TVec3,
    pub craft_positions: Vec<TVec3>,
    pub member_count: usize,
}

pub type Bundle = FlockStrategyBundleExtra<CAS, CASState>;

pub fn butler(
    mut commands: Commands,
    mut strategies: QuerySet<(
        QueryState<(Entity, &CAS, &FlockStrategy), Added<CAS>>, // added
        QueryState<(Entity, &CAS)>,                             // all
    )>,
    flocks: Query<&FlockMembers>,
    member_changes: Query<(Entity, &FlockMembers), Changed<FlockMembers>>,
    mut crafts: Query<(&mut boid::BoidMindDirective,)>,
) {
    for (strategy_entt, _param, strategy) in strategies.q0().iter_mut() {
        let members = flocks
            .get(strategy.flock_entt)
            .expect_or_log("unable to find Flock for new strategy");
        for boid_entt in members.iter() {
            let boid_entt = *boid_entt;
            let (mut directive,) = crafts
                .get_mut(boid_entt)
                .expect_or_log("unable to find Boid for Flock member");
            *directive = boid::BoidMindDirective::FlyWithFlockCAS {
                param: boid::steering::fly_with_flock::FlyWithFlock {
                    flock_strategy_entt: strategy_entt,
                },
            };
        }
        commands.entity(strategy_entt).insert(ActiveFlockStrategy);
    }

    for (flock_entt, members) in member_changes.iter() {
        if let Ok((strategy_entt, _param)) = strategies.q1().get_mut(flock_entt) {
            for boid_entt in members.iter() {
                let boid_entt = *boid_entt;
                let (mut directive,) = crafts
                    .get_mut(boid_entt)
                    .expect_or_log("unable to find Boid for Flock member");
                match directive.as_ref() {
                    boid::BoidMindDirective::FlyWithFlockCAS { .. } => {
                        continue;
                    }
                    _ => {
                        *directive = boid::BoidMindDirective::FlyWithFlockCAS {
                            param: boid::steering::fly_with_flock::FlyWithFlock {
                                flock_strategy_entt: strategy_entt,
                            },
                        };
                    }
                }
            }
        }
    }
}

pub fn update(
    mut strategies: Query<(&FlockStrategy, &mut CASState), With<ActiveFlockStrategy>>,
    flocks: Query<&FlockMembers>,
    crafts: Query<(&GlobalTransform, &RigidBodyVelocityComponent)>,
) {
    for (strategy, mut state) in strategies.iter_mut() {
        let members = flocks
            .get(strategy.flock_entt)
            .expect_or_log("unable to find FlockMind for new strategy");
        state.craft_positions.clear();
        state.vel_sum = TVec3::ZERO;
        state.center_sum = TVec3::ZERO;
        for craft in members.iter() {
            if let Ok((xform, vel)) = crafts.get(*craft) {
                state.craft_positions.push(xform.translation);
                state.vel_sum += TVec3::from(vel.linvel);
                state.center_sum += xform.translation;
            } else {
                tracing::error!("unable to find group mind member when updating flocks");
            }
        }
        state.member_count = members.len();
        state.avg_vel = state.vel_sum / members.len() as TReal;
        state.center = state.center_sum / members.len() as TReal;
    }
}
