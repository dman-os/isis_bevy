use deps::*;

use bevy::prelude::*;
use bevy_inspector_egui::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::craft::CraftDimensions;
use crate::math::*;

#[derive(Debug, Default, Clone, Component, Reflect, Inspectable)]
pub struct LinearEngineState {
    /// Linear velocity in local-space
    /// In m/s.
    pub velocity: TVec3,

    /// Input vector for driver. Meaning depends on driver implementation.
    /// e.g. target velocity to attain
    pub input: TVec3,

    /// Vector output of driver and input vector of a motor. Meaning depends on implementation.
    /// e.g. acceleration to apply
    pub flame: TVec3,
}

#[derive(Debug, Default, Clone, Component, Reflect, Inspectable)]
pub struct AngularEngineState {
    /// Angular velocity in local-space
    /// In rad/s.
    pub velocity: TVec3,

    /// Input vector for driver. Meaning depends on driver implementation.
    /// e.g. target velocity to attain
    pub input: TVec3,

    /// Vector output of driver and input vector of a motor. Meaning depends on implementation.
    pub flame: TVec3,
}

// TODO: break this up to multiple components. Maybe along the line of what's likely to mutate?
#[derive(Debug, Clone, Component, Reflect, Inspectable)]
pub struct EngineConfig {
    /// Total mass of the craft.
    /// In KG.
    pub mass: TReal,

    /// Maximum acceleration allowed to the craft.
    /// In m/s.
    pub acceleration_limit: TVec3,

    pub acceleration_limit_multiplier: TReal,

    /// Linear velocity cap no matter the input.
    /// In m/s.
    pub linvel_limit: TVec3,

    /// Angular velocity cap no matter the input.
    /// In rad/s.
    pub angvel_limit: TVec3,

    /// Max force the linear thrusters are capable of exerting.
    /// In Newtons.
    pub linear_thruster_force: TVec3,

    // FIXME: move this to the mind layer
    /// Whether or not to respect linvel_limit in the z axis.
    pub limit_forward_v: bool,

    // FIXME: move this to the mind layer
    /// Whether or not to respect linvel_limit in in the X or Y axis.
    pub limit_strafe_v: bool,

    // FIXME: move this to the mind layer
    /// Whether or not to respect angvel_limit.
    pub limit_angular_v: bool,

    ///  Whether or not to respect acceleration_limit.
    pub limit_acceleration: bool,

    /// Max force the angular thrusters are capable of exerting.
    /// In Newtons.
    pub angular_thruster_force: TVec3,

    pub thruster_force_multiplier: TReal,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            mass: 15_000.,
            acceleration_limit: [6., 6., 6.].into(),
            acceleration_limit_multiplier: 9.81,
            // matters not if v_limit.z is negative since this's a limit
            linvel_limit: [100., 100., 200.].into(),
            angvel_limit: [3., 3., 3.].into(),
            limit_forward_v: true,
            limit_strafe_v: true,
            limit_angular_v: true,
            limit_acceleration: true,
            linear_thruster_force: [1., 1., 1.5].into(),
            angular_thruster_force: [1., 1., 1.].into(),
            thruster_force_multiplier: 1_000_000.0,
        }
    }
}

impl EngineConfig {
    /// Use this everytime the [`EngineConfig`] or [`CraftDimensions`] changes to calculate transiet items.
    pub fn derive_items(&self, dimensions: CraftDimensions) -> DerivedEngineConfig {
        use bevy::math::vec2;
        // should I be doubling this?
        let axes_bounds: TVec3 = [
            vec2(dimensions.y, dimensions.z).length(),
            vec2(dimensions.x, dimensions.z).length(),
            vec2(dimensions.x, dimensions.y).length(),
        ]
        .into();

        DerivedEngineConfig {
            // TODO: proper angular accel limits
            angular_acceleration_limit: [TReal::INFINITY; 3].into(),
            thruster_torque: axes_bounds * self.angular_thruster_force,
        }
    }

    #[inline]
    pub fn actual_accel_limit(&self) -> TVec3 {
        self.acceleration_limit * self.acceleration_limit_multiplier
    }

    /// This doesn't take into account the [`acceleration_limit`]. Clapmp it yourself.
    #[inline]
    pub fn avail_lin_accel(&self) -> TVec3 {
        let max_force = self.linear_thruster_force * self.thruster_force_multiplier;
        max_force / self.mass
    }
}

#[derive(Debug, Clone, Component, Reflect, Inspectable)]
pub struct DerivedEngineConfig {
    /// Angular thruster toruqe, transient auto cacluated value from the
    /// angular_thrustuer_force according to the craft's shape and mass.
    /// In  Newton meters.
    pub thruster_torque: TVec3,

    /// Angular acceleration limit, another transient auto cacluated value. It's cacluated from
    /// the normal acceleration limit (which is in m/ss) and adjusted to the size/shape of the craft.
    /// In rad/s/s.
    ///
    /// Curretly unused. Defaults to INFINITY meaning there's no artifical acceleration_limit on
    /// the crafts. They use all of what's availaible from the thrusters.
    pub angular_acceleration_limit: TVec3,
    ///// Moment of inertia, transient auto cacluated value used to convert the required angular
    ///// acceleration into the appropriate torque. Aquried directly from Godot's physics engine.
    ///// In  kg*m*m.
    ///// Defaults to one to avoid hard to track division by zero errors. The moi is asychronously
    ///// retrieved from the engine and some frames pass before it happens. Time enough for the NANs
    ///// to propagate EVERYWHERE!
    //pub moment_of_inertia: Vector3,
}

