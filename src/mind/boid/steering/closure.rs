use deps::*;

use bevy::prelude::*;

use crate::craft::*;

use super::{
    ActiveSteeringRoutine, AngularRoutineOutput, LinAngRoutineBundle, LinearRoutineOutput,
    SteeringRoutine,
};

#[derive(Component)]
pub struct Closure {
    pub closure: Box<
        dyn FnMut(
                &GlobalTransform,
                &engine::LinearEngineState,
                &engine::AngularEngineState,
            ) -> (LinearRoutineOutput, AngularRoutineOutput)
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
    crafts: Query<(
        &GlobalTransform,
        &engine::LinearEngineState,
        &engine::AngularEngineState,
    )>,
) {
    for (mut param, routine, mut lin_out, mut ang_out) in routines.iter_mut() {
        let (xform, lin_state, ang_state) = crafts
            .get(routine.boid_entt)
            .expect_or_log("craft entt not found for routine");
        let (lin, ang) = (param.closure)(xform, lin_state, ang_state);
        *lin_out = lin;
        *ang_out = ang;
    }
}
