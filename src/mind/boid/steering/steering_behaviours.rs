//! All inputs are in world space unless otherwise remarked to be so.
//!
//! -Z is fwd in here

use deps::*;

use bevy::prelude::*;

use crate::math::*;
/*
/// All parameters should be in world space.
/// `target_facing` is assumed to be normalized
#[inline]
pub fn be_ray(
    target_pos: TVec3,
    target_facing: TVec3,
    target_lin_vel: TVec3,
    xform: &GlobalTransform,
    current_lin_vel: TVec3,
    max_lin_accel: TVec3,
    linvel_limit: TVec3,
) -> (
    crate::mind::boid::LinearRoutineOutput,
    crate::mind::boid::AngularRoutineOutput,
) {
    let target_offset = target_pos - xform.translation;
    let dst = target_offset.length_squared();
    let vel_diff = current_lin_vel - target_lin_vel;

    if dst < TReal::EPSILON && vel_diff.length_squared() < TReal::EPSILON {
        return (
            // (target_lin_vel.normalize() * (target_lin_vel / linvel_limit)).into(),
            TVec3::ZERO.into(),
            crate::mind::boid::steering_systems::look_to(
                xform.rotation.inverse() * target_facing,
            )
            .into(),
        );
    }
    let dst = dst.sqrt();

    // at our current speed to pos,
    // do we have enough distance (according to the avail accel)
    // to get to `target_lin_vel`

    // the mimum distance needed to adjust velocity in all axes
    let adjust_vel_dst = {
        let max_a_world = xform.rotation.inverse() * max_lin_accel;
        let mut max_dst: TReal = 0.;
        for ii in 0..2 {
            let accel = max_a_world[ii];
            // if we don't have any acceleration in that direction to make change
            if accel < TReal::EPSILON {
                // ignore it?
                continue;
            }
            let cur_spd = current_lin_vel[ii];
            let target_spd = target_lin_vel[ii];
            max_dst = max_dst.max(dst_to_change(cur_spd, target_spd, accel));
        }
        max_dst
    };
    if dst > adjust_vel_dst {
        let seek = target_offset.normalize();
        (
            seek.into(),
            crate::mind::boid::steering_systems::look_to(xform.rotation.inverse() * seek)
                .into(),
        )
    } else {
        (
            (target_lin_vel - current_lin_vel).normalize().into(),
            crate::mind::boid::steering_systems::look_to(
                xform.rotation.inverse() * target_facing,
            )
            .into(),
        )
    }
}
 */
