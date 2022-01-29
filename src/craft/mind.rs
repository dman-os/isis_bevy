use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_inspector_egui::Inspectable;
use bevy_inspector_egui::RegisterInspectable;
use bevy_rapier3d::prelude::*;

use crate::craft::engine::*;
use crate::math::*;

pub struct MindPlugin;

impl Plugin for MindPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(mind_update_engine_input)
            .add_system(craft_mind_smarts)
            .add_system(steering_systems::intercept)
            .add_system(steering_systems::fly_with_flock)
            .add_system(steering_systems::avoid_collision)
            .add_system(steering_systems::seek)
            .add_system(update_flocks)
            .register_inspectable::<MindConfig>()
            .register_inspectable::<ActiveRoutineResult>()
            .register_inspectable::<LinearRoutineResult>()
            .register_inspectable::<AngularRoutineResult>()
            .register_inspectable::<steering_systems::AvoidCollision>();
    }
}

#[derive(Debug, Clone, Copy, Inspectable, Component)]
pub struct MindConfig {
    pub angular_input_multiplier: TReal,
}

impl Default for MindConfig {
    fn default() -> Self {
        Self {
            angular_input_multiplier: 3.,
        }
    }
}

#[derive(Bundle, Default)]
pub struct CraftMindBundle {
    pub routine_output: ActiveRoutineResult,
    pub config: MindConfig,
}

#[derive(Debug, Clone, Copy, Default, Inspectable, Component)]
pub struct ActiveRoutineResult {
    /// local space
    lin: TVec3,
    /// local space
    ang: TVec3,
}

impl std::ops::Add for ActiveRoutineResult {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            lin: self.lin + rhs.lin,
            ang: self.ang + rhs.ang,
        }
    }
}

impl ActiveRoutineResult {
    #[inline]
    pub fn is_zero(&self) -> bool {
        // TODO: benchmark this vs. `TVec3.max_element() < TReal::EPSILON`
        // self.lin.length_squared() < TReal::EPSILON && self.ang.length_squared() < TReal::EPSILON
        self.lin.max_element() < TReal::EPSILON && self.ang.max_element() < TReal::EPSILON
    }
}

/// Output of linear steering routines which is usually linear velocity desired next frame in
/// fraction of [`EngineConfig:.linear_v_limit`] in world space.
#[derive(Debug, Clone, Copy, Default, Inspectable, Component)]
#[component(storage = "SparseSet")]
pub struct LinearRoutineResult(pub TVec3);

/// Output of angular steering routines which is usually angular velocity desired next frame in local space.
#[derive(Debug, Clone, Copy, Default, Inspectable, Component)]
#[component(storage = "SparseSet")]
pub struct AngularRoutineResult(pub TVec3);

pub fn mind_update_engine_input(
    mut crafts: Query<(
        &ActiveRoutineResult,
        &mut LinearEngineState,
        &mut AngularEngineState,
        &EngineConfig,
    )>,
) {
    crafts
        .iter_mut()
        .for_each(|(routine_res, mut lin_state, mut ang_state, config)| {
            lin_state.input = routine_res.lin * config.linear_v_limit;
            ang_state.input = routine_res.ang;
        });
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ActiveRoutineWeight {
    lin: TReal,
    ang: TReal,
}

impl std::ops::Mul<ActiveRoutineResult> for ActiveRoutineWeight {
    type Output = ActiveRoutineResult;

    fn mul(self, rhs: ActiveRoutineResult) -> Self::Output {
        ActiveRoutineResult {
            lin: rhs.lin * self.lin,
            ang: rhs.ang * self.ang,
        }
    }
}

// FIXME: find a new fucking name
#[derive(Component)]
pub enum ActiveRoutines {
    Single {
        entt: Entity,
    },
    WeightSummed {
        routines: smallvec::SmallVec<[(ActiveRoutineWeight, Entity); 2]>,
    },
    /// The first routine that returns a non zero value will be used.
    PriorityOverride {
        routines: smallvec::SmallVec<[Entity; 4]>,
    },
}

impl ActiveRoutines {
    fn get_active_res(
        xform: &GlobalTransform,
        lin_res: Option<&LinearRoutineResult>,
        ang_res: Option<&AngularRoutineResult>,
    ) -> ActiveRoutineResult {
        let mut active_res = ActiveRoutineResult::default();
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
            active_res.ang = steering_systems::look_at(active_res.lin).0;
            is_empty = false;
        }
        if is_empty {
            tracing::error!("Routine doesn't have linear or angular results");
        }
        active_res
    }
}

