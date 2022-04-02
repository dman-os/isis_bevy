use deps::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use super::{ActiveSteeringRoutine, LinOnlyRoutineBundle, LinearRoutineOutput, SteeringRoutine};
use crate::math::*;

#[derive(Debug, Clone, Component)]
pub struct Intercept {
    pub quarry_rb: RigidBodyHandle,
    /// Will use the craft engine's config if None.
    pub speed: Option<TReal>,
    pub linvel_limit: TVec3,
}

pub type Bundle = LinOnlyRoutineBundle<Intercept>;

pub fn update(
    mut routines: Query<
        (&Intercept, &SteeringRoutine, &mut LinearRoutineOutput),
        With<ActiveSteeringRoutine>,
    >,
    boids: Query<(&GlobalTransform, &RigidBodyVelocityComponent)>,
) {
    for (param, routine, mut output) in routines.iter_mut() {
        let (xform, _) = boids
            .get(routine.boid_entt)
            .expect_or_log("craft entt not found for routine");
        let (quarry_xform, quarry_vel) = boids
            .get(param.quarry_rb.entity())
            .expect_or_log("quarry rigid body not found for on Intercept routine");
        let travel_speed = param.speed.unwrap_or(param.linvel_limit.z);
        *output = super::steering_behaviours::intercept_target(
            xform.translation,
            travel_speed,
            quarry_xform.translation,
            quarry_vel.linvel.into(),
        );
        // *output = (dir - TVec3::from(vel.linvel)).normalize_or_zero().into();
    }
}
