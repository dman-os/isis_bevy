use deps::*;

use super::LinOnlyRoutineBundle;
use super::{steering_behaviours, ActiveSteeringRoutine, LinearRoutineOutput, SteeringRoutine};
use crate::math::*;
use bevy::{ecs as bevy_ecs, prelude::*};

#[derive(Debug, Clone, Component)]
pub enum SeekTarget {
    /// must have a global xform
    Object { entt: Entity },
    /// assumed to be in world basis
    Position { pos: TVec3 },
}

#[derive(Debug, Clone, Component)]
pub struct Seek {
    pub target: SeekTarget,
}

pub type SeekRoutineBundle = LinOnlyRoutineBundle<Seek>;

pub fn seek(
    mut routines: Query<
        (&Seek, &SteeringRoutine, &mut LinearRoutineOutput),
        With<ActiveSteeringRoutine>,
    >,
    objects: Query<&GlobalTransform>,
) {
    for (params, routine, mut output) in routines.iter_mut() {
        let xform = objects
            .get(routine.craft_entt)
            .expect("craft entt not found for routine");
        let pos = match params.target {
            SeekTarget::Object { entt } => match objects.get(entt) {
                Ok(obj_xform) => obj_xform.translation,
                Err(err) => {
                    tracing::error!("error getting SeekTarget Object g_xform: {err:?}");
                    continue;
                }
            },
            SeekTarget::Position { pos } => pos,
        };
        *output = steering_behaviours::seek_position(xform.translation, pos).into();
    }
}