/// This system sets the crafts' [`ActiveRoutineOutput`] and is decopuling layer
/// between the craft mind and whatever system is currently active. Right now, it's a dumb
/// system but later on should be replaced with some decision layer.
pub fn craft_mind_smarts(
    mut crafts: Query<(
        &mut ActiveRoutineResult,
        &ActiveRoutines,
        &MindConfig,
        &GlobalTransform,
    )>,
    routines: Query<
        (Option<&LinearRoutineResult>, Option<&AngularRoutineResult>),
        With<steering_systems::SteeringRoutine>,
    >,
    // routines: Query<&LinearRoutineResult, With<steering_systems::Intercept>>,
    //egui_context: ResMut<bevy_egui::EguiContext>,
) {
    for (mut active_res, active_routines, config, xform) in crafts.iter_mut() {
        // FIXME: i hate this code
        match &active_routines {
            ActiveRoutines::Single { entt } => {
                if let Ok((lin_res, ang_res)) = routines.get(*entt) {
                    *active_res = ActiveRoutines::get_active_res(xform, lin_res, ang_res);
                } else {
                    *active_res = ActiveRoutineResult::default();
                    tracing::error!("routine not found for ActiveRoutines::Single");
                }
            }
            ActiveRoutines::WeightSummed { routines: summed } => {
                // zero it out first
                let mut sum = ActiveRoutineResult::default();
                for (weight, entt) in summed {
                    if let Ok((lin_res, ang_res)) = routines.get(*entt) {
                        sum = sum
                            + (*weight * ActiveRoutines::get_active_res(xform, lin_res, ang_res));
                    } else {
                        tracing::error!("routine not found for ActiveRoutines::WeightSummed");
                        *active_res = ActiveRoutineResult::default();
                        break;
                    }
                }
                *active_res = sum;
            }
            // FIXME: CLEAN ME UP
            ActiveRoutines::PriorityOverride { routines: priority } => {
                // zero it out first
                *active_res = ActiveRoutineResult::default();
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
                            *active_res = ActiveRoutines::get_active_res(xform, lin_res, ang_res);
                            break 'priority_loop;
                        }
                    } else {
                        tracing::error!("routine not found for ActiveRoutines::WeightSummed");
                        *active_res = ActiveRoutineResult::default();
                        break;
                    }
                }
            }
        }
        active_res.ang *= config.angular_input_multiplier;
    }
}

pub enum ScanPresence {
    Obstacle {
        name: String,
        // silhouette_collider: ColliderHandle
    },
    Boid {
        name: String,
        rigidbody: RigidBodyHandle,
    },
}

#[derive(Debug, Default, Component)]
pub struct GroupMind {
    pub members: smallvec::SmallVec<[Entity; 8]>,
}

#[derive(Debug, Clone, Copy, Component)]
pub struct CraftGroup(pub Entity);

#[derive(Debug, Default, Component)]
pub struct BoidFlock {
    pub craft_positions: Vec<TVec3>,
    pub heading_sum: TVec3,
    pub avg_heading: TVec3,
    pub center_sum: TVec3,
    pub center: TVec3,
    pub member_count: usize,
}

pub fn update_flocks(
    mut flocks: Query<(&GroupMind, &mut BoidFlock)>,
    crafts: Query<(&GlobalTransform, &RigidBodyVelocityComponent)>,
) {
    for (g_mind, mut flock) in flocks.iter_mut() {
        flock.craft_positions.clear();
        flock.heading_sum = TVec3::ZERO;
        flock.center_sum = TVec3::ZERO;
        for craft in g_mind.members.iter() {
            if let Ok((xform, vel)) = crafts.get(*craft) {
                flock.heading_sum += TVec3::from(vel.linvel);
                flock.center_sum += xform.translation;
                flock.craft_positions.push(xform.translation);
            } else {
                tracing::error!("unable to find group mind member when updating flocks");
            }
        }
        flock.member_count = g_mind.members.len();
        flock.avg_heading = flock.heading_sum / g_mind.members.len() as TReal;
        flock.center = flock.center_sum / g_mind.members.len() as TReal;
    }
}

