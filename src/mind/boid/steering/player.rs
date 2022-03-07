use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};

use crate::mind::*;

use super::{
    ActiveSteeringRoutine, AngularRoutineOutput, LinAngRoutineBundle, LinearRoutineOutput,
};

#[derive(Debug, Clone, Component)]
pub struct Player;

pub type Bundle = LinAngRoutineBundle<Player>;

#[allow(clippy::type_complexity)]
pub fn update(
    player_input: Res<player::PlayerBoidInput>,
    mut routines: Query<
        (&mut LinearRoutineOutput, &mut AngularRoutineOutput),
        (With<ActiveSteeringRoutine>, With<Player>),
    >,
) {
    for (mut lin_out, mut ang_out) in routines.iter_mut() {
        *lin_out = player_input.engine_lin().into();
        *ang_out = player_input.engine_ang().into();
    }
}
