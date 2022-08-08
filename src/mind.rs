use deps::*;

use bevy::prelude::*;
use bevy_inspector_egui::RegisterInspectable;

pub mod boid;
pub mod flock;
pub mod guy;
pub mod player;
pub mod sensors;

/* pub mod tribe {
    use deps::*;
}  */
/*
pub mod master {}
*/

pub struct MindPlugin;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, SystemLabel)]
pub enum CraftMindSystems {
    CraftBoidStrategyOutputMgr,
    SteeringRoutineButler,
    SteeringRoutine,
    ComposeRoutineUpdate,
    BoidStrategyButler,
    BoidStrategy,
    FlockStrategyButler,
    FlockStrategy,
    ComposeButler,
    FlockChangeListener,
    FormationUpdate,
}

impl Plugin for MindPlugin {
    fn build(&self, app: &mut App) {
        use CraftMindSystems::*;
        app.init_resource::<sensors::CraftWeaponCrossRefIndex>()
            .add_system_to_stage(
                CoreStage::PreUpdate,
                sensors::craft_wpn_index_butler.before(BoidStrategyButler),
            )
            .init_resource::<sensors::BoidStrategyCrossRefIndex>()
            .add_system_to_stage(
                CoreStage::PreUpdate,
                sensors::craft_strategy_index_butler.before(BoidStrategyButler),
            )
            .init_resource::<sensors::SteeringRoutineCrossRefIndex>()
            .add_system_to_stage(
                CoreStage::PreUpdate,
                sensors::craft_routine_index_butler.after(ComposeButler),
            )
            // flock formation systems
            .add_system_to_stage(
                CoreStage::PreUpdate,
                flock::flock_members_change_listener.label(FlockChangeListener),
            )
            .add_system_to_stage(
                CoreStage::PreUpdate,
                flock::formation::butler.after(FlockChangeListener),
            )
            .add_system(flock::formation::formation_anchor_motion.before(FormationUpdate))
            .add_system(flock::formation::update.label(FormationUpdate))
            // flock strategy systems
            .add_system_set_to_stage(
                // FIXME: we need command flushing between flock strategy butlers and boid strategy butlers
                CoreStage::PreUpdate,
                SystemSet::new()
                    .label(FlockStrategyButler)
                    .after(FlockChangeListener)
                    .with_system(flock::strategy::form_up::butler)
                    .with_system(flock::strategy::cas::butler),
            )
            .add_system_set(
                SystemSet::new()
                    .label(FlockStrategy)
                    .with_system(flock::strategy::cas::update),
            )
            // boid strategy systems
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::new()
                    .label(BoidStrategyButler)
                    .after(FlockChangeListener)
                    .with_system(boid::strategy::attack_persue::butler)
                    .with_system(boid::strategy::run_circuit::butler)
                    .with_system(boid::strategy::form::butler)
                    .with_system(boid::strategy::custom::butler),
            )
            .add_system_set(
                SystemSet::new()
                    .label(BoidStrategy)
                    .with_system(boid::strategy::attack_persue::update)
                    .with_system(boid::strategy::form::update)
                    .with_system(boid::strategy::run_circuit::update),
            )
            .add_system(
                boid::strategy::craft_boid_strategy_output_mgr
                    .label(CraftBoidStrategyOutputMgr)
                    .after(BoidStrategy),
            )
            // boid steering systems
            .add_system_to_stage(
                CoreStage::PreUpdate,
                boid::steering::compose::butler
                    .label(ComposeButler)
                    .before(SteeringRoutineButler),
            )
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::new()
                    .label(SteeringRoutineButler)
                    .with_system(boid::steering::avoid_collision::butler),
            )
            .add_system_set(
                SystemSet::new()
                    .label(SteeringRoutine)
                    .with_system(boid::steering::intercept::update)
                    .with_system(boid::steering::fly_with_flock::update)
                    .with_system(boid::steering::avoid_collision::update)
                    .with_system(boid::steering::arrive::update)
                    .with_system(boid::steering::player::update)
                    .with_system(boid::steering::face::update)
                    .with_system(boid::steering::closure::update)
                    .with_system(boid::steering::seek::update),
            )
            .add_system(
                boid::steering::compose::update
                    .label(ComposeRoutineUpdate)
                    .after(SteeringRoutine),
            )
            .add_system(boid::steering::steering_output_to_engine.after(ComposeRoutineUpdate))
            // player systems
            .add_system_to_stage(CoreStage::PostUpdate, player::wpn_raycaster_butler)
            .add_system(player::cam_input)
            .add_system(player::engine_input)
            .add_system(player::wpn_input)
            .add_startup_system(player::setup_markers)
            .add_system(player::update_ui_markers)
            .insert_resource(player::PlayerMindConfig::default())
            .insert_resource(player::PlayerBoidInput::default())
            .insert_resource(player::CurrentCraft::default())
            .add_plugin(bevy_inspector_egui::InspectorPlugin::<
                player::PlayerEngineConfig,
            >::new())
            // minds
            .add_system_to_stage(CoreStage::PreUpdate, boid::boid_mind)
            .add_system_to_stage(CoreStage::PreUpdate, flock::flock_mind)
            .add_system_to_stage(CoreStage::PreUpdate, player::player_mind)
            // types
            .register_inspectable::<boid::strategy::CurrentBoidStrategy>()
            .register_inspectable::<flock::strategy::CurrentFlockStrategy>()
            .register_inspectable::<flock::CurrentFlockFormation>()
            .register_inspectable::<boid::steering::CurrentSteeringRoutine>()
            .register_inspectable::<player::CraftCamera>()
            .register_inspectable::<flock::strategy::cas::CASState>()
            .register_inspectable::<boid::BoidMindConfig>()
            .register_inspectable::<boid::steering::LinearRoutineOutput>()
            .register_inspectable::<boid::steering::AngularRoutineOutput>();
    }
}
/* #[derive(Debug, Clone, Component)]
pub enum ScanPresence {
    Obstacle,
    Boid,
}
 */