/* #[derive(Debug, Component, educe::Educe)]
#[educe(Deref, DerefMut)]
pub struct LinearDriverPid(pub crate::utils::PIDControllerVec3); */
#[derive(Debug, Component, educe::Educe)]
#[educe(Deref, DerefMut)]
pub struct AngularDriverPid(pub crate::utils::PIDControllerVec3);

pub fn sync_craft_state_velocities(
    mut crafts: Query<(
        &mut AngularEngineState,
        &mut LinearEngineState,
        &GlobalTransform,
        &Velocity,
    )>,
) {
    for (mut angular_state, mut linear_state, g_xform, vel) in crafts.iter_mut() {
        let g_xform = g_xform.compute_transform();
        // convert it to local space first
        let rotator = g_xform.rotation.inverse();
        linear_state.velocity = rotator * vel.linvel;
        angular_state.velocity = rotator * vel.angvel;
    }
}

// Currently assumes the inputs are acceleration
pub fn linear_pid_driver(
    mut crafts: Query<(&mut LinearEngineState, &EngineConfig)>,
    //time: Time,
) {
    for (mut state, config) in crafts.iter_mut() {
        let desired_accel = state.input;

        // calculate max acceleration possible using availaible force
        let accel_limit = {
            let mut accel_limit = config.avail_lin_accel();

            // NOTE: fwd is negative bc rh coord sys
            let move_fwd = desired_accel.z < 0.0;

            // if input wants to go bacwards
            if !move_fwd {
                // only use starfe thrusters force on the z
                accel_limit.z = accel_limit.x.max(accel_limit.y);
            }

            // FIXME: move this to the mind layer
            if config.limit_acceleration {
                let artificial_accel_limit = config.actual_accel_limit();

                // clamp the actual limit to the artifical limit
                accel_limit.clamp(-artificial_accel_limit, artificial_accel_limit)
            } else {
                accel_limit
            }
        };

        let desired_accel = desired_accel.clamp(-accel_limit, accel_limit);
        state.flame = desired_accel;

        /* let desired_vel = state.velocity + desired_accel;

        // if dampeners are on
        let desired_vel = if config.limit_strafe_v {
            let v_limit = config.linvel_limit;

            // clamp the input to the limit
            let mut clamped_v = desired_vel.clamp(-v_limit, v_limit);

            if !config.limit_forward_v {
                clamped_v.z = desired_vel.z;
            }
            clamped_v
        } else {
            desired_vel
        };

        let linear_flame = pid.update(state.velocity, desired_vel - state.velocity, 1.);

        state.flame = linear_flame.clamp(-accel_limit, accel_limit); */
    }
}

// Currently assumes the inputs are acceleration
pub fn angular_pid_driver(
    mut crafts: Query<(
        &mut AngularEngineState,
        &EngineConfig,
        &DerivedEngineConfig,
        &mut AngularDriverPid,
        &ReadMassProperties,
    )>,
    // time: Res<Time>,
) {
    for (mut state, config, derived_config, mut pid, mass_props) in crafts.iter_mut() {
        {
            let accel_limit = {
                let max_torque = derived_config.thruster_torque * config.thruster_force_multiplier;
                // TODO: work out if this is actually the inertia tensor
                // NOTICE: difference here
                // torque = Inertial_tensor * rotational acceleration
                // accel = torque / Inertial_tensor

                let inv_inertial_tensor = mass_props
                    .0
                    .into_rapier(1.)
                    .reconstruct_inverse_inertia_matrix();
                let accel_limit: TVec3 =
                    (inv_inertial_tensor * bevy_rapier3d::na::Vector3::from(max_torque)).into();
                // let accel_limit = max_torque * TVec3::from(mass_props.0.into_rapier(1.).inv_principal_inertia_sqrt);
                if config.limit_acceleration {
                    let artificial_accel_limit = derived_config.angular_acceleration_limit;
                    pid.0.integrat_max = accel_limit.min(artificial_accel_limit);
                    pid.0.integrat_min = -pid.0.integrat_max;

                    // clamp the actual limit to the artifical limit

                    accel_limit.clamp(-artificial_accel_limit, artificial_accel_limit)
                } else {
                    accel_limit
                }
            };

            let desired_accel = state.input.clamp(-accel_limit, accel_limit);

            let desired_vel = state.velocity + desired_accel;

            let desired_vel = if config.limit_angular_v {
                desired_vel.clamp(-config.angvel_limit, config.angvel_limit)
            } else {
                desired_vel
            };

            let angular_flame = pid.update(
                state.velocity,
                desired_vel - state.velocity,
                1.0, // time.delta_seconds(),
            );

            state.flame = angular_flame.clamp(-accel_limit, accel_limit);
        }
    }
}

/// Currenlty assumes the flames are acceleration
pub fn apply_flames_simple_accel(
    mut crafts: Query<(
        &GlobalTransform,
        &LinearEngineState,
        &AngularEngineState,
        &EngineConfig,
        &ReadMassProperties,
        &mut ExternalForce,
    )>,
    // time: Res<Time>,
) {
    for (g_xform, lin_state, ang_state, config, mass_props, mut ext_force) in crafts.iter_mut() {
        let g_xform = g_xform.compute_transform();
        let force = lin_state.flame * config.mass;
        let force = g_xform.rotation * force;
        ext_force.force = force;

        // sqrt this?
        let inertial_tensor = mass_props.0.into_rapier(1.).reconstruct_inertia_matrix();
        let torque: TVec3 =
            (inertial_tensor * bevy_rapier3d::na::Vector3::from(ang_state.flame)).into();
        // let torque = ang_state.flame / TVec3::from(mass_props.0.into_rapier(1.).inv_principal_inertia_sqrt);
        let torque = g_xform.rotation * torque;

        ext_force.torque = torque;
    }
}
