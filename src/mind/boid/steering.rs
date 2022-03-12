use deps::*;

use bevy::prelude::*;
use bevy_inspector_egui::prelude::*;

use crate::craft::*;
use crate::math::*;
use crate::mind::*;

pub mod arrive;
pub mod avoid_collision;
pub mod compose;
pub mod face;
pub mod fly_with_flock;
pub mod intercept;
pub mod player;
pub mod seek;
pub mod steering_behaviours;

#[derive(Debug, Default, Clone, Component, Reflect, Inspectable)]
pub struct CurrentSteeringRoutine {
    pub routine: Option<Entity>,
}

#[derive(Debug, Component, Default)]
#[component(storage = "SparseSet")]
pub struct ActiveSteeringRoutine;

pub type RoutineKind = std::any::TypeId;

/// This tags an entity as a steering routine
#[derive(Debug, Clone, Copy, Component)]
pub struct SteeringRoutine {
    boid_entt: Entity,
    kind: RoutineKind,
}

impl SteeringRoutine {
    pub fn new(boid_entt: Entity, kind: RoutineKind) -> Self {
        Self { kind, boid_entt }
    }

    #[inline]
    pub fn boid_entt(&self) -> Entity {
        self.boid_entt
    }

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
    pub name: Name,
    pub parent: Parent,
}

impl<P> LinOnlyRoutineBundle<P>
where
    P: Component,
{
    pub const DEFAULT_NAME: &'static str = "linear_steering_routine";
    pub fn new(param: P, boid_entt: Entity) -> Self {
        Self {
            param,
            output: Default::default(),
            tag: SteeringRoutine::new(boid_entt, RoutineKind::of::<P>()),
            name: Self::DEFAULT_NAME.into(),
            parent: Parent(boid_entt),
        }
    }
}

/// A generic bundle for steering routines that only have angular ouptuts.
#[derive(Bundle)]
pub struct AngOnlyRoutineBundle<P>
where
    P: Component,
{
    pub param: P,
    pub output: AngularRoutineOutput,
    pub tag: SteeringRoutine,
    pub name: Name,
    pub parent: Parent,
}

impl<P> AngOnlyRoutineBundle<P>
where
    P: Component,
{
    pub const DEFAULT_NAME: &'static str = "angular_steering_routine";
    pub fn new(param: P, boid_entt: Entity) -> Self {
        Self {
            param,
            output: Default::default(),
            tag: SteeringRoutine::new(boid_entt, RoutineKind::of::<P>()),
            name: Self::DEFAULT_NAME.into(),
            parent: Parent(boid_entt),
        }
    }
}
/// A generic bundle for steering routines that only have linear ouptuts.
#[derive(Bundle)]
pub struct LinOnlyRoutineBundleExtra<P, E>
where
    P: Component,
    E: Component,
{
    pub param: P,
    pub extra: E,
    pub output: LinearRoutineOutput,
    pub tag: SteeringRoutine,
    pub name: Name,
    pub parent: Parent,
}

