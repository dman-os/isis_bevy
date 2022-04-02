use deps::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::craft::*;
use crate::mind::flock::strategy::cas::*;

use super::{
    look_to, steering_behaviours, ActiveSteeringRoutine, AngularRoutineOutput,
    CraftControllerConsts, LinAngRoutineBundle, LinearRoutineOutput, SteeringRoutine,
};

#[derive(Debug, Clone, Component)]
pub struct FlyWithFlock {
    pub flock_strategy_entt: Entity,
}

pub type Bundle = LinAngRoutineBundle<FlyWithFlock>;

pub fn update(
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
    crafts: Query<(
        &GlobalTransform,
        &RigidBodyVelocityComponent,
        &engine::EngineConfig,
        &CraftControllerConsts,
    )>, // crafts
) {
    for (param, routine, mut lin_out, mut ang_out) in routines.iter_mut() {
        let (xform, vel, config, consts) = crafts.get(routine.boid_entt).unwrap_or_log();
        let cas = strategies.get(param.flock_strategy_entt).unwrap_or_log();
        let (cohesion, allignment, separation) =
            (
                10. * steering_behaviours::cohesion(
                    xform.translation,
                    cas.member_count,
                    cas.center_sum,
                )
                .to_accel(vel.linvel.into(), config, consts),
                steering_behaviours::allignment(vel.linvel.into(), cas.member_count, cas.vel_sum)
                    .to_accel(vel.linvel.into(), config, consts),
                // NOTE: 10x multiplier
                steering_behaviours::separation(xform.translation, &cas.craft_positions[..])
                    .to_accel(vel.linvel.into(), config, consts),
            );
        *lin_out = LinearRoutineOutput::Accel(cohesion + allignment + separation);
        // *lin_out = (dir - TVec3::from(vel.linvel)).normalize_or_zero().into();
        *ang_out = look_to(xform.rotation.inverse() * allignment).into();
    }
}
