use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_rapier3d::prelude::*;

use super::{ActiveSteeringRoutine, LinOnlyRoutineBundle, LinearRoutineOutput, SteeringRoutine};
use crate::{craft::engine::*, math::*};

#[derive(Debug, Clone, Component)]
pub struct Intercept {
    pub quarry_rb: RigidBodyHandle,
    /// Will use the craft engine's config if None.
    pub speed: Option<TReal>,
}

pub type Bundle = LinOnlyRoutineBundle<Intercept>;

pub fn update(
    mut routines: Query<
        (&Intercept, &SteeringRoutine, &mut LinearRoutineOutput),
        With<ActiveSteeringRoutine>,
    >,
    crafts: Query<(&GlobalTransform, &EngineConfig)>, // crafts
    quarries: Query<(&GlobalTransform, &RigidBodyVelocityComponent)>, // quarries
) {
    for (param, routine, mut output) in routines.iter_mut() {
        let (xform, config) = crafts
            .get(routine.craft_entt)
            .expect("craft entt not found for routine");
        let (quarry_xform, quarry_vel) = quarries
            .get(param.quarry_rb.entity())
            .expect("quarry rigid body not found for on Intercept routine");
        let speed = param.speed.unwrap_or(config.linear_v_limit.z);
        *output = super::steering_behaviours::intercept_target(
            xform.translation,
            speed,
            quarry_xform.translation,
            quarry_vel.linvel.into(),
        )
        .into();
    }
}
