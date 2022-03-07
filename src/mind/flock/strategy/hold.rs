use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};

use super::ActiveFlockStrategy;
use super::{super::FlockMembers, FlockStrategy, FlockStrategyBundleExtra};
use crate::math::*;
use crate::mind::*;

/* #[derive(Debug, Clone, Component)]
pub enum Target {
    /// must have a global xform
    Object { entt: Entity },
    /// assumed to be in world basis
    Position { pos: TVec3 },
} */

/// Hold a position.
#[derive(Debug, Clone, Component)]
pub struct Hold {
    pub pos: TVec3,
    // formation: Option<Entity>,
    pub formation: Entity,
}

#[derive(Debug, Clone, Component, Default)]
pub struct HoldState {}

pub type Bundle = FlockStrategyBundleExtra<Hold, HoldState>;

#[allow(clippy::type_complexity)]
pub fn butler(
    mut commands: Commands,
    mut strategies: QuerySet<(
        QueryState<(Entity, &FlockStrategy, &Hold, &mut HoldState), Added<Hold>>, // added
        QueryState<(&Hold, &mut HoldState)>,                                      // all
    )>,
    flocks: Query<(&FlockMembers,)>,
    mut crafts: Query<(&mut boid::BoidMindDirective,)>,
    member_changes: Query<(Entity, &FlockMembers), Changed<FlockMembers>>,
) {
    for (strategy_entt, strategy, param, _state) in strategies.q0().iter_mut() {
        let (members,) = flocks
            .get(strategy.flock_entt)
            .expect("unable to find Flock for new strategy");
        for craft_entt in members.iter() {
            let craft_entt = *craft_entt;

            let (mut directive,) = crafts
                .get_mut(craft_entt)
                .expect("unable to find craft for flock");
            *directive = boid::BoidMindDirective::JoinFomation {
                formation: param.formation,
            };
        }
        commands.entity(strategy_entt).insert(ActiveFlockStrategy);
    }

    for (flock_entt, members) in member_changes.iter() {
        if let Ok((param, _state)) = strategies.q1().get_mut(flock_entt) {
            for craft_entt in members.iter() {
                let craft_entt = *craft_entt;

                let (mut directive,) = crafts
                    .get_mut(craft_entt)
                    .expect("unable to find craft for flock");
                match directive.as_ref() {
                    boid::BoidMindDirective::JoinFomation { formation } => {
                        if *formation != param.formation {
                            *directive = boid::BoidMindDirective::JoinFomation {
                                formation: param.formation,
                            };
                        }
                    }
                    _ => {
                        *directive = boid::BoidMindDirective::JoinFomation {
                            formation: param.formation,
                        };
                    }
                }
            }
        }
    }
}

/* pub fn update(
    mut strategies: Query<(&FlockStrategy, &mut HoldState)>,
    flocks: Query<&FlockMembers>,
    crafts: Query<(&GlobalTransform, &RigidBodyVelocityComponent)>,
) {
    for (strategy, mut state) in strategies.iter_mut() {}
}
 */
