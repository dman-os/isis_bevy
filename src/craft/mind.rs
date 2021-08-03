use deps::*;

use bevy::{
    ecs::{
        self as bevy_ecs,
        component::{ComponentDescriptor, StorageType},
    },
    prelude::*,
};
use bevy_inspector_egui::Inspectable;

use crate::craft::engine::*;
use crate::math::{Real, *};

pub struct MindPlugin;
impl Plugin for MindPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.register_component(ComponentDescriptor::new::<LinearRoutineResult>(
            StorageType::SparseSet,
        ))
        .register_component(ComponentDescriptor::new::<AngularRoutineResult>(
            StorageType::SparseSet,
        ))
        .register_component(ComponentDescriptor::new::<steering_systems::Intercept>(
            StorageType::SparseSet,
        ))
        .add_system(mind_update_engine_input.system())
        .add_system(craft_mind_smarts.system())
        .add_system(steering_systems::intercept.system());
    }
}

#[derive(Debug, Clone, Copy, Inspectable)]
pub struct MindConfig {
    pub angular_input_multiplier: Real,
}
impl Default for MindConfig {
    fn default() -> Self {
        Self {
            angular_input_multiplier: 3.,
        }
    }
}

#[derive(Bundle, Default)]
pub struct CraftMindBundle {
    pub routine_output: ActiveRoutineResult,
    pub config: MindConfig,
}

#[derive(Debug, Clone, Copy, Default, Inspectable)]
pub struct ActiveRoutineResult {
    /// local space
    lin: Vector3,
    /// local space
    ang: Vector3,
}

/// Output of linear steering routines which is usually linear velocity desired next frame in
/// fraction of [`EngineConfig:.linear_v_limit`] in world space.
#[derive(Debug, Clone, Copy, Default, Inspectable)]
pub struct LinearRoutineResult(pub Vector3);

/// Output of angular steering routines which is usually angular velocity desired next frame in local space.
#[derive(Debug, Clone, Copy, Default, Inspectable)]
pub struct AngularRoutineResult(pub Vector3);

pub fn mind_update_engine_input(
    mut crafts: Query<(
        &ActiveRoutineResult,
        &mut LinearEngineState,
        &mut AngularEngineState,
        &EngineConfig,
    )>,
) {
    crafts
        .iter_mut()
        .for_each(|(routine_res, mut lin_state, mut ang_state, config)| {
            lin_state.input = routine_res.lin * config.linear_v_limit;
            ang_state.input = routine_res.ang;
        });
}

/// As of now, we always use
pub struct ActiveRoutines(pub Entity);

/// This system sets the crafts' [`ActiveRoutineOutput`] and is decopuling layer
/// between the craft mind and whatever system is currently active. Right now, it's a dumb
/// system but later on should be replaced with some decision layer.
pub fn craft_mind_smarts(
    mut crafts: Query<(
        &mut ActiveRoutineResult,
        &ActiveRoutines,
        &MindConfig,
        &GlobalTransform,
    )>,
    lin_routines: Query<&LinearRoutineResult, With<steering_systems::Intercept>>,
    //egui_context: ResMut<bevy_egui::EguiContext>,
) {
    for (mut active_res, routine_id, config, xform) in crafts.iter_mut() {
        if let Ok(routine_res) = lin_routines.get(routine_id.0) {
            let local_lin_inp = xform.rotation.inverse() * routine_res.0;
            active_res.lin = local_lin_inp;
            active_res.ang =
                config.angular_input_multiplier * steering_systems::look_at(local_lin_inp).0;
        } else {
            tracing::error!("no routine found for craft");
        }
    }
}

pub mod steering_systems {
    use deps::*;

    use bevy::{ecs as bevy_ecs, prelude::*};
    use bevy_rapier3d::prelude::*;

    use crate::craft::engine::*;
    use crate::math::Vector3;
    //use crate::math::{Real, *};
    use super::{AngularRoutineResult, LinearRoutineResult};

    #[derive(Debug, Clone, Copy)]
    pub struct AvoidCollision {
        pub craft_entt: Entity,
        pub fwd_prediction_secs: f32,
    }

