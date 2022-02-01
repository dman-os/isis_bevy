use deps::*;

use crate::math::*;
use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_rapier3d::prelude::*;

use self::mind::sensors::*;

pub mod arms;
pub mod attire;
pub mod engine;
pub mod mind;

pub struct CraftsPlugin;

impl Plugin for CraftsPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(engine::sync_craft_state_velocities)
            .add_system(engine::linear_pid_driver)
            .add_system(engine::angular_pid_driver)
            .add_system(engine::apply_flames_simple_accel)
            .add_plugin(attire::AttirePlugin)
            .add_plugin(mind::MindPlugin)
            .add_plugin(arms::ArmsPlugin);
    }
}

#[derive(Bundle)]
pub struct CraftBundle {
    pub xfrom: Transform,
    pub global_xform: GlobalTransform,

    #[bundle]
    pub rigid_body: RigidBodyBundle,

    pub rigid_body_sync: RigidBodyPositionSync,
    pub collision_damage_tag: attire::CollisionDamageEnabledRb,

    #[bundle]
    pub collider: attire::CollisionDamageEnabledColliderBundle,

    pub config: engine::EngineConfig,
    pub linear_state: engine::LinearEngineState,
    pub angular_state: engine::AngularEngineState,
    pub linear_pid: engine::LinearDriverPid,
    pub angular_pid: engine::AngularDriverPid,

    // indices
    pub routine_index: CraftRoutinesIndex,
    pub wpn_index: CraftWeaponsIndex,
    pub strategy_index: CraftStrategyIndex,
}

impl CraftBundle {
    pub fn default_rb_bundle() -> RigidBodyBundle {
        RigidBodyBundle {
            ccd: RigidBodyCcd {
                ccd_active: true,
                ..Default::default()
            }
            .into(),
            ..Default::default()
        }
    }
}

impl Default for CraftBundle {
    fn default() -> Self {
        Self {
            xfrom: Transform::default(),
            global_xform: GlobalTransform::default(),
            config: Default::default(),
            linear_state: Default::default(),
            angular_state: Default::default(),
            linear_pid: engine::LinearDriverPid(crate::utils::PIDControllerVec3::new(
                TVec3::ONE * 30. * 1.,
                TVec3::ZERO,
                TVec3::ZERO,
                TVec3::ZERO,
                TVec3::ZERO,
            )),
            angular_pid: engine::AngularDriverPid(crate::utils::PIDControllerVec3::new(
                TVec3::ONE * 4000.0 * 1.,
                TVec3::ZERO,
                TVec3::ZERO,
                TVec3::ZERO,
                TVec3::ZERO,
            )),
            rigid_body: Self::default_rb_bundle(),
            rigid_body_sync: RigidBodyPositionSync::Discrete,
            collision_damage_tag: attire::CollisionDamageEnabledRb,
            collider: Default::default(),
            routine_index: Default::default(),
            wpn_index: Default::default(),
            strategy_index: Default::default(),
        }
    }
}
