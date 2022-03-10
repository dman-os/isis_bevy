use deps::*;

use bevy::prelude::*;
use bevy_inspector_egui::prelude::*;

use crate::craft::*;
use crate::math::*;
use crate::mind::*;

pub mod arrive;
pub mod avoid_collision;
pub mod fly_with_flock;
pub mod intercept;
pub mod player;
pub mod seek;
pub mod steering_behaviours;

#[derive(Debug, Component)]
pub struct ActiveSteeringRoutine;

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
    pub name: Name,
    pub parent: Parent,
}

impl<P> LinOnlyRoutineBundle<P>
where
    P: Component,
{
    pub const DEFAULT_NAME: &'static str = "linear_steering_routine";
    pub fn new(param: P, craft_entt: Entity) -> Self {
        Self {
            param,
            output: Default::default(),
            tag: SteeringRoutine::new(craft_entt, RoutineKind::of::<P>()),
            name: Self::DEFAULT_NAME.into(),
            parent: Parent(craft_entt),
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
    pub fn new(param: P, craft_entt: Entity) -> Self {
        Self {
            param,
            lin_res: LinearRoutineOutput::default(),
            ang_res: AngularRoutineOutput::default(),
            tag: SteeringRoutine::new(craft_entt, RoutineKind::of::<P>()),
            name: Self::DEFAULT_NAME.into(),
            parent: Parent(craft_entt),
        }
    }
}

/// This tags the routines in the craft's current [`SteeringRoutineComposer`] with
/// [`ActiveRoutine`] and detags them when no longer in use.
pub fn active_routine_tagger(
    mut commands: Commands,
    crafts: Query<
        (&SteeringRoutineComposer, &sensors::SteeringRoutinesIndex),
        Changed<SteeringRoutineComposer>,
    >,
    mut cache: Local<bevy::utils::HashSet<Entity>>,
) {
    for (composer, index) in crafts.iter() {
        // make a set of all the composed routines
        cache.extend(composer.all_routines());
        // for all index
        for routine in index.entt_to_kind.keys() {
            // if being composed
            if cache.contains(routine) {
                // these routines are already in index so no modification
                // necessary
                // remove from the update set
                cache.remove(routine);
            } else {
                // deactivate routine
                commands.entity(*routine).remove::<ActiveSteeringRoutine>();
            }
        }
        // for remaining composed routines not in indices
        for entt in cache.drain() {
            commands.entity(entt).insert(ActiveSteeringRoutine);
        }
    }
}

/// Contains the engine inputs.
/// Decopling layer between the engine and the minds.
// FIXME: over engineering
#[derive(Debug, Clone, Copy, Default, Inspectable, Component)]
pub struct BoidSteeringSystemOutput {
    /// local space
    pub lin: TVec3,
    /// local space
    pub ang: TVec3,
}

impl std::ops::Add for BoidSteeringSystemOutput {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            lin: self.lin + rhs.lin,
            ang: self.ang + rhs.ang,
        }
    }
}

