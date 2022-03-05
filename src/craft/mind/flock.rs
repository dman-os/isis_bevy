use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_rapier3d::prelude::*;

use crate::math::*;

#[derive(Debug, Default, Component)]
pub struct FlockMind {
    pub members: smallvec::SmallVec<[Entity; 8]>,
    pub hostile_contacts: smallvec::SmallVec<[Entity; 8]>,
}

#[derive(Debug, Clone, Copy, Component)]
pub struct CraftGroup(pub Entity);

#[derive(Debug, Default, Component)]
pub struct BoidFlock {
    pub craft_positions: Vec<TVec3>,
    pub vel_sum: TVec3,
    pub avg_vel: TVec3,
    pub center_sum: TVec3,
    pub center: TVec3,
    pub member_count: usize,
}

pub fn update_flocks(
    mut flocks: Query<(&FlockMind, &mut BoidFlock)>,
    crafts: Query<(&GlobalTransform, &RigidBodyVelocityComponent)>,
) {
    for (g_mind, mut flock) in flocks.iter_mut() {
        flock.craft_positions.clear();
        flock.vel_sum = TVec3::ZERO;
        flock.center_sum = TVec3::ZERO;
        for craft in g_mind.members.iter() {
            if let Ok((xform, vel)) = crafts.get(*craft) {
                flock.vel_sum += TVec3::from(vel.linvel);
                flock.center_sum += xform.translation;
                flock.craft_positions.push(xform.translation);
            } else {
                tracing::error!("unable to find group mind member when updating flocks");
            }
        }
        flock.member_count = g_mind.members.len();
        flock.avg_vel = flock.vel_sum / g_mind.members.len() as TReal;
        flock.center = flock.center_sum / g_mind.members.len() as TReal;
    }
}
