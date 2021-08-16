use deps::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::math::*;

#[derive(Debug, Default, Clone)]
pub struct LinearEngineState {
    /// Linear velocity in local-space
    /// In m/s.
    pub velocity: TVec3,

    /// Input vector for driver. Meaning depends on driver implementation.
    /// e.g. target velocity to attain
    pub input: TVec3,

    /// Vector output of driver and input vector of a motor. Meaning depends on implementation.
    /// e.g. forve to apply
    pub flame: TVec3,
}

#[derive(Debug, Default, Clone)]
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

// TODO: break this up to multiple components
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(crate = "serde")]
pub struct EngineConfig {
    ///  Speed to travel at when there is no input i.e. how fast to travel when idle.
    pub set_speed: TVec3,

    /// Total mass of the craft.
    /// In KG.
    pub mass: TReal,

    /// Maximum acceleration allowed to the craft.
    /// In m/s.
    pub acceleration_limit: TVec3,

    pub acceleration_limit_multiplier: TReal,

    /// Linear velocity cap no matter the input.
    /// In m/s.
    pub linear_v_limit: TVec3,

    /// Angular velocity cap no matter the input.
    /// In rad/s.
    pub angular_v_limit: TVec3,

    /// Max force the linear thrusters are capable of exerting.
    /// In Newtons.
    pub linear_thruster_force: TVec3,

    /// Whether or not to respect linear_v_limit in the z axis.
    pub limit_forward_v: bool,

    /// Whether or not to respect linear_v_limit in in the X or Y axis.
    pub limit_strafe_v: bool,

    /// Whether or not to respect angular_v_limit.
    pub limit_angular_v: bool,

    ///  Whether or not to respect acceleration_limit.
    pub limit_acceleration: bool,

    /// Max force the angular thrusters are capable of exerting.
    /// In Newtons.
    pub angular_thruster_force: TVec3,

    pub thruster_force_multiplier: TReal,

    /// The dimensions of the craft.
    /// TODO: extract this to own component
    pub extents: TVec3,

    /// DERIVED ITEMS

    // TODO: make use of me
    /// Angular thruster toruqe, transient auto cacluated value from the
    /// angular_thrustuer_force according to the craft's shape and mass.
    /// In  Newton meters.
    pub thruster_torque: Option<TVec3>,
    /// Angular acceleration limit, another transient auto cacluated value. It's cacluated from
    /// the normal acceleration limit (which is in m/ss) and adjusted to the size/shape of the craft.
    /// In rad/s/s.
    ///
    /// Curretly unused. Defaults to INFINITY meaning there's no artifical acceleration_limit on
    /// the crafts. They use all of what's availaible from the thrusters.
    pub angular_acceleration_limit: Option<TVec3>,
    ///// Moment of inertia, transient auto cacluated value used to convert the required angular
    ///// acceleration into the appropriate torque. Aquried directly from Godot's physics engine.
    ///// In  kg*m*m.
    ///// Defaults to one to avoid hard to track division by zero errors. The moi is asychronously
    ///// retrieved from the engine and some frames pass before it happens. Time enough for the NANs
    ///// to propagate EVERYWHERE!
    //pub moment_of_inertia: Vector3,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            mass: 15_000.,
            set_speed: TVec3::ZERO,
            acceleration_limit: [6., 6., 6.].into(),
            acceleration_limit_multiplier: 9.81,
            // matters not if v_limit.z is negative since this's a limit
            linear_v_limit: [100., 100., 200.].into(),
            angular_v_limit: [3., 3., 3.].into(),
            limit_forward_v: true,
            limit_strafe_v: true,
            limit_angular_v: true,
            limit_acceleration: true,
            linear_thruster_force: [1., 1., 1.5].into(),
            angular_thruster_force: [1., 1., 1.].into(),
            thruster_force_multiplier: 1_000_000.0,
            extents: TVec3::ONE * 8.0,
            thruster_torque: None,
            angular_acceleration_limit: None,
        }
        .derive_items()
    }
}

impl EngineConfig {
    /// Use this everytime the config changes to calculate transiet items,
    pub fn derive_items(mut self) -> Self {
        self.angular_acceleration_limit = Some([TReal::INFINITY; 3].into());

        use bevy::math::vec2;
        let axes_diameter: TVec3 = [
            vec2(self.extents.y, self.extents.z).length(),
            vec2(self.extents.x, self.extents.z).length(),
            vec2(self.extents.x, self.extents.y).length(),
        ]
        .into();

        self.thruster_torque = Some(axes_diameter * self.angular_thruster_force);
        self
    }
}

#[derive(Debug)]
pub struct LinearDriverPid(pub crate::utils::PIDControllerVec3);
#[derive(Debug)]
pub struct AngularDriverPid(pub crate::utils::PIDControllerVec3);

