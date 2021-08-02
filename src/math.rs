use deps::*;

use bevy::prelude::*;

pub mod real {
    pub use std::f32::*;
}

pub type Real = f32;
pub type Vector3 = Vec3;
pub type IVector3 = IVec3;
use real::consts::{PI, TAU};

#[inline]
pub fn delta_angle_radians(a: Real, b: Real) -> Real {
    let spea1 = smallest_positve_equivalent_angle_rad(a);
    let spea2 = smallest_positve_equivalent_angle_rad(b);
    let result = (spea1 - spea2).abs();
    if result > PI {
        TAU - result
    } else {
        result
    }
}

#[inline]
pub fn smallest_equivalent_angle_radians(mut angle: Real) -> Real {
    angle %= TAU;
    if angle > PI {
        angle -= TAU
    } else if angle < -PI {
        angle += TAU;
    }
    angle
}

#[inline]
pub fn smallest_positve_equivalent_angle_rad(mut angle: Real) -> Real {
    angle %= TAU;
    if angle < 0. {
        angle + TAU
    } else {
        angle
    }
}

#[test]
fn smallest_positve_equivalent_angle_rad_test() {
    let d90 = PI * 0.5;
    assert!(smallest_positve_equivalent_angle_rad(0.) - 0. < Real::EPSILON);
    assert!(smallest_positve_equivalent_angle_rad(TAU) - 0. < Real::EPSILON);
    assert!(smallest_positve_equivalent_angle_rad(PI) - PI < Real::EPSILON);
    assert!(smallest_positve_equivalent_angle_rad(PI) - PI < Real::EPSILON);
    assert!(smallest_positve_equivalent_angle_rad(TAU - d90) - (PI + d90) < Real::EPSILON);
    assert!(smallest_positve_equivalent_angle_rad(TAU + d90) - d90 <= Real::EPSILON);
    assert!(smallest_positve_equivalent_angle_rad(-0.2) - (TAU - 0.2) <= Real::EPSILON);
}
#[test]
fn delta_angle_radians_test() {
    let d90 = PI * 0.5;
    let d45 = d90 * 0.5;
    let d30 = PI / 3.;
    assert!(delta_angle_radians(PI, TAU) - PI < Real::EPSILON);
    assert!(delta_angle_radians(-d90, 0.) - d90 <= Real::EPSILON);
    assert!(delta_angle_radians(-TAU - d90, d90) - PI <= Real::EPSILON);
    assert!(delta_angle_radians(0., 2. * TAU) < Real::EPSILON);
    assert!(delta_angle_radians(PI, d90) - d90 < Real::EPSILON);
    assert!(delta_angle_radians(TAU - d45, d45) - d90 < Real::EPSILON);
    assert!(delta_angle_radians(TAU - d45, 0.) - d45 < Real::EPSILON);
    assert!(delta_angle_radians(TAU + PI, 0.) - PI < Real::EPSILON);
    assert!(delta_angle_radians(TAU + d45, 0.) - d45 <= Real::EPSILON);
    assert!(delta_angle_radians(-d45, 0.) - d45 <= Real::EPSILON);
    assert!(delta_angle_radians(-d30, 0.) - d30 <= 2. * Real::EPSILON);
    assert!(delta_angle_radians(-0.2, 0.) - 0.2 <= Real::EPSILON);
}