    pub fn avoid_collisoin(
        mut routines: Query<(Entity, &AvoidCollision, &mut LinearRoutineResult)>,
        crafts: Query<(&GlobalTransform, &EngineConfig)>, // crafts
    ) {
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Intercept {
        pub craft_entt: Entity,
        pub quarry_rb: RigidBodyHandle,
    }

    #[derive(Bundle)]
    pub struct InterceptRoutineBundle {
        pub param: Intercept,
        pub output: LinearRoutineResult,
    }

    pub fn intercept(
        mut routines: Query<(Entity, &Intercept, &mut LinearRoutineResult)>,
        crafts: Query<(&GlobalTransform, &EngineConfig)>, // crafts
        quarries: Query<(&RigidBodyPosition, &RigidBodyVelocity)>, // quarries
    ) {
        for (routine_id, params, mut output) in routines.iter_mut() {
            match (
                crafts.get(params.craft_entt),
                quarries.get(params.quarry_rb.entity()),
            ) {
                (Ok((xform, config)), Ok((quarry_pos, quarry_vel))) => {
                    *output = intercept_rb(quarry_pos, quarry_vel, xform, config);
                }
                err => {
                    tracing::error!(
                        "invalid params for intercept routine {:?}: {:?}",
                        routine_id,
                        err
                    );
                }
            }
        }
    }

    #[inline]
    fn intercept_rb(
        quarry_pos: &RigidBodyPosition,
        quarry_vel: &RigidBodyVelocity,
        current_xform: &GlobalTransform,
        config: &EngineConfig,
    ) -> LinearRoutineResult {
        let linear_v_limit = config.linear_v_limit;
        LinearRoutineResult(steering_behaviours::intercept_target(
            current_xform.translation,
            linear_v_limit.z,
            quarry_pos.position.translation.into(),
            quarry_vel.linvel.into(),
        ))
    }

    #[inline]
    pub fn look_at(local_dir: Vector3) -> AngularRoutineResult {
        AngularRoutineResult({
            let fwd = -Vector3::Z;
            let dir = local_dir;
            fwd.angle_between(dir) * fwd.cross(dir)
        })
        /*AngularRoutineResult({
            // invert since fwd is -Z
            let dir = -local_dir;
            let (z, x, y) = {
                //// basis facing dir
                //let t = {
                //let forward = dir.normalize();
                //let right = Vector3::Y.cross(forward).normalize();
                //let up = forward.cross(right);
                //Mat3::from_cols(right, up, forward)
                //};
                ////t.euler_angles()
                nalgebra::UnitQuaternion::face_towards(&dir.into(), &Vector3::Y.into())
                    .euler_angles()
            };
            let (x, y, z) = (z, x, y);
            Vector3::new(
                crate::math::delta_angle_radians(0., x).copysign(x),
                crate::math::delta_angle_radians(0., y).copysign(y),
                crate::math::delta_angle_radians(0., z).copysign(z),
            )
        })*/
    }

    pub mod steering_behaviours {
        use crate::math::{Real, *};

        #[inline]
        pub fn seek_position(current_pos: Vector3, target_pos: Vector3) -> Vector3 {
            (target_pos - current_pos).normalize()
        }

        #[inline]
        pub fn find_intercept_pos(
            current_pos: Vector3,
            travel_speed: Real,
            target_pos: Vector3,
            target_vel: Vector3,
        ) -> Vector3 {
            let relative_pos = target_pos - current_pos;
            let distance_to_target = relative_pos.length();
            let time_to_target_pos = distance_to_target / travel_speed;
            target_pos + (time_to_target_pos * target_vel)
        }

        #[inline]
        pub fn intercept_target(
            current_pos: Vector3,
            travel_speed: Real,
            target_pos: Vector3,
            target_vel: Vector3,
        ) -> Vector3 {
            seek_position(
                current_pos,
                find_intercept_pos(current_pos, travel_speed, target_pos, target_vel),
            )
        }
    }
}

//pub type LinearRoutineFn = dyn FnMut(GlobalTransform, LinearEngineState, EngineConfig) -> Vector3;
