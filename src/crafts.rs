use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_rapier3d::prelude::*;

pub struct CraftsPlugin;
impl Plugin for CraftsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(sync_craft_state_velocities.system())
            .add_system(linear_pid_driver.system())
            .add_system(angular_pid_driver.system())
            .add_system(apply_flames_simple_accel.system());
    }
}

pub struct CraftCamera;

pub struct CurrentCraft(pub Entity);

#[derive(Bundle)]
pub struct CraftBundle {
    pub config: CraftConfig,
    pub linear_state: LinearCraftState,
    pub angular_state: AngularCraftState,
    pub linear_pid: LinearDriverPid,
    pub angular_pid: AngularDriverPid,
}

impl Default for CraftBundle {
    fn default() -> Self {
        Self {
            config: Default::default(),
            linear_state: Default::default(),
            angular_state: Default::default(),
            linear_pid: LinearDriverPid(crate::utils::PIDControllerVec3::new(
                Vec3::ONE,
                Vec3::ZERO,
                Vec3::ZERO,
                Vec3::ZERO,
                Vec3::ZERO,
            )),
            angular_pid: AngularDriverPid(crate::utils::PIDControllerVec3::new(
                Vec3::ONE * 22.0,
                Vec3::ZERO,
                Vec3::ZERO,
                Vec3::ZERO,
                Vec3::ZERO,
            )),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct LinearCraftState {
    /// Linear velocity in local-space
    /// In m/s.
    pub velocity: Vec3,

    /// Input vector for driver. Meaning depends on driver implementation.
    /// e.g. target velocity to attain
    pub input: Vec3,

    /// Vector output of driver and input vector of a motor. Meaning depends on implementation.
    /// e.g. forve to apply
    pub flame: Vec3,
}

#[derive(Debug, Default, Clone)]
pub struct AngularCraftState {
    /// Angular velocity in local-space
    /// In rad/s.
    pub velocity: Vec3,
    /// Input vector for driver. Meaning depends on driver implementation.
    /// e.g. target velocity to attain
    pub input: Vec3,
    /// Vector output of driver and input vector of a motor. Meaning depends on implementation.
    pub flame: Vec3,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(crate = "serde")]
pub struct CraftConfig {
    ///  Speed to travel at when there is no input i.e. how fast to travel when idle.
    pub set_speed: Vec3,

    /// Total mass of the craft.
    /// In KG.
    pub mass: f32,

    /// Maximum acceleration allowed to the craft.
    /// In m/s.
    pub acceleration_limit: Vec3,

    pub acceleration_limit_multiplier: f32,

    /// Linear velocity cap no matter the input.
    /// In m/s.
    pub linear_v_limit: Vec3,

    /// Angular velocity cap no matter the input.
    /// In rad/s.
    pub angular_v_limit: Vec3,

    /// Max force the linear thrusters are capable of exerting.
    /// In Newtons.
    pub linear_thruster_force: Vec3,

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
    pub angular_thruster_force: Vec3,

    pub thruster_force_multiplier: f32,

    /// The dimensions of the craft.
    pub extents: Vec3,

    /// DERIVED ITEMS

    // TODO: make use of me
    /// Angular thruster toruqe, transient auto cacluated value from the
    /// angular_thrustuer_force according to the craft's shape and mass.
    /// In  Newton meters.
    pub thruster_torque: Option<Vec3>,
    /// Angular acceleration limit, another transient auto cacluated value. It's cacluated from
    /// the normal acceleration limit (which is in m/ss) and adjusted to the size/shape of the craft.
    /// In rad/s/s.
    ///
    /// Curretly unused. Defaults to INFINITY meaning there's no artifical acceleration_limit on
    /// the crafts. They use all of what's availaible from the thrusters.
    pub angular_acceleration_limit: Option<Vec3>,
    ///// Moment of inertia, transient auto cacluated value used to convert the required angular
    ///// acceleration into the appropriate torque. Aquried directly from Godot's physics engine.
    ///// In  kg*m*m.
    ///// Defaults to one to avoid hard to track division by zero errors. The moi is asychronously
    ///// retrieved from the engine and some frames pass before it happens. Time enough for the NANs
    ///// to propagate EVERYWHERE!
    //pub moment_of_inertia: Vec3,
}

impl Default for CraftConfig {
    fn default() -> Self {
        Self {
            mass: 15_000.,
            set_speed: Vec3::ZERO,
            acceleration_limit: [6., 6., 6.].into(),
            acceleration_limit_multiplier: 9.81,
            linear_v_limit: [100., 100., 200.].into(),
            angular_v_limit: [3., 3., 3.].into(),
            limit_forward_v: true,
            limit_strafe_v: true,
            limit_angular_v: true,
            limit_acceleration: true,
            linear_thruster_force: [1., 1., 1.5].into(),
            angular_thruster_force: [1., 1., 1.].into(),
            thruster_force_multiplier: 1_000_000.0,
            extents: Vec3::ONE * 8.0,
            thruster_torque: None,
            angular_acceleration_limit: None,
        }
        .derive_items()
    }
}

impl CraftConfig {
    /// Use this everytime the config changes to calculate transiet items,
    pub fn derive_items(mut self) -> Self {
        self.angular_acceleration_limit = Some([f32::INFINITY; 3].into());

        use bevy::math::vec2;
        let axes_diameter: Vec3 = [
            vec2(self.extents.y, self.extents.z).length(),
            vec2(self.extents.x, self.extents.z).length(),
            vec2(self.extents.x, self.extents.y).length(),
        ]
        .into();

        self.thruster_torque = Some(axes_diameter * self.angular_thruster_force);
        self
    }
}

pub struct LinearDriverPid(crate::utils::PIDControllerVec3);
pub struct AngularDriverPid(crate::utils::PIDControllerVec3);

pub fn sync_craft_state_velocities(
    mut crafts: Query<(
        &mut AngularCraftState,
        &mut LinearCraftState,
        &RigidBodyVelocity,
    )>,
) {
    for (mut angular_state, mut linear_state, rb_velocity) in crafts.iter_mut() {
        angular_state.velocity = rb_velocity.angvel.into();
        linear_state.velocity = rb_velocity.linvel.into();
    }
}

pub fn linear_pid_driver(
    mut crafts: Query<(&mut LinearCraftState, &CraftConfig, &mut LinearDriverPid)>,
    //time: Time,
) {
    for (mut state, config, mut pid) in crafts.iter_mut() {
        let mut linear_input = state.input;

        let v_limit = config.linear_v_limit;

        // if dampeners are on
        if config.limit_strafe_v {
            // clamp the input to the limit
            linear_input = linear_input.clamp(-v_limit, v_limit);
        }

        // if forward dampenere is off
        if !config.limit_forward_v {
            // restore the clamped input on the z
            linear_input.z = state.input.z;
        }

        let mut max_force = config.linear_thruster_force * config.thruster_force_multiplier;

        let is_moving_fwd = linear_input.z > 0.0;
        // if moving backwards
        if !is_moving_fwd {
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
        &mut AngularCraftState,
        &CraftConfig,
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
            let mut acceleration_limit: Vec3 = [
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
            let angular_flame =
                pid.0
                    .update(state.velocity, angular_input - state.velocity, 1.0);
            let angular_flame = angular_flame.clamp(-acceleration_limit, acceleration_limit);
            state.flame = angular_flame;
        }
    }
}

pub fn apply_flames_simple_accel(
    mut crafts: Query<(
        &LinearCraftState,
        &AngularCraftState,
        &CraftConfig,
        &RigidBodyMassProps,
        &mut RigidBodyForces,
    )>,
    //time: Time,
) {
    for (lin_state, ang_state, config, mass_props, mut forces) in crafts.iter_mut() {
        //let force = lin_state.flame * config.mass;
        forces.force += Vector::from(lin_state.flame);

        let local_moi_inv_sqrt = mass_props.local_mprops.inv_principal_inertia_sqrt;
        let torque: Vec3 = [
            ang_state.flame.x / local_moi_inv_sqrt.x,
            ang_state.flame.y / local_moi_inv_sqrt.y,
            ang_state.flame.z / local_moi_inv_sqrt.z,
        ]
        .into();

        forces.torque += AngVector::from(torque);
    }
}
