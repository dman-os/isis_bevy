use deps::*;

use bevy::{
    ecs::{
        self as bevy_ecs,
        component::{ComponentDescriptor, StorageType},
    },
    prelude::*,
};
use bevy_rapier3d::prelude::*;

use crate::craft::engine::*;
use crate::math::{Real, *};

pub struct MindPlugin;
impl Plugin for MindPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.register_component(ComponentDescriptor::new::<SteeringSystemOutput>(
            StorageType::SparseSet,
        ))
        .register_component(ComponentDescriptor::new::<steering_systems::Intercept>(
            StorageType::SparseSet,
        ))
        .add_system(init_default_routines.system())
        .add_system(craft_mind_steering_routines.system())
        .add_system(craft_mind_smarts.system())
        .add_system(steering_systems::intercept.system());
    }
}

macro_rules! steering_routine_port {
    ($type_name:ident) => {
        #[derive(Debug, Clone, Copy)]
        pub enum $type_name {
            Linear {
                input: Vector3,
                //phantom: PhantomData<T>,
            },
            Angular {
                input: Vector3,
                //phantom: PhantomData<T>,
            },
            LinearAndAngular {
                linear: Vector3,
                angular: Vector3,
                //phantom: PhantomData<T>,
            },
        }

        impl Default for $type_name {
            fn default() -> Self {
                Self::Linear {
                    input: Default::default(),
                }
            }
        }

        #[allow(clippy::from_over_into)]
        impl Into<ActiveRoutineOutput> for $type_name {
            fn into(self) -> ActiveRoutineOutput {
                match self {
                    Self::Linear { input } => ActiveRoutineOutput {
                        linear_input: input,
                        angular_input: Default::default(),
                    },
                    Self::Angular { input } => ActiveRoutineOutput {
                        angular_input: input,
                        linear_input: Default::default(),
                    },
                    Self::LinearAndAngular { linear, angular } => ActiveRoutineOutput {
                        angular_input: angular,
                        linear_input: linear,
                    },
                }
            }
        }
    };
}
//steering_routine_port!(SteeringSystemInput);
steering_routine_port!(SteeringSystemOutput);

