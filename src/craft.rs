use deps::*;

use crate::math::*;
use bevy::prelude::*;
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use bevy_rapier3d::prelude::*;

pub mod arms;
pub mod attire;
pub mod engine;

pub struct CraftsPlugin;

impl Plugin for CraftsPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(CoreStage::PreUpdate, engine::sync_craft_state_velocities)
            .add_system(engine::linear_pid_driver.before(engine::apply_flames_simple_accel))
            .add_system(engine::angular_pid_driver.before(engine::apply_flames_simple_accel))
            .add_system(engine::apply_flames_simple_accel)
            .add_plugin(attire::AttirePlugin)
            .add_plugin(arms::ArmsPlugin)
            .register_inspectable::<engine::LinearEngineState>()
            .register_inspectable::<engine::AngularEngineState>()
            .register_inspectable::<engine::EngineConfig>();
    }
}

#[derive(Bundle)]
pub struct CraftBundle {
    #[bundle]
    pub spatial: SpatialBundle,

    pub rigid_body: RigidBody,
    pub velocity: Velocity,
    pub colliders: crate::Colliders,
    pub read_mass_props: ReadMassProperties,
    pub external_force: ExternalForce,
    pub ccd: Ccd,
    /* #[bundle]
    pub rigid_body_sync: RigidBodyPositionSync, */
    pub collision_damage_tag: attire::CollisionDamageEnabledRb,

    #[bundle]
    pub collider: attire::CollisionDamageEnabledColliderBundle,

    pub config: engine::EngineConfig,
    pub derived_config: engine::DerivedEngineConfig,
    pub dimensions: CraftDimensions,
    pub linear_state: engine::LinearEngineState,
    pub angular_state: engine::AngularEngineState,
    // pub linear_pid: engine::LinearDriverPid,
    pub angular_pid: engine::AngularDriverPid,

    pub name: Name,
}

impl CraftBundle {
    pub const DEFAULT_NAME: &'static str = "craft";

    pub fn new(engine_config: engine::EngineConfig, dimensions: CraftDimensions) -> Self {
        let derived_config = engine_config.derive_items(dimensions);
        Self {
            spatial: default(),
            config: engine_config,
            derived_config,
            dimensions,
            linear_state: default(),
            angular_state: default(),
            /* linear_pid: engine::LinearDriverPid(crate::utils::PIDControllerVec3::new(
                TVec3::ONE * 30.,
                TVec3::ZERO,
                TVec3::ZERO,
                TVec3::ZERO,
                TVec3::ZERO,
            )), */
            angular_pid: engine::AngularDriverPid(crate::utils::PIDControllerVec3::new(
                TVec3::ONE * 22.0,
                TVec3::ONE * 0.0,
                TVec3::ONE,
                TVec3::ONE,
                TVec3::ONE * -10.,
            )),
            rigid_body: RigidBody::Dynamic,
            ccd: Ccd::enabled(),
            collider: default(),
            collision_damage_tag: attire::CollisionDamageEnabledRb,
            name: Self::DEFAULT_NAME.into(),
            colliders: default(),
            velocity: default(),
            read_mass_props: default(),
            external_force: default(),
        }
    }
}

/// The dimensions of the craft.
#[derive(Debug, Clone, Copy, Component, Reflect, Inspectable, educe::Educe)]
#[educe(Deref, DerefMut)]
pub struct CraftDimensions(pub TVec3);

impl From<TVec3> for CraftDimensions {
    fn from(v: TVec3) -> Self {
        Self(v)
    }
}
