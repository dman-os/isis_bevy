use deps::*;

use bevy::prelude::*;
// use bevy_prototype_debug_lines::*;
use bevy_rapier3d::prelude::*;

use super::{ActiveSteeringRoutine, LinOnlyRoutineBundle, LinearRoutineOutput, SteeringRoutine};
use crate::math::*;

/// All vectors are in in world basis
#[derive(Debug, Clone, Component)]
pub enum Target {
    /// must have a [`GlobalTransform`] component.
    // Object { entt: Entity, offset: TVec3 },
    Vector {
        at_pos: TVec3,
        // with_linvel: TVec3,
        with_speed: TReal,
        pos_linvel: TVec3,
    },
}

#[derive(Debug, Clone, Component)]
pub struct Arrive {
    pub target: Target,
    pub arrival_tolerance: TReal,
    /// If not given, will be calculated based on accel and linvel_limit
    pub deceleration_radius: Option<TReal>,
    pub avail_accel: TVec3,
}

/*
use crate::utils::*;
#[derive(Debug, Clone, Component)]
pub struct ArriveState {
    pid: PIDControllerVec3,
}

impl Default for ArriveState {
    fn default() -> Self {
        Self {
            pid: PIDControllerVec3::new(
                TVec3::ONE * 1.0,
                TVec3::ONE * 0.,
                TVec3::ONE * 0.,
                TVec3::ONE * 0.,
                TVec3::ONE * 0.,
            ),
        }
    }
} */

pub type Bundle = LinOnlyRoutineBundle<Arrive>;

pub fn update(
    mut routines: Query<
        (
            &SteeringRoutine,
            &Arrive,
            // &mut ArriveState,
            &mut LinearRoutineOutput,
        ),
        With<ActiveSteeringRoutine>,
    >,
    boids: Query<(&Transform, &Velocity)>, // boids
                                           // objects: Query<&GlobalTransform>,
                                           // mut lines: ResMut<DebugLines>,
) {
    for (routine, param, mut output) in routines.iter_mut() {
        let (xform, vel) = boids
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
            Target::Vector {
                at_pos,
                // with_linvel,
                pos_linvel,
                with_speed,
            } => {
                /* super::steering_behaviours::seek_position(
                    xform.translation,
                    at_pos,
                ) */
                let max_accel = xform.rotation * param.avail_accel;
                let vel = vel.linvel;
                let target_offset = (at_pos + pos_linvel) - xform.translation;
                let target_spd = pos_linvel.length() + with_speed;

                let accel = max_accel.project_onto_scalar(target_offset).abs();
                let vel_to_target = vel.project_onto(target_offset);
                let spd_to_target = vel_to_target.project_onto_scalar(target_offset);

                super::steering_behaviours::arrive_at_position(
                    xform.translation,
                    at_pos + pos_linvel,
                    vel,
                    target_spd,
                    target_spd,
                    param.arrival_tolerance,
                    {
                        super::steering_behaviours::dst_to_change(spd_to_target, target_spd, accel)
                        // .max_element()
                    },
                )
                /*  super::steering_behaviours::be_ray(
                    at_pos,
                    // with_linvel,
                    TVec3::Z * -90.,
                    pos_linvel,
                    xform.translation,
                    vel.linvel.into(),
                    param.avail_accel,
                    param.linvel_limit,
                    xform.rotation,
                    &mut |v1, v2| lines.line(v1, v2, 0.),
                )
                .into() */
            }
        };
    }
}
