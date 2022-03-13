use deps::*;

use bevy::prelude::*;
use bevy_prototype_debug_lines::*;
use bevy_rapier3d::prelude::*;
use deps::bevy::utils::HashSet;

use super::{
    steering_behaviours, ActiveSteeringRoutine, LinOnlyRoutineBundleExtra, LinearRoutineOutput,
    SteeringRoutine,
};
use crate::craft::attire::*;
use crate::math::*;

#[derive(Debug, Clone, Component)]
pub struct AvoidCollision {
    pub fwd_prediction_secs: f32,
    pub raycast_exclusion: HashSet<ColliderHandle>,
    pub upheld_dodge_seconds: f64,
    pub raycast_toi_modifier: TReal,
    pub cast_shape_radius: TReal,
}

impl AvoidCollision {
    pub fn new(cast_shape_radius: TReal, raycast_toi_modifier: TReal) -> Self {
        Self {
            fwd_prediction_secs: 5.0,
            raycast_exclusion: Default::default(),
            upheld_dodge_seconds: 1.0,
            raycast_toi_modifier,
            cast_shape_radius,
        }
    }
}

#[derive(Debug, Clone, Component, Default)]
pub struct AvoidCollisionState {
    /// in world space
    pub cast_dir: TVec3,
    pub linvel: TVec3,
    pub last_dodge_dir: TVec3,
    pub last_dodge_timestamp: f64,
    pub craft_colliders: HashSet<ColliderHandle>,
}

pub type Bundle = LinOnlyRoutineBundleExtra<AvoidCollision, AvoidCollisionState>;

pub fn butler(
    mut routines: Query<
        (
            &mut AvoidCollisionState,
            &SteeringRoutine,
            ChangeTrackers<SteeringRoutine>,
        ),
        // FIXME: find a way to filter out non-active routines without missing out on changes to RigidBodyCollidersComponent
        // With<ActiveSteeringRoutine>,
    >,
    crafts: Query<(
        &GlobalTransform,
        &crate::craft::engine::LinearEngineState,
        &RigidBodyVelocityComponent,
        &RigidBodyCollidersComponent,
        ChangeTrackers<RigidBodyCollidersComponent>,
    )>,
) {
    for (mut state, routine, routine_change) in routines.iter_mut() {
        let (xform, lin_state, vel, colliders, colliders_change) = crafts
            .get(routine.boid_entt())
            .expect_or_log("craft entt not found for routine");

        state.linvel = vel.linvel.into();
        // use last frame's desired vel dir to cast for obstruction
        state.cast_dir = xform.rotation * lin_state.input.normalize();
        if routine_change.is_added() || colliders_change.is_changed() {
            state.craft_colliders.clear();
            state.craft_colliders.extend(colliders.0 .0.iter());
        }
    }
}

pub fn update(
    // NOTE: this steering system is stateful.
    mut routines: Query<
        (
            &AvoidCollision,
            &mut AvoidCollisionState,
            &SteeringRoutine,
            &mut LinearRoutineOutput,
        ),
        With<ActiveSteeringRoutine>,
    >,
    boids: Query<(&GlobalTransform,)>,
    query_pipeline: Res<QueryPipeline>,
    collider_query: QueryPipelineColliderComponentsQuery,
    time: Res<Time>,
    mut lines: ResMut<DebugLines>,
) {
    let mut avoid_collision_raycast_ctr = 0usize;
    // Wrap the bevy query so it can be used by the query pipeline.
    let collider_set = QueryPipelineColliderComponentsSet(&collider_query);
    for (param, mut state, routine, mut lin_out) in routines.iter_mut() {
        *lin_out = Default::default();
        let (xform,) = boids
            .get(routine.boid_entt)
            .expect_or_log("craft entt not found for routine");

        // let dir = TVec3::from(vel.linvel).normalize();
        let speed = state.linvel.length();
        let toi = param.fwd_prediction_secs * speed;
        let toi = toi + param.raycast_toi_modifier;

        avoid_collision_raycast_ctr += 1;

        let cast_shape = Ball::new(param.cast_shape_radius);
        // shape rotation matters not for balls
        let cast_pose = (xform.translation, xform.rotation).into();
        // if collision predicted
        if let Some((handle, hit)) = query_pipeline.cast_shape(
            &collider_set,
            &cast_pose,
            // world space
            &state.cast_dir.into(),
            &cast_shape,
            toi,
            InteractionGroups::new(
                ColliderGroups::SOLID.bits(),
                (ColliderGroups::SOLID/* | ColliderGroups::CRAFT_SOLID */).bits(),
            ),
            Some(&|handle| {
                // not a craft collider
                !state.craft_colliders.contains(&handle)
                    // not in the exclusion list
                    && !param.raycast_exclusion.contains(&handle)
            }),
        ) {
            lines.line_colored(xform.translation, state.cast_dir * hit.toi, 0., Color::RED);
            // use behavior to avoid it
            *lin_out = steering_behaviours::avoid_obstacle_seblague(
                state.cast_dir,
                &mut |cast_dir| {
                    lines.line_colored(xform.translation, cast_dir * toi, 0., Color::BLUE);
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
                                !state.craft_colliders.contains(&handle)
                                // not in the exclusion list
                                && !param.raycast_exclusion.contains(&handle)
                            }),
                        )
                        .is_some()
                },
                xform,
            )
            .into();
            // *lin_out = xform.left().into();

            lines.line_colored(xform.translation, lin_out.0 * toi, 0., Color::GREEN);

            // cache avoidance vector
            state.last_dodge_timestamp = time.seconds_since_startup();
            state.last_dodge_dir = lin_out.0;
            tracing::trace!(
                ?state.cast_dir,
                ?lin_out,
                ?toi,
                "collision predicted with {handle:?}\n{:?} meters away\n{:?} seconds away\ncorrecting {:?} degrees away",
                hit.toi,
                hit.toi / speed,
                state.cast_dir.angle_between(lin_out.0) * (180. / crate::math::real::consts::PI)
            );
        }
        // if recently had avoided collision
        else if state.last_dodge_timestamp > 0.0
            && time.seconds_since_startup()
                < (state.last_dodge_timestamp + param.upheld_dodge_seconds)
        {
            // stick to it until upheld time expires
            *lin_out = state.last_dodge_dir.into();
        }
    }
    tracing::trace!(avoid_collision_raycast_ctr);
}
