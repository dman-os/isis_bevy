use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};

use super::{AngularRoutineOutput, LinearRoutineOutput};
use crate::math::*;

#[derive(Debug, Component)]
pub struct ActiveRoutine;

pub type RoutineKind = std::any::TypeId;

/// This tags an entity as a steering routine
#[derive(Debug, Clone, Copy, Component)]
pub struct SteeringRoutine {
    craft_entt: Entity,
    kind: RoutineKind,
}

impl SteeringRoutine {
    pub fn new(craft_entt: Entity, kind: RoutineKind) -> Self {
        Self { kind, craft_entt }
    }

    /// Get a reference to the steering routine's craft entt.
    #[inline]
    pub fn craft_entt(&self) -> Entity {
        self.craft_entt
    }

    /// Get a reference to the steering routine's kind.
    #[inline]
    pub fn kind(&self) -> RoutineKind {
        self.kind
    }
}

/// A generic bundle for steering routines that only have linear ouptuts.
#[derive(Bundle)]
pub struct LinOnlyRoutineBundle<P>
where
    P: Component,
{
    pub param: P,
    pub output: LinearRoutineOutput,
    pub tag: SteeringRoutine,
}

impl<P> LinOnlyRoutineBundle<P>
where
    P: Component,
{
    pub fn new(param: P, craft_entt: Entity) -> Self {
        Self {
            param,
            output: Default::default(),
            tag: SteeringRoutine::new(craft_entt, RoutineKind::of::<P>()),
        }
    }
}

/// A generic bundle for steering routines that only have linear and angular ouptuts.
#[derive(Bundle)]
pub struct LinAngRoutineBundle<P>
where
    P: Component,
{
    pub param: P,
    pub lin_res: LinearRoutineOutput,
    pub ang_res: AngularRoutineOutput,
    pub tag: SteeringRoutine,
}

impl<P> LinAngRoutineBundle<P>
where
    P: Component,
{
    pub fn new(param: P, craft_entt: Entity) -> Self {
        Self {
            param,
            lin_res: LinearRoutineOutput::default(),
            ang_res: AngularRoutineOutput::default(),
            tag: SteeringRoutine::new(craft_entt, RoutineKind::of::<P>()),
        }
    }
}

mod avoid_collision;
pub use avoid_collision::*;
mod intercept;
pub use intercept::*;
mod fly_with_flock;
pub use fly_with_flock::*;
mod seek_target;
pub use seek_target::*;

pub mod steering_behaviours;

#[inline]
pub fn look_to(local_dir: TVec3) -> TVec3 {
    let fwd = -TVec3::Z;
    let dir = local_dir;
    // scaling by the angle proves troublesome
    // it takes too long to settle, the final inputs being progressively too minute
    // as we close on the target direction
    /* fwd.angle_between(dir) * */
    fwd.cross(dir)
    /*
        // invert since fwd is -Z
        let dir = -local_dir;
        let (z, x, y) = {
            //// basis facing dir
            //let t = {
            //let forward = dir.normalize();
            //let right = Vector3::Y.cross(forward).normalize();
            //let up = forward.cross(right);
            //Mat3::from_cols(right, up, forward)
            //};
            ////t.euler_angles()
            nalgebra::UnitQuaternion::face_towards(&dir.into(), &Vector3::Y.into())
                .euler_angles()
        };
        let (x, y, z) = (z, x, y);
        Vector3::new(
            crate::math::delta_angle_radians(0., x).copysign(x),
            crate::math::delta_angle_radians(0., y).copysign(y),
            crate::math::delta_angle_radians(0., z).copysign(z),
        )
    */
}
