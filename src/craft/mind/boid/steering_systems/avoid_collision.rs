use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_rapier3d::prelude::*;

use super::{
    steering_behaviours, ActiveRoutine, LinOnlyRoutineBundle, LinearRoutineOutput, SteeringRoutine,
};
use crate::craft::{attire::*, engine::*};
use crate::math::*;

// TODO: consider separating state and parameters
#[derive(Debug, Clone, Component, bevy_inspector_egui::Inspectable)]
pub struct AvoidCollision {
    pub fwd_prediction_secs: f32,
    // #[reflect(ignore)]
    #[inspectable(ignore)]
    pub raycast_exclusion: smallvec::SmallVec<[ColliderHandle; 4]>,
    /// in world space
    pub last_dodge_dir: TVec3,
    pub last_dodge_timestamp: f64,
    pub upheld_dodge_seconds: f64,
}

impl Default for AvoidCollision {
    fn default() -> Self {
        Self {
            fwd_prediction_secs: 5.0,
            raycast_exclusion: Default::default(),
            last_dodge_dir: Default::default(),
            last_dodge_timestamp: Default::default(),
            upheld_dodge_seconds: 1.5,
        }
    }
}

pub type AvoidCollisionRoutineBundle = LinOnlyRoutineBundle<AvoidCollision>;

pub fn avoid_collision(
    // NOTE: this steering system is stateful.
    mut routines: Query<
        (
            &mut AvoidCollision,
            &SteeringRoutine,
            &mut LinearRoutineOutput,
        ),
        With<ActiveRoutine>,
    >,
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
    for (mut avoid_coll, routine, mut lin_out) in routines.iter_mut() {
        *lin_out = Default::default();
        let (xform, config, vel, craft_colliders, lin_state) = crafts
            .get(routine.craft_entt)
            .expect("craft entt not found for routine");

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
            InteractionGroups::new(ColliderGroups::SOLID.bits(), ColliderGroups::SOLID.bits()),
            Some(&|handle| {
                // not a craft collider
                !craft_colliders.0.0[..].contains(&handle)
                    // not in the exclusion list
                    && !avoid_coll.raycast_exclusion[..].contains(&handle)
            }),
        ) {
            // use behavior to avoid it
            *lin_out = steering_behaviours::avoid_obstacle_seblague(
                dir,
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
                xform,
            )
            .into();

            // cache avoidance vector
            avoid_coll.last_dodge_timestamp = time.seconds_since_startup();
            avoid_coll.last_dodge_dir = lin_out.0;
            tracing::info!(
                ?dir,
                ?lin_out,
                ?toi,
                "collision predicted with {handle:?}\n{:?} meters away\n{:?} seconds away\ncorrecting {:?} degrees away",
                hit.toi,
                hit.toi / speed,
                dir.angle_between(lin_out.0) * (180. / crate::math::real::consts::PI)
            );
        }
        // if recently had avoided collision
        else if avoid_coll.last_dodge_timestamp > 0.0
            && time.seconds_since_startup()
                < (avoid_coll.last_dodge_timestamp + avoid_coll.upheld_dodge_seconds)
        {
            // stick to it until upheld time expires
            *lin_out = avoid_coll.last_dodge_dir.into();
        }
    }
    tracing::trace!(avoid_collision_raycast_ctr);
}
