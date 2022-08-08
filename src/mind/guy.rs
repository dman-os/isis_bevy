/* use deps::*;

use bevy::prelude::*;

use crate::math::*;
use crate::mind::*;

#[derive(Debug, Component)]
pub enum GuyTroops {
    Boid { entt: Entity },
    Flock { entt: Entity },
}

#[derive(Debug, Clone, Component, educe::Educe)]
#[educe(Default)]
pub enum GuyMindDirective {
    #[educe(Default)]
    None,
    Hold {
        pos: TVec3,
    },
    JoinFomation {
        formation: Entity,
    },
}

pub fn guy_mind(
    minds: Query<(Entity, &GuyTroops, &GuyMindDirective), Changed<GuyMindDirective>>,
    mut boid_minds: Query<(&mut boid::BoidMindDirective,)>,
    mut flock_minds: Query<(&mut flock::FlockMindDirective,)>,
) {
    for (guy_entt, troops, guy_directive) in minds.iter() {
        match troops {
            GuyTroops::Boid { entt } => {
                let (mut directive,) = boid_minds
                    .get_mut(*entt)
                    .expect_or_log("GuyTroop::SingleBoid entt not found in world");
                *directive = match guy_directive {
                    GuyMindDirective::None => boid::BoidMindDirective::None,
                    GuyMindDirective::Hold { pos } => {
                        boid::BoidMindDirective::HoldPosition { pos: *pos }
                    }
                    GuyMindDirective::JoinFomation { formation } => {
                        boid::BoidMindDirective::JoinFomation {
                            formation: *formation,
                        }
                    }
                };
            }
            GuyTroops::Flock { entt } => {
                let (mut directive,) = flock_minds
                    .get_mut(*entt)
                    .expect_or_log("GuyTroop::Flock entt not found in world");
                *directive = match guy_directive {
                    GuyMindDirective::None => flock::FlockMindDirective::None,
                    // GuyMindDirective::Hold { pos } => flock::FlockMindDirective::FormUp { pos: *pos },
                    GuyMindDirective::JoinFomation { formation } => {
                        flock::FlockMindDirective::JoinFomation {
                            formation: *formation,
                        }
                    }
                    GuyMindDirective::Hold { pos } => todo!(),
                };
            }
        }
    }
} */