#[inline]
pub fn be_ray(
    at_pos: TVec3,
    with_linvel: TVec3,
    pos_linvel: TVec3,
    current_pos: TVec3,
    current_lin_vel: TVec3,
    max_accel: TVec3,
    linvel_limit: TVec3,
    cur_rotation: TQuat,
    debug: &mut dyn FnMut(TVec3, TVec3),
) -> TVec3 {
    let vel = current_lin_vel - pos_linvel;
    let target_vel = with_linvel - pos_linvel;
    /* let acceleration_radius = {
        let time_to_change = {
            let accel = max_accel;
            let delta = target_vel - vel ;
            let time: TVec3 = delta / accel;
            time.abs()
        };
        let avg_vel = (vel + target_vel) * 0.5;
        let dst: TVec3 = avg_vel * time_to_change;
        // dst.abs().max_element()
        dst.length()
    }; */
    let accel_displacement = {
        /* let max_vel = if vel.length_squared() > f32::EPSILON {
            vel.normalize() * linvel_limit
        } else {
            cur_rotation * linvel_limit
        }; */
        let time_to_change = {
            let accel = cur_rotation * max_accel;
            let delta = target_vel - vel;
            (delta / accel).abs()
        };
        let avg_vel = (vel + target_vel) * 0.5;
        avg_vel * time_to_change
    };
    /*  let accel_displacement = {
        let accel = cur_rotation * max_accel;
        -((target_vel * target_vel) - (vel * vel)) / (2. * accel)
    }; */

    const BREATHING_SPACE_MULTIPLIER: TReal = 1.4;
    // let acceleration_displacement = accel_displacement * BREATHING_SPACE_MULTIPLIER;

    let adjusted_pos = at_pos + pos_linvel;
    debug(at_pos, adjusted_pos - with_linvel);
    debug(
        adjusted_pos - with_linvel,
        adjusted_pos - with_linvel - accel_displacement * BREATHING_SPACE_MULTIPLIER,
    );

    /*  let with_linvel = with_linvel / linvel_limit;
    let to_pos_vel = (adjusted_pos - with_linvel - accel_displacement - current_pos).normalize();
    let weight = (adjusted_pos - current_pos).length()
        / (accel_displacement.length() * BREATHING_SPACE_MULTIPLIER);
    with_linvel + (to_pos_vel - with_linvel) * weight */
    if (adjusted_pos - with_linvel - current_pos).length()
        > (accel_displacement.length() * BREATHING_SPACE_MULTIPLIER)
        || (adjusted_pos - current_pos).length()
            < (accel_displacement.length() * BREATHING_SPACE_MULTIPLIER)
    {
        (adjusted_pos - with_linvel - accel_displacement - current_pos).normalize()
    } else {
        debug(current_pos, current_pos + with_linvel);
        // with_linvel.lerp(
        //     current_lin_vel,
        //     // current_lin_vel.normalize() * linvel_limit,
        //     dst / acceleration_radius,
        // ) / linvel_limit
        with_linvel / linvel_limit
    }
    // let offset = at_pos - current_pos;
    // let to_pos = offset.normalize();
    // let to_vel = with_linvel / linvel_limit;

    // to_vel.lerp(to_pos, offset.length() / acceleration_radius)

    /*    // let adjusted_pos = (at_pos + pos_linvel) - with_linvel;
    // - (with_linvel.normalize() * (acceleration_radius + with_linvel.length()));
    // let weight = dst / acceleration_radius;

    debug(current_pos, adjusted_pos);
    // debug(adjusted_pos, adjusted_pos + with_linvel);
    debug(at_pos, at_pos + (TVec3::Z * acceleration_radius));
    debug(at_pos, at_pos + (TVec3::Z * -acceleration_radius));
    debug(at_pos, at_pos + (TVec3::Y * acceleration_radius));
    debug(at_pos, at_pos + (TVec3::Y * -acceleration_radius));
    debug(at_pos, at_pos + (TVec3::X * acceleration_radius));
    debug(at_pos, at_pos + (TVec3::X * -acceleration_radius));

    // (with_linvel / linvel_limit).lerp(
    //     (adjusted_pos - current_pos).normalize(),
    //     (at_pos - current_pos).length() / acceleration_radius,
    // )

    /* let with_linvel = with_linvel / linvel_limit;
    let to_pos_vel = (adjusted_pos - current_pos).normalize();
    let weight = (adjusted_pos - current_pos).length() / acceleration_radius;
    with_linvel + (to_pos_vel - with_linvel) * weight */

    /* let with_linvel = with_linvel / linvel_limit;
    let to_pos_vel = (at_pos - current_pos).normalize();
    let weight = (at_pos - current_pos).length() / acceleration_radius;
    with_linvel + (to_pos_vel - with_linvel) * weight */

    if (at_pos - current_pos).length() > acceleration_radius {
        (at_pos - current_pos).normalize()
    } else {
        // with_linvel.lerp(
        //     current_lin_vel,
        //     // current_lin_vel.normalize() * linvel_limit,
        //     dst / acceleration_radius,
        // ) / linvel_limit
        with_linvel / linvel_limit
    } */
}

#[inline(always)]
pub fn seek_position(current_pos: TVec3, target_pos: TVec3) -> TVec3 {
    (target_pos - current_pos).normalize()
}

