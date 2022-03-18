use deps::*;

use super::{
    ActiveSteeringRoutine, AngularRoutineOutput, LinAngRoutineBundle, LinearRoutineOutput,
    SteeringRoutine,
};
use bevy::prelude::*;

#[derive(Component)]
pub struct Closure {
    pub closure: Box<
        dyn FnMut(&GlobalTransform) -> (LinearRoutineOutput, AngularRoutineOutput)
            + 'static
            + Send
            + Sync,
    >,
}

pub type Bundle = LinAngRoutineBundle<Closure>;

pub fn update(
    mut routines: Query<
        (
            &mut Closure,
            &SteeringRoutine,
            &mut LinearRoutineOutput,
            &mut AngularRoutineOutput,
        ),
        With<ActiveSteeringRoutine>,
    >,
    objects: Query<&GlobalTransform>,
) {
    for (mut param, routine, mut lin_out, mut ang_out) in routines.iter_mut() {
        let xform = objects
            .get(routine.boid_entt)
            .expect_or_log("craft entt not found for routine");
        let (lin, ang) = (param.closure)(xform);
        *lin_out = lin;
        *ang_out = ang;
    }
}
