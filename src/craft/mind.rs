use deps::*;

use bevy::{
    ecs::{
        self as bevy_ecs,
        component::{ComponentDescriptor, StorageType},
    },
    prelude::*,
};
use bevy_inspector_egui::Inspectable;
use bevy_rapier3d::prelude::*;

use crate::craft::engine::*;
use crate::math::*;

pub struct MindPlugin;
impl Plugin for MindPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.register_component(ComponentDescriptor::new::<LinearRoutineResult>(
            StorageType::SparseSet,
        ))
        .register_component(ComponentDescriptor::new::<AngularRoutineResult>(
            StorageType::SparseSet,
        ))
        .register_component(ComponentDescriptor::new::<steering_systems::Intercept>(
            StorageType::SparseSet,
        ))
        .add_system(mind_update_engine_input.system())
        .add_system(craft_mind_smarts.system())
        .add_system(steering_systems::intercept.system())
        .add_system(steering_systems::fly_with_flock.system())
        .add_system(update_flocks.system());
    }
}

#[derive(Debug, Clone, Copy, Inspectable)]
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

#[derive(Debug, Clone, Copy, Default, Inspectable)]
pub struct ActiveRoutineResult {
    /// local space
    lin: TVec3,
    /// local space
    ang: TVec3,
}

/// Output of linear steering routines which is usually linear velocity desired next frame in
/// fraction of [`EngineConfig:.linear_v_limit`] in world space.
#[derive(Debug, Clone, Copy, Default, Inspectable)]
pub struct LinearRoutineResult(pub TVec3);

/// Output of angular steering routines which is usually angular velocity desired next frame in local space.
#[derive(Debug, Clone, Copy, Default, Inspectable)]
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

/// As of now, we always use
pub struct ActiveRoutines(pub Entity);

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
    routines: Query<(&LinearRoutineResult, Option<&AngularRoutineResult>)>,
    // routines: Query<&LinearRoutineResult, With<steering_systems::Intercept>>,
    //egui_context: ResMut<bevy_egui::EguiContext>,
) {
    for (mut active_res, routine_id, config, xform) in crafts.iter_mut() {
        if let Ok((lin_res, ang_res)) = routines.get(routine_id.0) {
            // let local_lin_inp = xform.rotation.inverse() * (lin_res.0 + (-TVec3::Z * 0.15));// add foward movement
            let local_lin_inp = xform.rotation.inverse() * lin_res.0; // add foward movement
            active_res.lin = local_lin_inp;
            if let Some(ang_res) = ang_res {
                active_res.ang = ang_res.0;
            } else {
                active_res.ang =
                    config.angular_input_multiplier * steering_systems::look_at(local_lin_inp).0;
            }
        } else {
            tracing::error!("no routine found for craft");
        }
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

#[derive(Debug, Default)]
pub struct GroupMind {
    pub members: smallvec::SmallVec<[Entity; 8]>,
}

#[derive(Debug, Clone, Copy)]
pub struct CraftGroup(pub Entity);

#[derive(Debug, Default)]
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
    crafts: Query<(&GlobalTransform, &RigidBodyVelocity)>,
) {
    for (g_mind, mut flock) in flocks.iter_mut() {
        flock.craft_positions.clear();
        flock.heading_sum = TVec3::ZERO;
        flock.center_sum = TVec3::ZERO;
        for craft in g_mind.members.iter() {
            if let Ok((xform, vel)) = crafts.get(*craft) {
                flock.heading_sum += vel.linvel.into();
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

    use bevy::{ecs as bevy_ecs, prelude::*};
    use bevy_rapier3d::prelude::*;

    use super::{AngularRoutineResult, BoidFlock, CraftGroup, LinearRoutineResult};
    use crate::craft::engine::*;
    use crate::math::*;

    /*#[derive(Debug, Clone)]
    pub struct AvoidCollision {
        pub craft_entt: Entity,
        pub fwd_prediction_secs: f32,
        pub raycast_exclusion: smallvec::SmallVec<[ColliderHandle; 8]>,
    }

    pub fn avoid_collision(
        mut routines: Query<(Entity, &AvoidCollision, &mut LinearRoutineResult)>,
        crafts: Query<(&GlobalTransform, &EngineConfig, &RigidBodyVelocity)>, // crafts
        query_pipeline: Res<QueryPipeline>,
        collider_query: QueryPipelineColliderComponentsQuery,
    ) {
        // Wrap the bevy query so it can be used by the query pipeline.
        let collider_set = QueryPipelineColliderComponentsSet(&collider_query);
        for (_, avoid_coll, result) in routines.iter_mut() {
            if let Ok((xform, config, vel)) = crafts.get(avoid_coll.craft_entt) {
                // check for collision
                let ray = vel.linvel;
                let widest_dim = config.extents.max_element();
                if let Some((handle, toi)) = query_pipeline.cast_shape(
                    &collider_set,
                    &(xform.translation, xform.rotation).into(),
                    &ray,
                    &Ball::new(0.5 * widest_dim),
                    avoid_coll.fwd_prediction_secs,
                    InteractionGroups::all(),
                    Some(&|handle| avoid_coll.raycast_exclusion[..].contains(&handle)),
                ) {}
            } else {
                tracing::error!("craft_entt of AvoidCollision routine not found");
            }
        }
    }*/

    #[derive(Debug, Clone, Copy)]
    pub struct Intercept {
        pub craft_entt: Entity,
        pub quarry_rb: RigidBodyHandle,
    }

    #[derive(Bundle)]
    pub struct InterceptRoutineBundle {
        pub param: Intercept,
        pub output: LinearRoutineResult,
    }

    pub fn intercept(
        mut routines: Query<(Entity, &Intercept, &mut LinearRoutineResult)>,
        crafts: Query<(&GlobalTransform, &EngineConfig)>, // crafts
        quarries: Query<(&RigidBodyPosition, &RigidBodyVelocity)>, // quarries
    ) {
        for (routine_id, params, mut output) in routines.iter_mut() {
            match (
                crafts.get(params.craft_entt),
                quarries.get(params.quarry_rb.entity()),
            ) {
                (Ok((xform, config)), Ok((quarry_pos, quarry_vel))) => {
                    *output = intercept_rb(quarry_pos, quarry_vel, xform, config);
                }
                err => {
                    tracing::error!(
                        "invalid params for intercept routine {:?}: {:?}",
                        routine_id,
                        err
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

    pub struct FlyWithFlock {
        pub craft_entt: Entity,
    }

    #[derive(Bundle)]
    pub struct FlyWithFlockRoutineBundle {
        pub param: FlyWithFlock,
        pub lin_res: LinearRoutineResult,
        pub ang_res: AngularRoutineResult,
    }

    pub fn fly_with_flock(
        mut routines: Query<(
            Entity,
            &FlyWithFlock,
            &mut LinearRoutineResult,
            &mut AngularRoutineResult,
        )>,
        flocks: Query<&BoidFlock>,
        crafts: Query<(&GlobalTransform, &RigidBodyVelocity, &CraftGroup)>, // crafts
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
                tracing::error!("invalid params for fly_with_flock routine {:?}", routine_id,);
            }
        }
    }

    pub mod steering_behaviours {
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
    }
}