impl BoidSteeringSystemOutput {
    #[inline]
    pub fn is_zero(&self) -> bool {
        // TODO: benchmark this vs. `TVec3.max_element() < TReal::EPSILON`
        // self.lin.length_squared() < TReal::EPSILON && self.ang.length_squared() < TReal::EPSILON
        self.lin.max_element() < TReal::EPSILON && self.ang.max_element() < TReal::EPSILON
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
#[derive(Debug, Clone, Copy)]
pub struct SteeringRoutineWeight {
    lin: TReal,
    ang: TReal,
}

impl Default for SteeringRoutineWeight {
    fn default() -> Self {
        Self { lin: 1., ang: 1. }
    }
}

impl std::ops::Mul<BoidSteeringSystemOutput> for SteeringRoutineWeight {
    type Output = BoidSteeringSystemOutput;

    fn mul(self, rhs: BoidSteeringSystemOutput) -> Self::Output {
        BoidSteeringSystemOutput {
            lin: rhs.lin * self.lin,
            ang: rhs.ang * self.ang,
        }
    }
}

// Boid mind Component
#[derive(Debug, Clone, Component)]
pub enum SteeringRoutineComposer {
    None,
    Single {
        entt: Entity,
    },
    // Linear sum of the routine outputs
    WeightSummed {
        routines: smallvec::SmallVec<[(SteeringRoutineWeight, Entity); 2]>,
    },
    /// The first routine that returns a non zero value will be used.
    PriorityOverride {
        routines: smallvec::SmallVec<[Entity; 4]>,
    },
}

impl Default for SteeringRoutineComposer {
    fn default() -> Self {
        Self::None
    }
}

impl SteeringRoutineComposer {
    /// This returns a vector of all the routines currently being composed.
    pub fn all_routines(&self) -> smallvec::SmallVec<[Entity; 4]> {
        match self {
            SteeringRoutineComposer::Single { entt } => smallvec::smallvec![*entt],
            SteeringRoutineComposer::WeightSummed { routines } => {
                routines.iter().map(|(_, entt)| *entt).collect()
            }
            SteeringRoutineComposer::PriorityOverride { routines } => routines.clone(),
            SteeringRoutineComposer::None => Default::default(),
        }
    }

    fn get_active_res(
        xform: &GlobalTransform,
        lin_res: Option<&LinearRoutineOutput>,
        ang_res: Option<&AngularRoutineOutput>,
    ) -> BoidSteeringSystemOutput {
        let mut active_res = BoidSteeringSystemOutput::default();
        let mut is_empty = true;
        if let Some(lin_res) = lin_res {
            // let local_lin_inp = (xform.rotation.inverse() * lin_res.0) + (-TVec3::Z * 0.15);// add foward movement
            let local_lin_inp = xform.rotation.inverse() * lin_res.0; // add foward movement
            active_res.lin = local_lin_inp;
            is_empty = false;
        }

        if let Some(ang_res) = ang_res {
            active_res.ang = ang_res.0;
        } else {
            // defaults to look where you want to go
            active_res.ang = look_to(active_res.lin);
            is_empty = false;
        }
        if is_empty {
            tracing::error!("Routine doesn't have linear or angular results");
        }
        active_res
    }
}

/// Synchronizes [`ActiveRoutineResult`] to the engine inputs.
///
/// TODO: merge this with [`routine_composer`]
pub fn mind_update_engine_input(
    mut crafts: Query<(
        &BoidSteeringSystemOutput,
        &mut engine::LinearEngineState,
        &mut engine::AngularEngineState,
        &engine::EngineConfig,
    )>,
) {
    crafts
        .iter_mut()
        .for_each(|(routine_res, mut lin_state, mut ang_state, config)| {
            lin_state.input = routine_res.lin * config.linvel_limit.abs();
            ang_state.input = routine_res.ang;
        });
}

/// This system sets the crafts' [`ActiveRoutineOutput`] and is decopuling layer
/// between the craft mind and whatever steering system is currently active. Right now, it's a dumb
/// system but later on should be replaced with some decision making layer.
pub fn routine_composer(
    mut crafts: Query<(
        &mut BoidSteeringSystemOutput,
        &SteeringRoutineComposer,
        &boid::BoidMindConfig,
        &GlobalTransform,
    )>,
    routines: Query<
        (Option<&LinearRoutineOutput>, Option<&AngularRoutineOutput>),
        With<SteeringRoutine>,
    >,
    // routines: Query<&LinearRoutineResult, With<steering_systems::Intercept>>,
    //egui_context: ResMut<bevy_egui::EguiContext>,
) {
    for (mut active_res, active_routines, config, xform) in crafts.iter_mut() {
        // FIXME: i hate this code
        match &active_routines {
            SteeringRoutineComposer::None => *active_res = BoidSteeringSystemOutput::default(),
            SteeringRoutineComposer::Single { entt } => {
                if let Ok((lin_res, ang_res)) = routines.get(*entt) {
                    *active_res = SteeringRoutineComposer::get_active_res(xform, lin_res, ang_res);
                } else {
                    *active_res = BoidSteeringSystemOutput::default();
                    tracing::error!("routine not found for ActiveRoutines::Single");
                }
            }
            SteeringRoutineComposer::WeightSummed { routines: summed } => {
                // zero it out first
                let mut sum = BoidSteeringSystemOutput::default();
                for (weight, entt) in summed {
                    if let Ok((lin_res, ang_res)) = routines.get(*entt) {
                        sum = sum
                            + (*weight
                                * SteeringRoutineComposer::get_active_res(xform, lin_res, ang_res));
                    } else {
                        tracing::error!("routine not found for ActiveRoutines::WeightSummed");
                        *active_res = BoidSteeringSystemOutput::default();
                        break;
                    }
                }
                *active_res = sum;
            }
            // FIXME: CLEAN ME UP
            SteeringRoutineComposer::PriorityOverride { routines: priority } => {
                // zero it out first
                *active_res = BoidSteeringSystemOutput::default();
                'priority_loop: for entt in priority {
                    if let Ok((lin_res, ang_res)) = routines.get(*entt) {
                        let is_zero = match (lin_res, ang_res) {
                            (Some(lin_res), Some(ang_res)) => {
                                lin_res.0.length_squared() < TReal::EPSILON
                                    && ang_res.0.length_squared() < TReal::EPSILON
                            }
                            (Some(lin_res), None) => lin_res.0.length_squared() < TReal::EPSILON,
                            (None, Some(ang_res)) => ang_res.0.length_squared() < TReal::EPSILON,
                            (None, None) => {
                                tracing::error!("result less routine");
                                true
                            }
                        };
                        if !is_zero {
                            *active_res =
                                SteeringRoutineComposer::get_active_res(xform, lin_res, ang_res);
                            break 'priority_loop;
                        }
                    } else {
                        tracing::error!("routine not found for ActiveRoutines::WeightSummed");
                        *active_res = BoidSteeringSystemOutput::default();
                        break;
                    }
                }
            }
        }
        active_res.ang *= config.angular_input_multiplier;
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
    for (craft_entt, composer, mut index) in crafts.iter_mut() {
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
#[component(storage = "SparseSet")]
pub struct LinearRoutineOutput(pub TVec3);

impl From<TVec3> for LinearRoutineOutput {
    fn from(v: TVec3) -> Self {
        Self(v)
    }
}

/// Output of angular steering routines which is usually angular velocity desired next frame in local space.
#[derive(Debug, Clone, Copy, Default, Inspectable, Component)]
#[component(storage = "SparseSet")]
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