pub fn sync_craft_state_velocities(
    mut crafts: Query<(
        &mut AngularEngineState,
        &mut LinearEngineState,
        &GlobalTransform,
        &RigidBodyVelocity,
    )>,
) {
    for (mut angular_state, mut linear_state, g_xform, rb_velocity) in crafts.iter_mut() {
        // convert it to local space first
        let rotator = g_xform.rotation.inverse();
        angular_state.velocity = rotator * TVec3::from(rb_velocity.angvel);
        linear_state.velocity = rotator * TVec3::from(rb_velocity.linvel);
    }
}

pub fn linear_pid_driver(
    mut crafts: Query<(&mut LinearEngineState, &EngineConfig, &mut LinearDriverPid)>,
    //time: Time,
) {
    for (mut state, config, mut pid) in crafts.iter_mut() {
        let mut linear_input = state.input;

        // if dampeners are on
        if config.limit_strafe_v {
            let v_limit = config.linear_v_limit;

            // clamp the input to the limit
            linear_input = linear_input.clamp(-v_limit, v_limit);

            // if forward dampenere is off
            if !config.limit_forward_v {
                // restore the clamped input on the z
                linear_input.z = state.input.z;
            }
        }

        let mut max_force = config.linear_thruster_force * config.thruster_force_multiplier;

        // NOTE: fwd is negative bc rh coord sys
        let move_fwd = linear_input.z < 0.0;
        // if input wants to go bacwards
        if !move_fwd {
            // only use starfe thrusters force on the z
            max_force.z = max_force.x;
        }

        // calculate max acceleration possible using availaible force
        let mut acceleration_limit = max_force / config.mass;

        if config.limit_acceleration {
            let artificial_accel_limit =
                config.acceleration_limit * config.acceleration_limit_multiplier;

            // clamp the actual limit to the artifical limit
            acceleration_limit =
                acceleration_limit.clamp(-artificial_accel_limit, artificial_accel_limit);
        }

        let linear_flame = pid
            .0
            .update(state.velocity, linear_input - state.velocity, 1.);

        let linear_flame = linear_flame.clamp(-acceleration_limit, acceleration_limit);

        state.flame = linear_flame;
    }
}

pub fn angular_pid_driver(
    mut crafts: Query<(
        &mut AngularEngineState,
        &EngineConfig,
        &mut AngularDriverPid,
        &RigidBodyMassProps,
    )>,
    //time: Time,
) {
    for (mut state, config, mut pid, mass_props) in crafts.iter_mut() {
        {
            let mut angular_input = state.input;

            if config.limit_angular_v {
                angular_input =
                    angular_input.clamp(-config.angular_v_limit, config.angular_v_limit);
            }
            let max_torque = config
                .thruster_torque
                .expect("transient values weren't derived")
                * config.thruster_force_multiplier;

            // TODO: work out if this is actually the inertia tensor
            let local_moi_inv_sqrt = mass_props.local_mprops.inv_principal_inertia_sqrt;
            // NOTICE: difference here
            let mut acceleration_limit: TVec3 = [
                max_torque.x * local_moi_inv_sqrt.x,
                max_torque.y * local_moi_inv_sqrt.y,
                max_torque.z * local_moi_inv_sqrt.z,
            ]
            .into();

            if config.limit_acceleration {
                let artificial_accel_limit = config
                    .angular_acceleration_limit
                    .expect("transient values weren't derived");
                pid.0.integrat_max = acceleration_limit.min(artificial_accel_limit);
                pid.0.integrat_min = -pid.0.integrat_min;

                // clamp the actual limit to the artifical limit
                acceleration_limit =
                    acceleration_limit.clamp(-artificial_accel_limit, artificial_accel_limit);
            }
            let angular_flame = pid
                .0
                .update(state.velocity, angular_input - state.velocity, 1.0);
            let angular_flame = angular_flame.clamp(-acceleration_limit, acceleration_limit);
            state.flame = angular_flame;
        }
    }
}

pub fn apply_flames_simple_accel(
    mut crafts: Query<(
        &GlobalTransform,
        &LinearEngineState,
        &AngularEngineState,
        &EngineConfig,
        &RigidBodyMassProps,
        &mut RigidBodyForces,
    )>,
    //time: Time,
) {
    for (g_xform, lin_state, ang_state, config, mass_props, mut forces) in crafts.iter_mut() {
        let force = lin_state.flame * config.mass;
        let force = g_xform.rotation * force;
        forces.force += Vector::from(force);

        let local_moi_inv_sqrt = mass_props.local_mprops.inv_principal_inertia_sqrt;
        let torque: TVec3 = [
            ang_state.flame.x / local_moi_inv_sqrt.x,
            ang_state.flame.y / local_moi_inv_sqrt.y,
            ang_state.flame.z / local_moi_inv_sqrt.z,
        ]
        .into();
        let torque = g_xform.rotation * torque;

        forces.torque += AngVector::from(torque);
    }
}
