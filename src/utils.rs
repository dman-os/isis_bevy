use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};

#[derive(Debug, Component)]
pub struct PIDControllerVec3 {
    last_state: Vec3,
    integrat_err: Vec3,
    pub proportional_gain: Vec3,
    pub integrat_gain: Vec3,
    pub integrat_max: Vec3,
    pub integrat_min: Vec3,
    pub differntial_gain: Vec3,
}

impl PIDControllerVec3 {
    pub fn new(
        proportional_gain: Vec3,
        integrat_gain: Vec3,
        integrat_max: Vec3,
        integrat_min: Vec3,
        differntial_gain: Vec3,
    ) -> Self {
        Self {
            last_state: Default::default(),
            integrat_err: Default::default(),
            proportional_gain,
            integrat_gain,
            integrat_max,
            integrat_min,
            differntial_gain,
        }
    }

    pub fn update(&mut self, state: Vec3, err: Vec3, delta_time: f32) -> Vec3 {
        // cacluate the inegral error
        // clamp the integrator state to mitigate windup
        self.integrat_err =
            (self.integrat_err + (err * delta_time)).clamp(self.integrat_min, self.integrat_max);

        let drive_v =
            // calculate the proportional term
            self.proportional_gain * err
            // caclulate the integral term
            + self.integrat_gain * self.integrat_err
            // caclulate the differntal term
            + self.differntial_gain * ((state - self.last_state) * delta_time);

        self.last_state = state;

        drive_v
    }
}
