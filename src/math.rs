use deps::*;

use bevy::prelude::*;

pub mod real {
    pub use std::f32::*;
}

pub type TReal = f32;
pub type TVec3 = Vec3;
pub type TQuat = Quat;

use real::consts::{PI, TAU};

#[inline]
pub fn delta_angle_radians(a: TReal, b: TReal) -> TReal {
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
pub fn smallest_equivalent_angle_radians(mut angle: TReal) -> TReal {
    angle %= TAU;
    if angle > PI {
        angle -= TAU
    } else if angle < -PI {
        angle += TAU;
    }
    angle
}

#[inline]
pub fn smallest_positve_equivalent_angle_rad(mut angle: TReal) -> TReal {
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
    assert!(smallest_positve_equivalent_angle_rad(0.) - 0. < TReal::EPSILON);
    assert!(smallest_positve_equivalent_angle_rad(TAU) - 0. < TReal::EPSILON);
    assert!(smallest_positve_equivalent_angle_rad(PI) - PI < TReal::EPSILON);
    assert!(smallest_positve_equivalent_angle_rad(PI) - PI < TReal::EPSILON);
    assert!(smallest_positve_equivalent_angle_rad(TAU - d90) - (PI + d90) < TReal::EPSILON);
    assert!(smallest_positve_equivalent_angle_rad(TAU + d90) - d90 <= TReal::EPSILON);
    assert!(smallest_positve_equivalent_angle_rad(-0.2) - (TAU - 0.2) <= TReal::EPSILON);
}
#[test]
fn delta_angle_radians_test() {
    let d90 = PI * 0.5;
    let d45 = d90 * 0.5;
    let d30 = PI / 3.;
    assert!(delta_angle_radians(PI, TAU) - PI < TReal::EPSILON);
    assert!(delta_angle_radians(-d90, 0.) - d90 <= TReal::EPSILON);
    assert!(delta_angle_radians(-TAU - d90, d90) - PI <= TReal::EPSILON);
    assert!(delta_angle_radians(0., 2. * TAU) < TReal::EPSILON);
    assert!(delta_angle_radians(PI, d90) - d90 < TReal::EPSILON);
    assert!(delta_angle_radians(TAU - d45, d45) - d90 < TReal::EPSILON);
    assert!(delta_angle_radians(TAU - d45, 0.) - d45 < TReal::EPSILON);
    assert!(delta_angle_radians(TAU + PI, 0.) - PI < TReal::EPSILON);
    assert!(delta_angle_radians(TAU + d45, 0.) - d45 <= TReal::EPSILON);
    assert!(delta_angle_radians(-d45, 0.) - d45 <= TReal::EPSILON);
    assert!(delta_angle_radians(-d30, 0.) - d30 <= 2. * TReal::EPSILON);
    assert!(delta_angle_radians(-0.2, 0.) - 0.2 <= TReal::EPSILON);
}

pub trait Vec3Ext {
    fn move_towards(self, other: Self, max: TReal) -> Self;
}
impl Vec3Ext for TVec3 {
    #[inline]
    fn move_towards(self, other: Self, max: TReal) -> Self {
        self + (other - self).clamp_length(-max, max)
    }
}

pub trait TransformExt {
    /// Calculate the parent transform from the entity's [`Transform`] and [`GlobalTransform`].
    /// Forget not to check if the entity indeed has a parent first.
    ///
    /// ```
    /// let parent_xform = Transform::from_translation([1., 2., 3.].into())
    ///     .with_rotation(Quat::from_rotation_y(3.145))
    ///     .with_scale([2., 2., 2.].into());
    /// let xform = Transform::from_translation([1., 2., 3.].into())
    ///     .with_rotation(Quat::from_rotation_y(3.145))
    ///     .with_scale([2., 2., 2.].into())
    ///     .into();
    ///
    /// let glob_xform = parent_xform.mul_transform(xform);
    ///
    /// let calc_parent_xform = calc_parent_xform(&xform, &glob_xform.into());
    ///
    /// assert!(
    ///     (parent_xform.translation - calc_parent_xform.translation).length_squared() < f32::EPSILON,
    /// );
    /// assert!((parent_xform.rotation - calc_parent_xform.rotation).length_squared() < f32::EPSILON);
    /// assert!((parent_xform.scale - calc_parent_xform.scale).length_squared() < f32::EPSILON);
    /// ```
    fn calc_parent_xform(&self, glob_xform: &GlobalTransform) -> GlobalTransform;
    fn is_nan(&self) -> bool;
    fn inverse(self) -> Transform;
}

impl TransformExt for Transform {
    #[inline]
    fn calc_parent_xform(&self, glob_xform: &GlobalTransform) -> GlobalTransform {
        // glob_xform.inverse().mul_transform(*self).into()
        let scale = glob_xform.scale / self.scale;
        let rotation = self.rotation.inverse() * glob_xform.rotation;
        GlobalTransform {
            translation: rotation.inverse() * ((glob_xform.translation - self.translation) / scale),
            // translation: glob_xform.translation - ((rotation.inverse() * self.translation) / scale),
            rotation,
            scale,
        }
    }

    #[inline]
    fn is_nan(&self) -> bool {
        self.translation.is_nan() || self.rotation.is_nan() || self.scale.is_nan()
    }

    #[inline]
    fn inverse(self) -> Transform {
        Self::from_matrix(self.compute_matrix().inverse())
    }
}

impl TransformExt for GlobalTransform {
    #[inline]
    fn calc_parent_xform(&self, glob_xform: &GlobalTransform) -> GlobalTransform {
        let scale = glob_xform.scale / self.scale;
        let rotation = (self.rotation.inverse() * glob_xform.rotation).normalize();
        GlobalTransform {
            translation: rotation.inverse() * ((glob_xform.translation - self.translation) / scale),
            rotation,
            scale,
        }
    }

    #[inline]
    fn is_nan(&self) -> bool {
        self.translation.is_nan() || self.rotation.is_nan() || self.scale.is_nan()
    }
    #[inline]
    fn inverse(self) -> Transform {
        Transform::from_matrix(self.compute_matrix().inverse())
    }
}

pub trait QuatExt {
    fn looking_to(to: Vec3, up: Vec3) -> Self;
}

impl QuatExt for TQuat {
    fn looking_to(to: Vec3, up: Vec3) -> Self {
        let forward = Vec3::normalize(to);
        let right = up.cross(forward).normalize();
        let up = forward.cross(right);
        TQuat::from_mat3(&Mat3::from_cols(right, up, forward))
    }
}

#[test]
fn parent_xform_calc() {
    let parent_xform = Transform::from_translation([1., 2., 3.].into())
        .with_rotation(Quat::from_rotation_y(3.145))
        .with_scale([2., 2., 2.].into());

    let xform = Transform::from_translation([1., 2., 3.].into())
        .with_rotation(Quat::from_rotation_y(3.145))
        .with_scale([2., 2., 2.].into());

    let glob_xform = parent_xform.mul_transform(xform);

    let calc_parent_xform = xform.calc_parent_xform(&glob_xform.into());

    assert!(
        (parent_xform.translation - calc_parent_xform.translation).length_squared() < f32::EPSILON,
        "\n{parent_xform:?}\n{calc_parent_xform:?}"
    );
    assert!((parent_xform.rotation - calc_parent_xform.rotation).length_squared() < f32::EPSILON);
    assert!((parent_xform.scale - calc_parent_xform.scale).length_squared() < f32::EPSILON);
}

#[test]
fn inverse_xform() {
    let xform = Transform::from_translation([1., 2., 3.].into())
        .with_rotation(Quat::from_rotation_y(3.145))
        .with_scale([2., 2., 2.].into());
    let inv_xform = xform.inverse();
    let inv_mat_xform = Transform::from_matrix(xform.compute_matrix().inverse());

    assert!(
        (inv_xform.translation - inv_mat_xform.translation).length_squared() < f32::EPSILON,
        "\n{inv_xform:?}\n{inv_mat_xform:?}"
    );
    assert!((inv_xform.rotation - inv_mat_xform.rotation).length_squared() < f32::EPSILON);
    assert!((inv_xform.scale - inv_mat_xform.scale).length_squared() < f32::EPSILON);

    let inv_inv_xform = inv_xform.inverse();

    assert!((xform.translation - inv_inv_xform.translation).length_squared() < f32::EPSILON);
    assert!((xform.rotation - inv_inv_xform.rotation).length_squared() < f32::EPSILON);
    assert!((xform.scale - inv_inv_xform.scale).length_squared() < f32::EPSILON);
}
