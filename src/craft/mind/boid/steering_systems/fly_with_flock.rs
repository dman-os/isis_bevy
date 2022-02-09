use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_rapier3d::prelude::*;

use crate::craft::mind::{BoidFlock, CraftGroup};

use super::{
    look_to, steering_behaviours, ActiveRoutine, AngularRoutineOutput, LinAngRoutineBundle,
    LinearRoutineOutput, SteeringRoutine,
};

#[derive(Debug, Clone, Copy, Component)]
pub struct FlyWithFlock;

pub type FlyWithFlockRoutineBundle = LinAngRoutineBundle<FlyWithFlock>;

pub fn fly_with_flock(
    mut routines: Query<
        (
            &FlyWithFlock,
            &SteeringRoutine,
            &mut LinearRoutineOutput,
            &mut AngularRoutineOutput,
        ),
        With<ActiveRoutine>,
    >,
    flocks: Query<&BoidFlock>,
    crafts: Query<(&GlobalTransform, &RigidBodyVelocityComponent, &CraftGroup)>, // crafts
) {
    for (_params, routine, mut lin_out, mut ang_out) in routines.iter_mut() {
        let (xform, vel, craft_group) = crafts
            .get(routine.craft_entt)
            .expect("craft entt not found for routine");
        let flock = flocks
            .get(craft_group.0)
            .expect("unable to find craft_group for fly_with_flock routine");
        let (cohesion, allignment, separation) = (
            steering_behaviours::cohesion(xform.translation, flock.member_count, flock.center_sum),
            steering_behaviours::allignment(
                vel.linvel.into(),
                flock.member_count,
                flock.heading_sum,
            ),
            // NOTE: 10x multiplier
            10.0 * steering_behaviours::separation(xform.translation, &flock.craft_positions[..]),
        );
        *lin_out = LinearRoutineOutput(cohesion + allignment + separation);
        *ang_out = AngularRoutineOutput(look_to(xform.rotation * allignment));
    }
}
