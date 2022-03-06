use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*, reflect as bevy_reflect, utils::HashMap};
use bevy_inspector_egui::prelude::*;
use bevy_rapier3d::prelude::*;

use super::{
    super::{CraftFlock, FlockMembers},
    FlockStrategy, FlockStrategyBundleExtra,
};
use crate::craft::mind::boid::*;
use crate::math::*;

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
    #[inspectable(ignore)]
    member_to_strategy: HashMap<Entity, Entity>,
}

pub type CASBundle = FlockStrategyBundleExtra<CAS, CASState>;

pub fn cas_butler(
    mut commands: Commands,
    mut strategies: QuerySet<(
        QueryState<(Entity, &CAS, &FlockStrategy, &mut CASState), Added<CAS>>, // added
        QueryState<(Entity, &CAS, &FlockStrategy, &mut CASState)>,             // added
    )>,
    flocks: Query<&FlockMembers>,
    member_changes: Query<(Entity, &FlockMembers), Changed<FlockMembers>>,
    mut cache: Local<bevy::utils::HashMap<Entity, Entity>>,
) {
    for (strategy_entt, _params, strategy, mut state) in strategies.q0().iter_mut() {
        let members = flocks
            .get(strategy.flock_entt)
            .expect("unable to find FlockMind for new strategy");
        for craft_entt in members.iter() {
            let craft_entt = *craft_entt;
            let strategy_entt = commands
                .spawn()
                .insert(Name::new("boid_strategy"))
                .insert_bundle(strategy::SingleRoutineBundle::new(
                    strategy::SingleRoutine::new(Box::new(move |commands, _| {
                        commands
                            .spawn()
                            .insert_bundle(steering::FlyWithFlockRoutineBundle::new(
                                steering::FlyWithFlock { strategy_entt },
                                craft_entt,
                            ))
                            .id()
                    })),
                    craft_entt,
                    Default::default(),
                ))
                .insert(Parent(craft_entt))
                .id();
            state.member_to_strategy.insert(
                craft_entt,
                commands
                    .entity(craft_entt)
                    .insert(CraftFlock(strategy.flock_entt))
                    .insert(ActiveCraftStrategy {
                        strategy: Some(strategy_entt),
                    })
                    .id(),
            );
        }
    }

    for (flock_entt, members) in member_changes.iter() {
        if let Ok((strategy_entt, _params, strategy, mut state)) =
            strategies.q1().get_mut(flock_entt)
        {
            for craft_entt in members.iter() {
                let craft_entt = *craft_entt;
                let strategy_entt = commands
                    .spawn()
                    .insert(Name::new("boid_strategy"))
                    .insert_bundle(strategy::SingleRoutineBundle::new(
                        strategy::SingleRoutine::new(Box::new(move |commands, _| {
                            commands
                                .spawn()
                                .insert_bundle(steering::FlyWithFlockRoutineBundle::new(
                                    steering::FlyWithFlock { strategy_entt },
                                    craft_entt,
                                ))
                                .id()
                        })),
                        craft_entt,
                        Default::default(),
                    ))
                    .insert(Parent(craft_entt))
                    .id();
                cache.insert(
                    craft_entt,
                    state
                        .member_to_strategy
                        .remove(&craft_entt)
                        .unwrap_or_else(|| {
                            commands
                                .entity(craft_entt)
                                .insert(CraftFlock(strategy.flock_entt))
                                .insert(ActiveCraftStrategy {
                                    strategy: Some(strategy_entt),
                                })
                                .id()
                        }),
                );
            }
            for (_, routine) in state.member_to_strategy.drain() {
                commands.entity(routine).despawn_recursive();
            }
            state.member_to_strategy.extend(cache.drain());
        }
    }
}

pub fn cas(
    mut strategies: Query<(&FlockStrategy, &mut CASState)>,
    flocks: Query<&FlockMembers>,
    crafts: Query<(&GlobalTransform, &RigidBodyVelocityComponent)>,
) {
    for (strategy, mut state) in strategies.iter_mut() {
        let members = flocks
            .get(strategy.flock_entt)
            .expect("unable to find FlockMind for new strategy");
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