impl<P, E> LinOnlyRoutineBundleExtra<P, E>
where
    P: Component,
    E: Component,
{
    pub const DEFAULT_NAME: &'static str = "linear_steering_routine";
    pub fn new(param: P, boid_entt: Entity, extra: E) -> Self {
        Self {
            param,
            extra,
            output: Default::default(),
            tag: SteeringRoutine::new(boid_entt, RoutineKind::of::<P>()),
            name: Self::DEFAULT_NAME.into(),
            parent: Parent(boid_entt),
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
    pub name: Name,
    pub parent: Parent,
}

impl<P> LinAngRoutineBundle<P>
where
    P: Component,
{
    pub const DEFAULT_NAME: &'static str = "linear_angular_steering_routine";
    pub fn new(param: P, boid_entt: Entity) -> Self {
        Self {
            param,
            lin_res: LinearRoutineOutput::default(),
            ang_res: AngularRoutineOutput::default(),
            tag: SteeringRoutine::new(boid_entt, RoutineKind::of::<P>()),
            name: Self::DEFAULT_NAME.into(),
            parent: Parent(boid_entt),
        }
    }
}

/// A generic bundle for steering routines that only have linear and angular ouptuts.
#[derive(Bundle)]
pub struct LinAngRoutineBundleExtra<P, E>
where
    P: Component,
    E: Component,
{
    pub param: P,
    pub extra: E,
    pub lin_res: LinearRoutineOutput,
    pub ang_res: AngularRoutineOutput,
    pub tag: SteeringRoutine,
    pub name: Name,
    pub parent: Parent,
}

impl<P, E> LinAngRoutineBundleExtra<P, E>
where
    P: Component,
    E: Component,
{
    pub const DEFAULT_NAME: &'static str = "linear_angular_steering_routine";
    pub fn new(param: P, boid_entt: Entity, extra: E) -> Self {
        Self {
            param,
            extra,
            lin_res: LinearRoutineOutput::default(),
            ang_res: AngularRoutineOutput::default(),
            tag: SteeringRoutine::new(boid_entt, RoutineKind::of::<P>()),
            name: Self::DEFAULT_NAME.into(),
            parent: Parent(boid_entt),
        }
    }
}
/*
   #[derive(Debug, Clone, Copy, Inspectable, Component)]
   pub enum SteeringRoutineOutpt {
       Linear(TVec3),
       Angular(TVec3),
       LinAng(TVec3, TVec3),
   }
*/

pub fn steering_output_to_engine(
    mut crafts: Query<(
        &GlobalTransform,
        &CurrentSteeringRoutine,
        &boid::BoidMindConfig,
        &mut engine::LinearEngineState,
        &mut engine::AngularEngineState,
        &engine::EngineConfig,
    )>,
    routines: Query<(&LinearRoutineOutput, &AngularRoutineOutput), With<SteeringRoutine>>,
    // routines: Query<&LinearRoutineResult, With<steering_systems::Intercept>>,
    //egui_context: ResMut<bevy_egui::EguiContext>,
) {
    for (xform, cur_routine, config, mut lin_state, mut ang_state, engine_config) in
        crafts.iter_mut()
    {
        let (lin_out, ang_out) = if let Some(cur_routine) = cur_routine.routine.as_ref() {
            let (lin_out, ang_out) = routines.get(*cur_routine)
            .expect_or_log("CurrentSteeringRoutine's routine not located in world. \
            Use a routine with both Linear and Angular outputs or wrap whatever you're using now with a Compose routine");
            (lin_out.0, ang_out.0)
        } else {
            (TVec3::ZERO, TVec3::ZERO)
        };
        lin_state.input = (xform.rotation.inverse() * lin_out) * engine_config.linvel_limit.abs();
        ang_state.input = ang_out * config.angular_input_multiplier;
    }
}

/* fn routine_garbage_collector(
    mut commands: Commands,
    mut crafts: Query<
        (Entity, &RoutineComposer, &mut CraftRoutinesIndex),
        Changed<RoutineComposer>,
    >,
    routines: Query<&SteeringRoutine>,
    mut cache: Local<bevy::utils::HashSet<Entity>>,
) {
    for (boid_entt, composer, mut index) in crafts.iter_mut() {
        // make a set of all the composed routines
        cache.extend(composer.all_routines());
        // for all index
        for routine in index.entt_to_kind.keys() {
            // if being composed
            if cache.contains(routine) {
                // remove from set
                cache.remove(routine);
            } else {
                // destroy routine
                commands.entity(*routine).despawn_recursive();
            }
        }
        // for remaining composed routines not in indices
        for entt in cache.drain() {
            let routine = routines
                .get(entt)
                .expect_or_log("composed steering routine not found");
            index.add_routine(entt, routine.kind);
        }
    }
}
 */

/// Output of linear steering routines which is usually linear velocity desired next frame in
/// fraction of [`EngineConfig:.linvel_limit`] in world space.
#[derive(Debug, Clone, Copy, Default, Inspectable, Component)]
pub struct LinearRoutineOutput(pub TVec3);

impl From<TVec3> for LinearRoutineOutput {
    fn from(v: TVec3) -> Self {
        Self(v)
    }
}

/// Output of angular steering routines which is usually angular velocity desired next frame in local space.
#[derive(Debug, Clone, Copy, Default, Inspectable, Component)]
pub struct AngularRoutineOutput(pub TVec3);

impl From<TVec3> for AngularRoutineOutput {
    fn from(v: TVec3) -> Self {
        Self(v)
    }
}

/*
#[inline]
pub fn just_be(
    target_pos: TVec3,
    target_facing: TVec3,
    target_lin_vel: TVec3,
    target_ang_vel: TVec3,
    xform: &GlobalTransform,
    current_lin_vel: TVec3,
    current_ang_vel: TVec3,
    max_lin_accel: TVec3,
    max_ang_accel: TVec3,
    linvel_limit: TVec3,
    angvel_limit: TVec3,
) -> (LinearRoutineOutput, AngularRoutineOutput) {
    todo!()
} */

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
