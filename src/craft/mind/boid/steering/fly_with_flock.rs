use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_rapier3d::prelude::*;

use crate::craft::mind::flock::strategy::CASState;

use super::{
    look_to, steering_behaviours, ActiveSteeringRoutine, AngularRoutineOutput, LinAngRoutineBundle,
    LinearRoutineOutput, SteeringRoutine,
};

#[derive(Debug, Clone, Copy, Component)]
pub struct FlyWithFlock {
    pub strategy_entt: Entity,
}

pub type FlyWithFlockRoutineBundle = LinAngRoutineBundle<FlyWithFlock>;

pub fn fly_with_flock(
    mut routines: Query<
        (
            &FlyWithFlock,
            &SteeringRoutine,
            &mut LinearRoutineOutput,
            &mut AngularRoutineOutput,
        ),
        With<ActiveSteeringRoutine>,
    >,
    strategies: Query<&CASState>,
    crafts: Query<(&GlobalTransform, &RigidBodyVelocityComponent)>, // crafts
) {
    for (params, routine, mut lin_out, mut ang_out) in routines.iter_mut() {
        let (xform, vel) = crafts
            .get(routine.craft_entt)
            .expect("craft entt not found for routine");
        let cas = strategies
            .get(params.strategy_entt)
            .expect("unable to find craft_group for fly_with_flock routine");
        let (cohesion, allignment, separation) = (
            steering_behaviours::cohesion(xform.translation, cas.member_count, cas.center_sum),
            steering_behaviours::allignment(vel.linvel.into(), cas.member_count, cas.vel_sum),
            // NOTE: 10x multiplier
            10.0 * steering_behaviours::separation(xform.translation, &cas.craft_positions[..]),
        );
        *lin_out = (cohesion + allignment + separation).into();
        *ang_out = look_to(xform.rotation * allignment).into();
    }
}