pub mod steering_systems {
    use deps::*;

    use bevy::{ecs as bevy_ecs, prelude::*, reflect as bevy_reflect};
    use bevy_rapier3d::prelude::*;

    use super::{AngularRoutineResult, BoidFlock, CraftGroup, LinearRoutineResult};
    use crate::craft::attire::*;
    use crate::craft::engine::*;
    use crate::math::*;

    #[derive(Component)]
    pub struct SteeringRoutine;

    #[derive(Bundle)]
    pub struct LinOnlyRoutineBundle<P>
    where
        P: Component,
    {
        pub param: P,
        pub output: LinearRoutineResult,
        pub mark: SteeringRoutine,
    }

    impl<P> LinOnlyRoutineBundle<P>
    where
        P: Component,
    {
        pub fn new(param: P) -> Self {
            Self {
                param,
                output: Default::default(),
                mark: SteeringRoutine,
            }
        }
    }

    #[derive(Bundle)]
    pub struct LinAngRoutineBundle<P>
    where
        P: Component,
    {
        pub param: P,
        pub lin_res: LinearRoutineResult,
        pub ang_res: AngularRoutineResult,
        pub mark: SteeringRoutine,
    }

    impl<P> LinAngRoutineBundle<P>
    where
        P: Component,
    {
        pub fn new(param: P) -> Self {
            Self {
                param,
                lin_res: LinearRoutineResult::default(),
                ang_res: AngularRoutineResult::default(),
                mark: SteeringRoutine,
            }
        }
    }

    #[derive(Debug, Clone, Component, bevy_inspector_egui::Inspectable)]
    pub struct AvoidCollision {
        pub craft_entt: Entity,
        pub fwd_prediction_secs: f32,
        // #[reflect(ignore)]
        #[inspectable(ignore)]
        pub raycast_exclusion: smallvec::SmallVec<[ColliderHandle; 4]>,
        /// in world space
        pub last_dodge_direction: TVec3,
        pub last_dodge_timestamp: f64,
        pub upheld_dodge_seconds: f64,
    }

    pub type AvoidCollisionRoutineBundle = LinOnlyRoutineBundle<AvoidCollision>;