#[inline]
pub fn arrive_at_position(
    current_pos: TVec3,
    target_pos: TVec3,
    target_speed: TReal,
    max_speed: TReal,
    arrival_tolerance: TReal,
    deceleration_radius: TReal,
) -> TVec3 {
    let target_offset = target_pos - current_pos;
    let dst = target_offset.length_squared();
    // if we've arrived according to the tolerance
    if dst < arrival_tolerance * arrival_tolerance {
        // stop
        return TVec3::ZERO;
    }
    let dst = dst.sqrt();
    // let deceleration_radius = dst_to_change(speed_to_target, 0., max_accel);

    const BREATHING_SPACE_MULTIPLIER: TReal = 1.4;
    let deceleration_radius = deceleration_radius * BREATHING_SPACE_MULTIPLIER;
    // let speed_to_target = current_vel.dot(target_offset) / dst;
    let weight = (dst - arrival_tolerance) / deceleration_radius;
    target_offset.normalize() * ((target_speed + (max_speed - target_speed) * weight) / max_speed)
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
pub fn allignment(current_vel: TVec3, flock_size: usize, flock_vel_sum: TVec3) -> TVec3 {
    if flock_size > 1 {
        // subtract current vel since flock includes current craft
        // and we didn'exclude it when it was orginally summed
        let exculidng_heading_sum = flock_vel_sum - current_vel;
        // subtract from count by one to exclude current craft
        let flock_average_vel = exculidng_heading_sum / (flock_size - 1) as TReal;

        flock_average_vel.normalize()
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
    use once_cell::sync::Lazy;
    static RAY_DIRECTIONS: Lazy<[TVec3; RAY_COUNT]> = Lazy::new(|| {
        let mut directions = [TVec3::ZERO; RAY_COUNT];
        #[allow(clippy::unnecessary_cast)]
        let golden_ratio = (1.0 + (5.0 as TReal).sqrt()) * 0.5;
        let angle_increment = real::consts::TAU * golden_ratio;
        #[allow(clippy::needless_range_loop)]
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

    // also, we have to negate it since fwd is -Z or something, I'm really confused too
    let transformer = Transform::identity().looking_at(-cast_root, xform.local_y());

    // skip the first option which is directly ahead
    for ii in 1..RAY_COUNT {
        let dir = RAY_DIRECTIONS[ii];
        // in world space
        let dir = transformer.rotation.mul_vec3(dir);
        if !is_dir_obstructed(dir) {
            return dir;
        }
    }
    // TVec3::ZERO
    -cast_root
}

#[inline]
pub fn time_to_change(cur_spd: TReal, target_spd: TReal, accel: TReal) -> TReal {
    // a = (vf - vi) / t
    // t = (vf - vi) / a
    debug_assert!(accel > TReal::EPSILON, "acceleration is zero");
    let delta = target_spd - cur_spd;
    let time = delta / accel;
    time.abs()
}

#[inline]
pub fn dst_to_change(cur_spd: TReal, target_spd: TReal, accel: TReal) -> TReal {
    // d = vt
    // v = (vf + vi) / 2 = 0.5 (vf + vi)
    // d = 0.5 (vf + vi) t
    // vf = vi + at
    // d = 0.5 ((vi + at) + vi) t
    // d = vit + 0.5att

    let dist = 0.5 * (cur_spd + target_spd) * time_to_change(cur_spd, target_spd, accel);
    // let dist = cur_sped * time + 0.5 * accel * time * time;
    // let dist = cur_spd * time + 0.5 * delta * (delta / accel);
    dist.abs()
}

#[test]
fn zmblo() {
    let to_target: Vec3 = [10., 10., 0.].into();
    let vel: Vec3 = [5., 5., 0.].into();
    let out = to_target.dot(vel);
    let out = (out / (to_target.length())) * to_target.normalize();
    let out2 = vel.project_onto(to_target);
    println!("{out:?},{out2:?}");

    let vi = 5.;
    let vf = 15.;
    let a = 1.;
    let d = vf - vi;
    let t = d / a;
    // let (dst, time) = stop_dst_time(vi, vf, accel);
    let out = vi * t + 0.5 * d * (d / a);
    let out2 = 0.5 * (vi + vf) * t;
    println!("{out:?}, {out2:?}");
    // assert!(dst -  < TReal::EPSILON);

    let a: Vec3 = [138.13618, 0.0, 144.63193].into();
    let b = [577.4335, 0.0, 604.58813].into();
    let out = a.dot(b) / b.length();
    let out2 = b.length();
    println!("{out:?},{out2:?}");
}
