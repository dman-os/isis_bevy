use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*, utils::HashMap};
use bevy_rapier3d::prelude::*;

use super::{
    super::{CraftFlock, FlockMembers},
    ActiveFlockStrategy, FlockStrategy, FlockStrategyBundleJumbo,
};
use crate::craft::mind::*;
use crate::math::*;

#[derive(Debug, Clone, Component)]
pub struct Formation {
    pub pattern: FormationPattern,
    pub slotting_strategy: SlottingStrategy,
}

#[derive(Debug, Clone, Component)]
pub enum FormationPattern {
    Sphere {
        center: FormationPivot,
        radius: TReal,
    },
}

#[derive(Debug, Clone, Component)]
pub enum SlottingStrategy {
    Simple,
}

impl SlottingStrategy {
    pub fn slot<'a>(
        &self,
        members: &'a flock::FlockMembers,
        _member_xforms: &'a dyn Fn(Entity) -> &'a GlobalTransform,
    ) -> FormationSlots {
        FormationSlots {
            slots: members.iter().enumerate().map(|(i, c)| (*c, i)).collect(),
        }
    }
}

#[derive(Debug, Clone, Component)]
pub enum FormationPivot {
    Craft { entt: Entity },
    Anchor { xform: GlobalTransform },
}

#[derive(Debug, Default)]
pub struct FormationSlots {
    pub slots: HashMap<Entity, usize>,
}

impl FormationSlots {
    pub fn len(&self) -> usize {
        self.slots.len()
    }
}

#[derive(Debug, Component, Default)]
pub struct FormationState {
    pub slots: FormationSlots,
    pub member_to_strategy: HashMap<Entity, Entity>,
}

#[derive(Debug, Default, Component)]
pub struct FormationOutput {
    pub positions: HashMap<Entity, TVec3>,
}

pub type FormationBundle = FlockStrategyBundleJumbo<Formation, (FormationState, FormationOutput)>;

pub fn formation_butler(
    mut commands: Commands,
    mut strategies: QuerySet<(
        QueryState<
            (
                Entity,
                &Formation,
                &FlockStrategy,
                &mut FormationState,
                &mut FormationOutput,
            ),
            Added<Formation>,
        >, // added
        QueryState<(
            Entity,
            &Formation,
            &FlockStrategy,
            &mut FormationState,
            &mut FormationOutput,
        )>, // all
    )>,
    flocks: Query<&FlockMembers>,
    crafts: Query<&GlobalTransform>,
    member_changes: Query<(Entity, &FlockMembers), Changed<FlockMembers>>,
    mut cache: Local<bevy::utils::HashMap<Entity, Entity>>,
) {
    for (entt, param, strategy, mut state, mut output) in strategies.q0().iter_mut() {
        let members = flocks
            .get(strategy.flock_entt)
            .expect("unable to find Flock for new strategy");

        for craft_entt in members.iter() {
            let craft_entt = *craft_entt;
            output.positions.insert(craft_entt, Default::default());

            let strategy_entt = commands
                .spawn()
                .insert_bundle(boid::strategy::FormBundle::new(
                    boid::strategy::Form { formation: entt },
                    craft_entt,
                    Default::default(),
                ))
                .insert(Parent(craft_entt))
                .id();

            commands
                .entity(craft_entt)
                .insert(CraftFlock(strategy.flock_entt))
                .insert(CurrentBoidStrategy {
                    strategy: Some(strategy_entt),
                });
            state.member_to_strategy.insert(craft_entt, strategy_entt);
        }

        state.slots = param
            .slotting_strategy
            .slot(members, &|entt| crafts.get(entt).unwrap());

        commands.entity(entt).insert(ActiveFlockStrategy);
    }

    for (flock_entt, members) in member_changes.iter() {
        if let Ok((strategy_entt, param, strategy, mut state, mut output)) =
            strategies.q1().get_mut(flock_entt)
        {
            output.positions.clear();
            for craft_entt in members.iter() {
                let craft_entt = *craft_entt;
                output.positions.insert(craft_entt, Default::default());

                cache.insert(
                    craft_entt,
                    state
                        .member_to_strategy
                        .remove(&craft_entt)
                        .unwrap_or_else(|| {
                            let strategy_entt = commands
                                .spawn()
                                .insert_bundle(boid::strategy::FormBundle::new(
                                    boid::strategy::Form {
                                        formation: strategy_entt,
                                    },
                                    craft_entt,
                                    Default::default(),
                                ))
                                .insert(Parent(craft_entt))
                                .id();
                            commands
                                .entity(craft_entt)
                                .insert(CraftFlock(strategy.flock_entt))
                                .insert(CurrentBoidStrategy {
                                    strategy: Some(strategy_entt),
                                });
                            strategy_entt
                        }),
                );
            }
            for (_, routine) in state.member_to_strategy.drain() {
                commands.entity(routine).despawn_recursive();
            }
            state.member_to_strategy.extend(cache.drain());

            state.slots = param
                .slotting_strategy
                .slot(members, &|entt| crafts.get(entt).unwrap());
        }
    }
}

pub fn formation(
    mut strategies: Query<
        (
            &FlockStrategy,
            &Formation,
            &mut FormationState,
            &mut FormationOutput,
        ),
        With<ActiveFlockStrategy>,
    >,
    // flocks: Query<&FlockMembers>,
    crafts: Query<(&GlobalTransform, &RigidBodyVelocityComponent)>,
) {
    for (_strategy, param, state, mut out) in strategies.iter_mut() {
        let member_size = state.slots.len();
        if member_size == 0 {
            continue;
        }
        match &param.pattern {
            FormationPattern::Sphere { center, radius } => {
                let center_pivot = match center {
                    FormationPivot::Craft { entt } => crafts.get(*entt).unwrap().0,
                    FormationPivot::Anchor { xform } => xform,
                };
                let rays = crate::utils::points_on_sphere(member_size);
                for (craft_entt, ii) in state.slots.slots.iter() {
                    out.positions.insert(
                        *craft_entt,
                        center_pivot.translation + (rays[*ii] * *radius),
                    );
                }
            }
        }
    }
}
