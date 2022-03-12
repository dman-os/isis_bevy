use deps::*;

use bevy::prelude::*;

use crate::math::*;

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

pub fn points_on_sphere(point_count: usize) -> Vec<TVec3> {
    let mut directions = Vec::with_capacity(point_count);
    #[allow(clippy::unnecessary_cast)]
    let golden_ratio = (1.0 + (5.0 as TReal).sqrt()) * 0.5;
    let angle_increment = real::consts::TAU * golden_ratio;
    #[allow(clippy::needless_range_loop)]
    for ii in 0..point_count {
        let t = ii as TReal / point_count as TReal;
        let inclination = (1.0 - (2.0 * t)).acos();
        let azimuth = angle_increment * (ii as TReal);
        directions.push(
            TVec3::new(
                inclination.sin() * azimuth.cos(),
                inclination.sin() * azimuth.sin(),
                inclination.cos(),
            )
            .normalize(),
        );
    }
    directions
}
