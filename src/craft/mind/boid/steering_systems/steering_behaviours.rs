use deps::*;

use bevy::prelude::*;

use crate::math::*;

#[inline]
pub fn seek_position(current_pos: TVec3, target_pos: TVec3) -> TVec3 {
    (target_pos - current_pos).normalize()
}

#[inline]
pub fn find_intercept_pos(
    current_pos: TVec3,
    travel_speed: TReal,
    target_pos: TVec3,
    target_vel: TVec3,
) -> TVec3 {
    let relative_pos = target_pos - current_pos;
    let distance_to_target = relative_pos.length();
    let time_to_target_pos = distance_to_target / travel_speed;
    target_pos + (time_to_target_pos * target_vel)
}

#[inline]
pub fn intercept_target(
    current_pos: TVec3,
    travel_speed: TReal,
    target_pos: TVec3,
    target_vel: TVec3,
) -> TVec3 {
    seek_position(
        current_pos,
        find_intercept_pos(current_pos, travel_speed, target_pos, target_vel),
    )
}

/// Assumes the current craft's in the flock.
#[inline]
pub fn cohesion(current_pos: TVec3, flock_size: usize, flock_center_sum: TVec3) -> TVec3 {
    if flock_size > 1 {
        // subtract current position since flock includes current craft
        // and we didn'exclude it when it was orginally summed
        let exculidng_center_sum = flock_center_sum - current_pos;
        // subtract from count by one to exclude current craft
        let flock_average_center = exculidng_center_sum / (flock_size - 1) as TReal;

        seek_position(current_pos, flock_average_center)
    } else {
        TVec3::ZERO
    }
}

/// Assumes the current craft's in the flock.
#[inline]
pub fn allignment(current_vel: TVec3, flock_size: usize, flock_heading_sum: TVec3) -> TVec3 {
    if flock_size > 1 {
        // subtract current vel since flock includes current craft
        // and we didn'exclude it when it was orginally summed
        let exculidng_heading_sum = flock_heading_sum - current_vel;
        // subtract from count by one to exclude current craft
        let flock_average_heading = exculidng_heading_sum / (flock_size - 1) as TReal;

        flock_average_heading.normalize()
    } else {
        TVec3::ZERO
    }
}

/// Based on Craig Reynold's OpenSteer
#[inline]
pub fn separation(current_pos: TVec3, flock_positions: &[TVec3]) -> TVec3 {
    let mut steering = TVec3::ZERO;
    if flock_positions.len() > 1 {
        for craft_pos in flock_positions {
            // add in steering contribution
            // (opposite of the offset direction, divided once by distance
            // to normalize, divided another time to get 1/d falloff)
            let relative_pos = *craft_pos - current_pos;
            let dist_squared = relative_pos.length_squared();
            // filter out the current craft
            if dist_squared > TReal::EPSILON {
                steering -= relative_pos / dist_squared;
            }
        }
        // steering /= flock_positions.len() as TReal;
    }
    steering
}
use once_cell::sync::Lazy;

/// Based on Craig Reynold's OpenSteer
#[inline]
pub fn avoid_obstacle_seblague(
    cast_root: TVec3,
    // A function that casts _something_ from the craft's position into the given
    // direction and checks for obstruction.
    is_dir_obstructed: &mut dyn FnMut(TVec3) -> bool,
    xform: &GlobalTransform,
) -> TVec3 {
    const RAY_COUNT: usize = 30;
    static RAY_DIRECTIONS: Lazy<[TVec3; RAY_COUNT]> = Lazy::new(|| {
        let mut directions = [TVec3::ZERO; RAY_COUNT];
        let golden_ratio = (1.0 + (5.0 as TReal).sqrt()) * 0.5;
        let angle_increment = real::consts::TAU * golden_ratio;
        for ii in 0..RAY_COUNT {
            let t = ii as TReal / RAY_COUNT as TReal;
            let inclination = (1.0 - (2.0 * t)).acos();
            let azimuth = angle_increment * (ii as TReal);
            directions[ii] = TVec3::new(
                inclination.sin() * azimuth.cos(),
                inclination.sin() * azimuth.sin(),
                inclination.cos(),
            )
            .normalize();
        }
        directions
    });

    // since we'll be testing from the cast_root vector outwards (not the forward vector)
    // we can't use the object's transform
    let transformer = Transform::identity().looking_at(cast_root, xform.local_y());

    for ii in 0..RAY_COUNT {
        let dir = RAY_DIRECTIONS[ii];
        // in world space
        let dir = transformer.rotation.mul_vec3(dir);
        if !is_dir_obstructed(dir) {
            return dir;
        }
    }
    // TVec3::ZERO
    cast_root
}