#[derive(Debug, Clone, Copy)]
pub struct MindConfig {
    pub angular_input_multiplier: Real,
}
impl Default for MindConfig {
    fn default() -> Self {
        Self {
            angular_input_multiplier: 3.,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ActiveRoutineOutput {
    /// Linear velocity desired next frame in fraction of [`EngineConfig:.linear_v_limit`] in world
    /// space.
    pub linear_input: Vector3,
    /// Angular velocity desired next frame in local space.
    pub angular_input: Vector3,
}

#[derive(Bundle, Default)]
pub struct CraftMindBundle {
    pub routine_output: ActiveRoutineOutput,
    pub config: MindConfig,
}

pub fn init_default_routines(
    mut commands: Commands,
    player: Res<crate::craft::CurrentCraft>,
    crafts: Query<Entity, (With<MindConfig>, Without<ActiveRoutineId>)>,
) {
    for craft in crafts.iter() {
        let intercept_routine = commands
            .spawn_bundle(steering_systems::InterceptRoutineBundle {
                param: steering_systems::Intercept {
                    craft_entt: craft,
                    quarry_rb: player.0.handle(),
                },
                output: Default::default(),
            })
            .id();
        commands
            .entity(craft)
            .insert(ActiveRoutineId(intercept_routine));
    }
}

/// This is a simple system that updates the engine state according to whatever output
/// dictated by the active system routine.
pub fn craft_mind_steering_routines(
    mut crafts: Query<(
        &ActiveRoutineOutput,
        &mut LinearEngineState,
        &mut AngularEngineState,
        &EngineConfig,
    )>,
) {
    for (routine_output, mut lin_state, mut ang_state, config) in crafts.iter_mut() {
        lin_state.input = routine_output.linear_input * config.linear_v_limit;
        ang_state.input = routine_output.angular_input;
    }
}

pub struct ActiveRoutineId(pub Entity);

/// This system sets the crafts' [`ActiveRoutineOutput`] and is decopuling layer
/// between the craft mind and whatever system is currently active. Right now, it's a dumb
/// system but later on should be replaced with some decision layer.
pub fn craft_mind_smarts(
    mut crafts: Query<(
        &mut ActiveRoutineOutput,
        &ActiveRoutineId,
        &MindConfig,
        &GlobalTransform,
    )>,
    routines: Query<&SteeringSystemOutput, With<steering_systems::Intercept>>,
    //egui_context: ResMut<bevy_egui::EguiContext>,
) {
    for (mut active_output, routine_id, config, xform) in crafts.iter_mut() {
        if let Ok(routine_output) = routines.get(routine_id.0) {
            *active_output = (*routine_output).into();
            active_output.linear_input = xform.rotation.inverse() * active_output.linear_input;
            active_output.angular_input = config.angular_input_multiplier
                * steering_systems::face_local_dir(active_output.linear_input);

            //bevy_egui::egui::Window::new("mind peer").show(egui_context.ctx(), |ui| {
            //ui.label(format!(
            //"intercept linear output: {:+03.1?}",
            //routine_output.linear_input
            //));
            //ui.label(format!(
            //"intercept angular output: {:+03.1?}",
            //steering_systems::face_local_dir(
            ////xform.rotation * routine_output.linear_input,
            //xform.rotation.inverse() * routine_output.linear_input
            //)
            //));
            //ui.label(format!(
            //"fld output: {:+03.1?}",
            //active_output.angular_input
            //));
            //let mut dir = routine_output.linear_input;
            ////dir.z *= -1.;
            //let dir = -dir;
            //let (z, x, y) = {
            ////// basis facing dir
            ////let t = {
            ////let forward = dir.normalize();
            ////let right = Vector3::Y.cross(forward).normalize();
            ////let up = forward.cross(right);
            ////Mat3::from_cols(right, up, forward)
            ////};
            //////t.euler_angles()
            //bevy_rapier3d::na::UnitQuaternion::face_towards(&dir.into(), &Vector3::Y.into())
            //.euler_angles()
            //};
            //let (x, y, z) = (z, x, y);
            //ui.label(format!("eular angle output: {:+03.1?}", (x, y, z)));
            //let delta_angled = [
            //crate::math::delta_angle_radians(0., x).copysign(x),
            //crate::math::delta_angle_radians(0., y).copysign(y),
            //crate::math::delta_angle_radians(0., z).copysign(z),
            //];
            //ui.label(format!("delta angle output: {:+03.1?}", delta_angled));
            //});
        } else {
            tracing::error!("no routine found for craft");
        }
    }
}

pub mod steering_systems {
    use deps::*;

    use bevy::{ecs as bevy_ecs, prelude::*};
    use bevy_rapier3d::prelude::*;

    use crate::craft::engine::*;
    use crate::math::Vector3;
    //use crate::math::{Real, *};

    use super::SteeringSystemOutput;

    //#[derive(Debug, Clone)]
    //pub struct RigidBodySnap {
    //pos: RigidBodyPosition,
    //vel: RigidBodyVelocity,
    //}
    //pub struct RbSnapper {
    //to_be_tracked: HashSet<Entity>,
    //tracked: HashMap<Entity, RigidBodySnap>,
    //}
    //impl RbSnapper {
    //pub fn get(&self, entt: Entity) -> Option<&RigidBodySnap> {
    //if let Some(ii) = self.tracked.get(&entt) {
    //Some(&self.result[*ii])
    //} else {
    //None
    //}
    //}
    //pub fn track(&mut self, entt: Entity) {
    //self.tracked.entry(entt).or_insert(None);
    //}
    //pub fn get_and_track(&mut self, entt: Entity) -> Option<&RigidBodySnap> {
    //if let Some(Some(ii)) = self.tracked.get(&entt) {
    //Some(&self.result[*ii])
    //} else {
    //None
    //}
    //}
    //}
    //fn rb_sensor(
    //mut rb_snapper: ResMut<RbSnapper>,
    //rigidbodies: Query<(&RigidBodyPosition, &RigidBodyVelocity)>,
    //) {
    //rb_snapper.tracked.clear();
    //rb_snapper.to_be_tracked.clear();
    //}
    //
    //pub type RoutineId = u64;

    //#[derive(Debug, Clone, Copy)]
    //pub struct RoutineHandle<R> {
    //pub id: RoutineId,
    //pub phantom: PhantomData<R>,
    //}

    //pub struct SteeringRoutines<P> {
    //last_routine_id: u64,
    //routines: HashMap<RoutineId, (P, SteeringSystemOutput)>,
    //}
    //impl<P> SteeringRoutines<P> {
    //pub fn new() -> Self {
    //Self {
    //last_routine_id: 0,
    //routines: Default::default(),
    //}
    //}
    //pub fn add_routine(&mut self, param: P) -> RoutineHandle<P> {
    //self.last_routine_id += 1;
    //self.routines.insert(
    //self.last_routine_id,
    //(param, SteeringSystemOutput::default()),
    //);
    //RoutineHandle {
    //id: self.last_routine_id,
    //phantom: PhantomData,
    //}
    //}
    //pub fn get_routine(
    //&self,
    //handle: RoutineHandle<P>,
    //) -> Option<&(P, SteeringSystemOutput)> {
    //self.routines.get(&handle.id)
    //}
    //}

    #[derive(Debug, Clone, Copy)]
    pub struct Intercept {
        pub craft_entt: Entity,
        pub quarry_rb: RigidBodyHandle,
    }
    //pub(super) fn intercept(
    //mut state: ResMut<SteeringRoutines<Intercept>>,
    //queries: QuerySet<(
    //Query<(&GlobalTransform, &EngineConfig)>,        // crafts
    //Query<(&RigidBodyPosition, &RigidBodyVelocity)>, // quarries
    //)>,
    //) {
    //for (routine_id, (params, output)) in state.routines.iter_mut() {
    //match (
    //queries.q0().get(params.craft_entt),
    //queries.q1().get(params.quarry_rb.entity()),
    //) {
    //(Ok((xform, config)), Ok(quarry_rb)) => {
    //*output = intercept_rb(quarry_rb, xform, config);
    //}
    //err => {
    //tracing::error!(
    //"invalid params for intercept routine {:?}: {:?}",
    //routine_id,
    //err
    //);
    //}
    //}
    //}
    //}

    #[derive(Bundle)]
    pub struct InterceptRoutineBundle {
        pub param: Intercept,
        pub output: SteeringSystemOutput,
    }

    pub fn intercept(
        mut routines: Query<(Entity, &Intercept, &mut SteeringSystemOutput)>,
        crafts: Query<(&GlobalTransform, &EngineConfig)>, // crafts
        quarries: Query<(&RigidBodyPosition, &RigidBodyVelocity)>, // quarries
    ) {
        for (routine_id, params, mut output) in routines.iter_mut() {
            match (
                crafts.get(params.craft_entt),
                quarries.get(params.quarry_rb.entity()),
            ) {
                (Ok((xform, config)), Ok((quarry_pos, quarry_vel))) => {
                    *output = SteeringSystemOutput::Linear {
                        input: intercept_rb(quarry_pos, quarry_vel, xform, config),
                    };
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
    ) -> Vector3 {
        let linear_v_limit = config.linear_v_limit;
        steering_behaviours::intercept_target(
            current_xform.translation,
            linear_v_limit.z,
            quarry_pos.position.translation.into(),
            quarry_vel.linvel.into(),
        )
    }

    #[inline]
    pub fn face_local_dir(dir: Vector3) -> Vector3 {
        let dir = -dir;
        let (z, x, y) = {
            //// basis facing dir
            //let t = {
            //let forward = dir.normalize();
            //let right = Vector3::Y.cross(forward).normalize();
            //let up = forward.cross(right);
            //Mat3::from_cols(right, up, forward)
            //};
            ////t.euler_angles()
            nalgebra::UnitQuaternion::face_towards(&dir.into(), &Vector3::Y.into()).euler_angles()
        };
        let (x, y, z) = (z, x, y);
        [
            crate::math::delta_angle_radians(0., x).copysign(x),
            crate::math::delta_angle_radians(0., y).copysign(y),
            crate::math::delta_angle_radians(0., z).copysign(z),
        ]
        .into()
    }

    pub mod steering_behaviours {
        use crate::math::{Real, *};

        #[inline]
        pub fn seek_position(current_pos: Vector3, target_pos: Vector3) -> Vector3 {
            (target_pos - current_pos).normalize()
        }

        #[inline]
        pub fn find_intercept_pos(
            current_pos: Vector3,
            travel_speed: Real,
            target_pos: Vector3,
            target_vel: Vector3,
        ) -> Vector3 {
            let relative_pos = target_pos - current_pos;
            let distance_to_target = relative_pos.length();
            let time_to_target_pos = distance_to_target / travel_speed;
            target_pos + (time_to_target_pos * target_vel)
        }

        #[inline]
        pub fn intercept_target(
            current_pos: Vector3,
            travel_speed: Real,
            target_pos: Vector3,
            target_vel: Vector3,
        ) -> Vector3 {
            seek_position(
                current_pos,
                find_intercept_pos(current_pos, travel_speed, target_pos, target_vel),
            )
        }
    }
}

//pub type LinearRoutineFn = dyn FnMut(GlobalTransform, LinearEngineState, EngineConfig) -> Vector3;
