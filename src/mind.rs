use deps::*;

use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_inspector_egui::RegisterInspectable;

pub mod boid;
pub mod flock;
pub mod player;
pub mod sensors;

pub struct MindPlugin;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, SystemLabel)]
pub enum CraftMindSystems {
    CraftBoidStrategyOutputMgr,
    SteeringSystems,
    RoutineComposer,
    BoidStrategyButler,
    BoidStrategy,
    FlockStrategyButler,
    FlockStrategy,
    ActiveRoutineTagger,
}

impl Plugin for MindPlugin {
    fn build(&self, app: &mut App) {
        use CraftMindSystems::*;
        app.add_system_to_stage(CoreStage::PostUpdate, player::wpn_raycaster_butler)
            .init_resource::<sensors::CraftWeaponCrossRefIndex>()
            .add_system_to_stage(
                CoreStage::PreUpdate,
                sensors::craft_wpn_index_butler.before(BoidStrategyButler),
            )
            .init_resource::<sensors::BoidStrategyCrossRefIndex>()
            .add_system_to_stage(
                CoreStage::PreUpdate,
                sensors::craft_strategy_index_butler.before(BoidStrategyButler),
            )
            .add_system_set_to_stage(
                // FIXME: we need command flushing between flock strategy butlers and boid strategy butlers
                CoreStage::PreUpdate,
                SystemSet::new()
                    .label(FlockStrategyButler)
                    .with_system(flock::strategy::hold::butler)
                    .with_system(flock::strategy::cas::butler),
            )
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::new()
                    .label(BoidStrategyButler)
                    .with_system(boid::strategy::attack_persue::butler)
                    .with_system(boid::strategy::run_circuit::butler)
                    .with_system(boid::strategy::form::butler)
                    .with_system(boid::strategy::custom::butler),
            )
            .add_system_to_stage(CoreStage::PreUpdate, flock::formation::butler)
            .add_system_set(
                SystemSet::new()
                    .label(BoidStrategy)
                    .with_system(boid::strategy::attack_persue::update)
                    .with_system(boid::strategy::form::update)
                    .with_system(boid::strategy::run_circuit::update),
            )
            .add_system_set(
                SystemSet::new()
                    .label(FlockStrategy)
                    .with_system(flock::strategy::cas::update),
            )
            .add_system(
                boid::strategy::craft_boid_strategy_output_mgr
                    .label(CraftBoidStrategyOutputMgr)
                    .after(BoidStrategy),
            )
            .add_system(
                boid::steering::active_routine_tagger
                    .label(ActiveRoutineTagger)
                    .after(CraftBoidStrategyOutputMgr)
                    .before(SteeringSystems),
            )
            .init_resource::<sensors::SteeringRoutineCrossRefIndex>()
            .add_system(sensors::craft_routine_index_butler.after(ActiveRoutineTagger))
            .add_system(sensors::craft_routine_index_butler)
            .add_system(flock::formation::update)
            .add_system_set(
                SystemSet::new()
                    .label(SteeringSystems)
                    .with_system(boid::steering::intercept::update)
                    .with_system(boid::steering::fly_with_flock::update)
                    .with_system(boid::steering::avoid_collision::update)
                    .with_system(boid::steering::arrive::update)
                    .with_system(boid::steering::player::update)
                    .with_system(boid::steering::seek::update),
            )
            .add_system(
                boid::steering::routine_composer
                    .label(RoutineComposer)
                    .after(SteeringSystems),
            )
            .add_system(boid::steering::mind_update_engine_input.after(RoutineComposer))
            .add_system(player::cam_input)
            .add_system(player::engine_input)
            .add_system(player::wpn_input)
            .add_startup_system(player::setup_markers)
            .add_system(player::update_ui_markers)
            .add_system_to_stage(CoreStage::PreUpdate, boid::boid_mind)
            .add_system_to_stage(CoreStage::PreUpdate, flock::flock_mind)
            .insert_resource(player::PlayerMindConfig::default())
            .insert_resource(player::PlayerBoidInput::default())
            .add_system_to_stage(CoreStage::PreUpdate, player::player_mind)
            .register_inspectable::<player::CraftCamera>()
            .register_inspectable::<flock::strategy::cas::CASState>()
            .register_inspectable::<boid::BoidMindConfig>()
            .register_inspectable::<boid::steering::BoidSteeringSystemOutput>()
            .register_inspectable::<boid::steering::LinearRoutineOutput>()
            .register_inspectable::<boid::steering::AngularRoutineOutput>()
            .register_inspectable::<boid::steering::avoid_collision::AvoidCollision>();
    }
}
/* #[derive(Debug, Clone, Component)]
pub enum ScanPresence {
    Obstacle,
    Boid,
}
 */

/*
use master::*;
mod master {} */
