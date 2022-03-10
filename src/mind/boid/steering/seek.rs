use deps::*;

use super::LinOnlyRoutineBundle;
use super::{steering_behaviours, ActiveSteeringRoutine, LinearRoutineOutput, SteeringRoutine};
use crate::math::*;
use bevy::{ prelude::*};

#[derive(Debug, Clone, Component)]
pub enum Target {
    /// must have a global xform
    Object { entt: Entity },
    /// assumed to be in world basis
    Position { pos: TVec3 },
}

#[derive(Debug, Clone, Component)]
pub struct Seek {
    pub target: Target,
}

pub type Bundle = LinOnlyRoutineBundle<Seek>;

pub fn update(
    mut routines: Query<
        (&Seek, &SteeringRoutine, &mut LinearRoutineOutput),
        With<ActiveSteeringRoutine>,
    >,
    objects: Query<&GlobalTransform>,
) {
    for (param, routine, mut output) in routines.iter_mut() {
        let xform = objects
            .get(routine.craft_entt)
            .expect_or_log("craft entt not found for routine");
        let pos = match param.target {
            Target::Object { entt } => match objects.get(entt) {
                Ok(obj_xform) => obj_xform.translation,
                Err(err) => {
                    tracing::error!("error getting SeekTarget Object g_xform: {err:?}");
                    continue;
                }
            },
            Target::Position { pos } => pos,
        };
        *output = steering_behaviours::seek_position(xform.translation, pos).into();
    }
}