    pub fn avoid_collision(
        // NOTE: this steering system is stateful.
        mut routines: Query<(Entity, &mut AvoidCollision, &mut LinearRoutineResult)>,
        crafts: Query<(
            &GlobalTransform,
            &EngineConfig,
            &RigidBodyVelocityComponent,
            &RigidBodyCollidersComponent,
            &crate::craft::engine::LinearEngineState,
        )>,
        query_pipeline: Res<QueryPipeline>,
        collider_query: QueryPipelineColliderComponentsQuery,
        time: Res<Time>,
    ) {
        let mut avoid_collision_raycast_ctr = 0usize;
        // Wrap the bevy query so it can be used by the query pipeline.
        let collider_set = QueryPipelineColliderComponentsSet(&collider_query);
        for (_, mut avoid_coll, mut result) in routines.iter_mut() {
            *result = LinearRoutineResult::default();
            if let Ok((xform, config, vel, craft_colliders, lin_state)) =
                crafts.get(avoid_coll.craft_entt)
            {
                // let dir = TVec3::from(vel.linvel).normalize();
                let dir = xform.rotation * lin_state.input.normalize();
                let speed = vel.linvel.magnitude();
                let toi = avoid_coll.fwd_prediction_secs * speed;
                // adjust for the dimensions of the craft
                let widest_dim = config.extents.max_element();
                let toi = toi + widest_dim;
                avoid_collision_raycast_ctr += 1;
                let cast_shape = Ball::new(0.5 * widest_dim);
                // shape rotation matters not for balls
                let cast_pose = (xform.translation, xform.rotation).into();
                // if collision predicted
                if let Some((handle, hit)) = query_pipeline.cast_shape(
                    &collider_set,
                    &cast_pose,
                    // world space
                    &dir.into(),
                    &cast_shape,
                    toi,
                    InteractionGroups::new(
                        ColliderGroups::SOLID.bits(),
                        ColliderGroups::SOLID.bits(),
                    ),
                    Some(&|handle| {
                        // not a craft collider
                        !craft_colliders.0.0[..].contains(&handle)
                            // not in the exclusion list
                            && !avoid_coll.raycast_exclusion[..].contains(&handle)
                    }),
                ) {
                    // use behavior to avoid it
                    *result = LinearRoutineResult(steering_behaviours::avoid_obstacle_seblague(
                        dir.into(),
                        &mut |cast_dir| {
                            avoid_collision_raycast_ctr += 1;
                            query_pipeline
                                .cast_shape(
                                    &collider_set,
                                    &cast_pose,
                                    &cast_dir.into(),
                                    &cast_shape,
                                    toi,
                                    *OBSTACLE_COLLIDER_IGROUP,
                                    Some(&|handle| {
                                        // not a craft collider
                                        !craft_colliders.0.0[..].contains(&handle)
                                        // not in the exclusion list
                                        && !avoid_coll.raycast_exclusion[..].contains(&handle)
                                    }),
                                )
                                .is_some()
                        },
                        &xform,
                    ));
                    // cache avoidance vector
                    avoid_coll.last_dodge_timestamp = time.seconds_since_startup();
                    avoid_coll.last_dodge_direction = result.0;
                    tracing::trace!(
                        ?dir,
                        ?result,
                        ?toi,
                        "collision predicted with {:?}\n{:?} meters away\n{:?} seconds away\ncorrecting {:?} degrees away",
                        handle,
                        hit.toi,
                        hit.toi / speed,
                        dir.angle_between(result.0) * (180. / crate::math::real::consts::PI)
                    );
                }
                // if recently had avoided collision
                else if avoid_coll.last_dodge_timestamp > 0.0
                    && time.seconds_since_startup()
                        < (avoid_coll.last_dodge_timestamp + avoid_coll.upheld_dodge_seconds)
                {
                    // stick to it until upheld time expires
                    *result = LinearRoutineResult(avoid_coll.last_dodge_direction);
                }
            }
            //*result = LinearRoutineResult(TVec3::ONE * 100.0);
        }
        tracing::trace!(avoid_collision_raycast_ctr);
    }

    #[derive(Debug, Clone, Copy, Component)]
    #[component(storage = "SparseSet")]
    pub struct Intercept {
        pub craft_entt: Entity,
        pub quarry_rb: RigidBodyHandle,
    }
    pub type InterceptRoutineBundle = LinOnlyRoutineBundle<Intercept>;

    pub fn intercept(
        mut routines: Query<(Entity, &Intercept, &mut LinearRoutineResult)>,
        crafts: Query<(&GlobalTransform, &EngineConfig)>, // crafts
        quarries: Query<(&RigidBodyPositionComponent, &RigidBodyVelocityComponent)>, // quarries
    ) {
        for (routine_id, params, mut output) in routines.iter_mut() {
            match (
                crafts.get(params.craft_entt),
                quarries.get(params.quarry_rb.entity()),
            ) {
                (Ok((xform, config)), Ok((quarry_pos, quarry_vel))) => {
                    *output = intercept_rb(quarry_pos, quarry_vel, xform, config);
                }
                _err => {
                    tracing::error!(
                        "unable to find craft_entt for Intercept routine {:?}",
                        routine_id
                    );
                }
            }
        }
    }

    #[inline]
    fn intercept_rb(
        quarry_pos: &RigidBodyPosition,
        quarry_vel: &RigidBodyVelocity,
        current_xform: &GlobalTransform,
        config: &EngineConfig,
    ) -> LinearRoutineResult {
        let linear_v_limit = config.linear_v_limit;
        LinearRoutineResult(steering_behaviours::intercept_target(
            current_xform.translation,
            // negate it since forward is negative
            -linear_v_limit.z,
            quarry_pos.position.translation.into(),
            quarry_vel.linvel.into(),
        ))
    }

