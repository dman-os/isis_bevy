use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};

use super::{ActiveSteeringRoutine, LinOnlyRoutineBundle, LinearRoutineOutput, SteeringRoutine};
use crate::{craft::engine::*, math::*};

#[derive(Debug, Clone, Copy, Component)]
pub enum ArriveTarget {
    /// must have a global xform
    Object { entt: Entity, offset: TVec3 },
    /// assumed to be in world basis
    Position { pos: TVec3 },
}

#[derive(Debug, Clone, Copy, Component)]
pub struct Arrive {
    pub target: ArriveTarget,
    pub arrival_tolerance: TReal,
    pub deceleration_radius: Option<TReal>,
}

pub type ArriveRoutineBundle = LinOnlyRoutineBundle<Arrive>;

pub fn arrive(
    mut routines: Query<
        (&Arrive, &SteeringRoutine, &mut LinearRoutineOutput),
        With<ActiveSteeringRoutine>,
    >,
    crafts: Query<(&GlobalTransform, &EngineConfig)>, // crafts
    objects: Query<&GlobalTransform>,                 // quarries
) {
    for (params, routine, mut output) in routines.iter_mut() {
        let (xform, config) = crafts
            .get(routine.craft_entt)
            .expect("craft entt not found for routine");
        let pos = match params.target {
            ArriveTarget::Object { entt, offset } => match objects.get(entt) {
                Ok(obj_xform) => obj_xform.translation + offset,
                Err(err) => {
                    tracing::error!("error getting SeekTarget Object g_xform: {err:?}");
                    continue;
                }
            },
            ArriveTarget::Position { pos } => pos,
        };

        let deceleration_radius = params.deceleration_radius.unwrap_or_else(|| {
            // calclulate the radius from the max speed and avail accel
            let max_accel = {
                let max_accel = config.acceleration_limit * config.acceleration_limit_multiplier;
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
            super::steering_behaviours::dst_to_change(config.linear_v_limit.z, 0., max_accel)
        });
        *output = super::steering_behaviours::arrive_at_position(
            xform.translation,
            pos,
            // assume we're always using teh least avail accel
            params.arrival_tolerance,
            deceleration_radius,
        )
        .into();
    }
}
