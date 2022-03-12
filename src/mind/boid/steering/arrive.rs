use deps::*;

use bevy::prelude::*;
// use bevy_prototype_debug_lines::*;
// use bevy_rapier3d::prelude::*;

use super::{ActiveSteeringRoutine, LinOnlyRoutineBundle, LinearRoutineOutput, SteeringRoutine};
use crate::math::*;

/// All vectors are in in world basis
#[derive(Debug, Clone, Component)]
pub enum Target {
    /// must have a [`GlobalTransform`] component.
    // Object { entt: Entity, offset: TVec3 },
    Position {
        pos: TVec3,
        /// on a best effort basis
        speed: TReal,
    },
}

#[derive(Debug, Clone, Component)]
pub struct Arrive {
    pub target: Target,
    pub arrival_tolerance: TReal,
    pub deceleration_radius: Option<TReal>,
    pub accel_limit: TVec3,
    pub linvel_limit: TVec3,
}

pub type Bundle = LinOnlyRoutineBundle<Arrive>;

pub fn update(
    mut routines: Query<
        (&Arrive, &SteeringRoutine, &mut LinearRoutineOutput),
        With<ActiveSteeringRoutine>,
    >,
    boids: Query<(&GlobalTransform,)>, // boids
                                       // objects: Query<&GlobalTransform>,
                                       // mut lines: ResMut<DebugLines>,
) {
    for (param, routine, mut output) in routines.iter_mut() {
        let (xform,) = boids
            .get(routine.boid_entt)
            .expect_or_log("craft entt not found for routine");
        *output = match param.target {
            /* Target::Object { entt, offset } => match objects.get(entt) {
                Ok(obj_xform) => obj_xform.translation + offset,
                Err(err) => {
                    tracing::error!("error getting SeekTarget Object g_xform: {err:?}");
                    continue;
                }
            }, */
            Target::Position {
                pos,
                speed: target_spd,
            } => {
                // let world_vel = vel.linvel.into();
                // let target_vel = target_spd * -TVec3::Z;
                // calclulate the radius from the max speed and avail accel
                let deceleration_radius = param.deceleration_radius.unwrap_or_else(|| {
                    let max_accel = {
                        let max_accel = param.accel_limit;
                        let mut min_a = max_accel[0];
                        for ii in [1, 2] {
                            let a = max_accel[ii];
                            // if acceleration is disabled in that direction,
                            if a < TReal::EPSILON {
                                // ignore it
                                continue;
                            }
                            min_a = min_a.min(a);
                        }
                        min_a
                    };
                    /* let max_accel =
                    config.acceleration_limit * config.acceleration_limit_multiplier; */
                    // FIXME: using the limit vel is too conservative, too slow in the final leg
                    super::steering_behaviours::dst_to_change(
                        param.linvel_limit.z,
                        target_spd,
                        max_accel,
                    )
                    /* let time_to_change = {
                        let accel = max_accel;
                        let delta = world_vel - target_vel;
                        let time: TVec3 = delta / accel;
                        time.abs()
                    };
                    let avg_vel = (world_vel + target_vel) * 0.5;
                    let dst: TVec3 = avg_vel * time_to_change;
                    dst.max_element() */
                    // super::steering_behaviours::dst_to_change(vel, 0., max_accel)
                });
                super::steering_behaviours::arrive_at_position(
                    xform.translation,
                    pos,
                    target_spd,
                    param.linvel_limit.z,
                    param.arrival_tolerance,
                    deceleration_radius,
                )
                .into()
                /* super::steering_behaviours::arrive_at_vector(
                    xform.translation,
                    pos,
                    world_vel,
                    target_vel,
                    deceleration_radius,
                    config.linvel_limit,
                )
                .into() */
            }
        };
    }
}