    #[inline]
    pub fn look_at(local_dir: TVec3) -> AngularRoutineResult {
        AngularRoutineResult({
            let fwd = -TVec3::Z;
            let dir = local_dir;
            fwd.angle_between(dir) * fwd.cross(dir)
        })
        /*AngularRoutineResult({
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
        })*/
    }

    #[derive(Debug, Clone, Copy, Component)]
    pub struct FlyWithFlock {
        pub craft_entt: Entity,
    }

    pub type FlyWithFlockRoutineBundle = LinAngRoutineBundle<FlyWithFlock>;

    pub fn fly_with_flock(
        mut routines: Query<(
            Entity,
            &FlyWithFlock,
            &mut LinearRoutineResult,
            &mut AngularRoutineResult,
        )>,
        flocks: Query<&BoidFlock>,
        crafts: Query<(&GlobalTransform, &RigidBodyVelocityComponent, &CraftGroup)>, // crafts
    ) {
        for (routine_id, params, mut lin_out, mut ang_out) in routines.iter_mut() {
            if let Ok((xform, vel, craft_group)) = crafts.get(params.craft_entt) {
                if let Ok(flock) = flocks.get(craft_group.0) {
                    let (cohesion, allignment, separation) = (
                        steering_behaviours::cohesion(
                            xform.translation,
                            flock.member_count,
                            flock.center_sum,
                        ),
                        steering_behaviours::allignment(
                            vel.linvel.into(),
                            flock.member_count,
                            flock.heading_sum,
                        ),
                        // NOTE: 10x multiplier
                        10.0 * steering_behaviours::separation(
                            xform.translation,
                            &flock.craft_positions[..],
                        ),
                    );
                    *lin_out = LinearRoutineResult(cohesion + allignment + separation);
                    *ang_out = look_at(xform.rotation * allignment);
                } else {
                    tracing::error!("unable to find craft_group for fly_with_flock routine");
                }
            } else {
                tracing::error!(
                    "unable to find craft_entt for FlyWithFlock routine {:?}",
                    routine_id,
                );
            }
        }
    }
    #[derive(Debug, Clone, Copy, Component)]
    pub enum SeekTarget {
        /// must have a global xform
        Object { entt: Entity },
        /// must be global position
        Position { pos: TVec3 },
    }

    #[derive(Debug, Clone, Copy, Component)]
    pub struct Seek {
        pub craft_entt: Entity,
        pub target: SeekTarget,
    }

    pub type SeekRoutineBundle = LinOnlyRoutineBundle<Seek>;

    pub fn seek(
        mut routines: Query<(Entity, &Seek, &mut LinearRoutineResult)>,
        objects: Query<&GlobalTransform>,
    ) {
        for (routine_id, params, mut output) in routines.iter_mut() {
            if let Ok(xform) = objects.get(params.craft_entt) {
                match params.target {
                    SeekTarget::Object { entt } => match objects.get(entt) {
                        Ok(obj_xform) => {
                            *output = LinearRoutineResult(steering_behaviours::seek_position(
                                xform.translation,
                                obj_xform.translation,
                            ))
                        }
                        Err(err) => {
                            tracing::error!("error getting SeekTarget Object g_xform: {:?}", err);
                        }
                    },
                    SeekTarget::Position { pos } => {
                        *output = LinearRoutineResult(steering_behaviours::seek_position(
                            xform.translation,
                            pos,
                        ))
                    }
                }
            } else {
                tracing::error!(
                    "unable to find craft_entt for Seek routine {:?}",
                    routine_id
                );
            }
        }
    }

    pub mod steering_behaviours {
        use crate::math::*;
        use bevy::prelude::*;
        use deps::*;

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
        pub fn allignment(
            current_vel: TVec3,
            flock_size: usize,
            flock_heading_sum: TVec3,
        ) -> TVec3 {
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
            const RAY_COUNT: usize = 90;
            const RAY_DIRECTIONS: Lazy<[TVec3; RAY_COUNT]> = Lazy::new(|| {
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
    }
}
