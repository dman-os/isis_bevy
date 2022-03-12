use deps::*;

use super::{ActiveSteeringRoutine, AngOnlyRoutineBundle, AngularRoutineOutput, SteeringRoutine};
use crate::math::*;
use bevy::prelude::*;

#[derive(Debug, Clone, Component)]
pub enum Target {
    /// must have a global xform
    Object { entt: Entity },
    /// assumed to be in world basis
    Direction { dir: TVec3 },
}

#[derive(Debug, Clone, Component)]
pub struct Face {
    pub target: Target,
}

pub type Bundle = AngOnlyRoutineBundle<Face>;

pub fn update(
    mut routines: Query<
        (&Face, &SteeringRoutine, &mut AngularRoutineOutput),
        With<ActiveSteeringRoutine>,
    >,
    objects: Query<&GlobalTransform>,
) {
    for (param, routine, mut output) in routines.iter_mut() {
        let xform = objects
            .get(routine.boid_entt)
            .expect_or_log("craft entt not found for routine");
        let pos = match param.target {
            Target::Object { entt } => {
                let target_pos = objects.get(entt).unwrap_or_log().translation;
                (target_pos - xform.translation).normalize()
            }
            Target::Direction { dir } => dir,
        };
        *output = super::look_to(xform.rotation.inverse() * pos).into();
    }
}
