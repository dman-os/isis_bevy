use deps::*;

use bevy::{prelude::*, utils::HashMap};
use bevy_rapier3d::prelude::*;

use crate::math::*;

use super::{FlockChangeEvent, FlockChangeEvents, FlockChangeEventsReader, FlockMembers};

#[derive(Bundle)]
pub struct FlockFormationBundle {
    pub pattern: FormationPattern,
    pub center_pivot: FormationCenterPivot,
    pub slotting_strategy: SlottingStrategy,
    pub flock_change_reader: FlockChangeEventsReader,
    pub slots: FormationSlots,
    pub state: FormationState,
    pub output: FormationOutputs,
    pub name: Name,
    pub tag: FlockFormation,
    pub parent: Parent,
}

impl FlockFormationBundle {
    pub const DEFAULT_NAME: &'static str = "flock_formation";
    pub fn new(
        pattern: FormationPattern,
        center_pivot: Entity,
        slotting_strategy: SlottingStrategy,
        flock_entt: Entity,
    ) -> Self {
        Self {
            pattern,
            center_pivot: FormationCenterPivot {
                boid_entt: center_pivot,
            },
            slotting_strategy,
            slots: Default::default(),
            state: Default::default(),
            output: Default::default(),
            tag: FlockFormation::new(flock_entt),
            parent: Parent(flock_entt),
            name: Self::DEFAULT_NAME.into(),
            flock_change_reader: FlockChangeEventsReader::default(),
        }
    }
}

#[derive(Debug, Component)]
#[component(storage = "SparseSet")]
pub struct ActiveFlockFormation;

#[derive(Debug, Clone, Copy, Component)]
pub struct FlockFormation {
    flock_entt: Entity,
}

impl FlockFormation {
    pub fn new(flock_entt: Entity) -> Self {
        Self { flock_entt }
    }

    pub fn flock_entt(&self) -> Entity {
        self.flock_entt
    }
}

#[derive(Debug, Clone, Component)]
pub struct FormationCenterPivot {
    boid_entt: Entity,
}

impl FormationCenterPivot {
    #[inline]
    pub fn boid_entt(&self) -> Entity {
        self.boid_entt
    }
}

#[derive(Debug, Clone, Component)]
pub enum FormationPattern {
    Sphere { radius: TReal },
}

#[derive(Debug, Clone, Component)]
pub enum SlottingStrategy {
    Simple,
}

#[derive(Debug, Clone)]
pub enum FormationSlotKind {
    Boid,
    Anchor,
}

#[derive(Debug, Clone)]
pub struct FormationSlotDesc {
    pub kind: FormationSlotKind,
}

#[derive(Debug, Default, Component)]
pub struct FormationSlots {
    slots: HashMap<Entity, FormationSlotDesc>,
    added: smallvec::SmallVec<[Entity; 4]>,
    removed: smallvec::SmallVec<[(Entity, FormationSlotDesc); 2]>,
}

impl FormationSlots {
    #[inline]
    pub fn insert(&mut self, entt: Entity, kind: FormationSlotKind) -> Option<FormationSlotDesc> {
        self.added.push(entt);
        self.slots.insert(entt, FormationSlotDesc { kind })
    }

    #[inline]
    pub fn get(&self, entt: Entity) -> Option<&FormationSlotDesc> {
        self.slots.get(&entt)
    }

    #[inline]
    pub fn remove(&mut self, entt: Entity) -> Option<FormationSlotDesc> {
        match self.slots.remove(&entt) {
            Some(desc) => {
                self.removed.push((entt, desc.clone()));
                Some(desc)
            }
            None => None,
        }
    }

    #[inline]
    fn added(&mut self) -> smallvec::Drain<[Entity; 4]> {
        self.added.drain(0..self.added.len())
    }

    #[inline]
    fn removed(&mut self) -> smallvec::Drain<[(Entity, FormationSlotDesc); 2]> {
        self.removed.drain(0..self.removed.len())
    }
}

#[derive(Debug, Component, Default)]
pub struct FormationState {
    pub boid_strategies: HashMap<Entity, Entity>,
    pub shadow_leader_anchor: Option<Entity>,
}

#[derive(Debug, Default)]
pub struct FormationOutput {
    pub pos: TVec3,
    pub facing: TVec3,
    pub speed: TReal,
}

#[derive(Debug, Default, Component)]
pub struct FormationOutputs {
    pub index: HashMap<Entity, FormationOutput>,
}

// TODO: formation constraints
// TODO: formation dissolution
pub fn butler(
    mut commands: Commands,
    mut formations: QuerySet<(
        // new
        QueryState<
            (
                Entity,
                &FlockFormation,
                &FormationCenterPivot,
                &mut FlockChangeEventsReader,
                &mut FormationState,
                &mut FormationOutputs,
                &SlottingStrategy,
                &mut FormationSlots,
            ),
            Added<FlockFormation>,
        >,
        // changed SlottingStrategy
        QueryState<
            (&FlockFormation, &SlottingStrategy, &mut FormationSlots),
            Changed<SlottingStrategy>,
        >,
        // changed slots
        QueryState<
            (
                &SlottingStrategy,
                &mut FormationSlots,
                &mut FormationOutputs,
                &mut FormationState,
            ),
            Changed<FormationSlots>,
        >,
        // all
        QueryState<(
            &FlockFormation,
            &mut FlockChangeEventsReader,
            &SlottingStrategy,
            &mut FormationSlots,
            &mut FormationOutputs,
            &mut FormationState,
        )>,
    )>,
    flocks: Query<(&FlockMembers, &FlockChangeEvents)>,
    formants: Query<(&GlobalTransform /* Option<&mut Formant> */,)>,
) {
    // serve new formatins
    // TODO: consider slotting strategy
    for (
        entt,
        formation,
        center_pivot,
        mut reader,
        mut state,
        mut output,
        _slotting_strategy,
        mut slots,
    ) in formations.q0().iter_mut()
    {
        let (members, events) = flocks.get(formation.flock_entt()).unwrap_or_log();
        // we're not interested in any events before activation
        let _ = reader.iter(events).skip_while(|_| true);
        state.shadow_leader_anchor = Some(
            commands
                .spawn()
                .insert_bundle(FormationAnchorBundle::new(
                    FormationAnchorDirectives::Shadow {
                        boid: center_pivot.boid_entt,
                    },
                    entt,
                ))
                .id(),
        );
        for member in members.iter().filter(|e| **e != center_pivot.boid_entt) {
            let (xform,) = formants.get(*member).unwrap_or_log();
            output.index.insert(
                *member,
                FormationOutput {
                    pos: xform.translation,
                    ..Default::default()
                },
            );
            slots.slots.insert(
                *member,
                FormationSlotDesc {
                    kind: FormationSlotKind::Boid,
                },
            );
        }
        commands.entity(entt).insert(ActiveFlockFormation);
    }
    // TODO: serve slotting strategy changes
    /* for (formation, slotting_strategy, mut slots) in formations.q1().iter_mut() {
        let (members, ..) = flocks.get(formation.flock_entt()).unwrap_or_log();
        match slotting_strategy {
            SlottingStrategy::Simple => {
                // slots.slots = slots.slots.into_iter().map(|(e, d)| (e, d)).collect()
                todo!();
            }
        }
    } */
    // serve slot changes
    // TODO: consider slotting strategy
    for (slotting_strategy, mut slots, mut output, mut state) in formations.q2().iter_mut() {
        // skip early to avoid reslotting
        if slots.added.is_empty() && slots.removed.is_empty() {
            continue;
        }
        for (removed, _) in slots.removed() {
            output.index.remove(&removed);
            state.boid_strategies.remove(&removed);
        }
        for added in slots.added() {
            let (xform,) = formants.get(added).unwrap_or_log();
            output.index.insert(
                added,
                FormationOutput {
                    pos: xform.translation,
                    ..Default::default()
                },
            );
        }
        match slotting_strategy {
            SlottingStrategy::Simple => todo!(),
        }
    }

    // serve new foramnts
    // TODO: leader removal replacement
    // TODO: consider slotting strategy
    for (formation, mut reader, _slotting_strategy, mut slots, mut output, mut state) in
        formations.q3().iter_mut()
    {
        let (_, events) = flocks.get(formation.flock_entt()).unwrap_or_log();
        // we're not interested in any events before the formation's creation
        for event in reader.iter(events) {
            match event {
                FlockChangeEvent::MemberAdded { entt } => {
                    let entt = *entt;
                    let (xform,) = formants.get(entt).unwrap_or_log();
                    // NOTE: avoid triggering change detection by directly accessing the slots
                    slots.slots.insert(
                        entt,
                        FormationSlotDesc {
                            kind: FormationSlotKind::Boid,
                        },
                    );
                    output.index.insert(
                        entt,
                        FormationOutput {
                            pos: xform.translation,
                            ..Default::default()
                        },
                    );
                }
                FlockChangeEvent::MemberRemoved { entt } => {
                    let entt = *entt;
                    // NOTE: avoid triggering change detection by directly accessing the slots
                    slots.slots.remove(&entt);
                    output.index.remove(&entt);
                    state.boid_strategies.remove(&entt);
                }
            }
        }
    }
}

pub fn update(
    mut formations: Query<
        (
            &FormationPattern,
            &FormationSlots,
            &FormationCenterPivot,
            &mut FormationState,
            &mut FormationOutputs,
        ),
        With<ActiveFlockFormation>,
    >,
    anchors: Query<(&FormationAnchorState,)>,
    // flocks: Query<&FlockMembers>,
    // crafts: Query<&GlobalTransform>,
) {
    for (pattern, slots, center_pivot, state, mut out) in formations.iter_mut() {
        let member_size = if slots.slots.contains_key(&center_pivot.boid_entt) {
            slots.slots.len() - 1
        } else {
            slots.slots.len()
        };
        if member_size == 0 {
            continue;
        }
        let (anchor_state,) = anchors
            .get(state.shadow_leader_anchor.unwrap_or_log())
            .unwrap_or_log();
        let facing = anchor_state.rot * -TVec3::Z;
        let speed = anchor_state.linvel.length();
        match &pattern {
            FormationPattern::Sphere { radius } => {
                // TODO: LRU cache the rays
                let rays = crate::utils::points_on_sphere(member_size);
                for (ii, (boid_entt, _)) in slots
                    .slots
                    .iter()
                    .filter(|(e, _)| **e != center_pivot.boid_entt)
                    .enumerate()
                {
                    out.index.insert(
                        *boid_entt,
                        FormationOutput {
                            pos: anchor_state.pos + (rays[ii] * *radius),
                            facing,
                            speed,
                        },
                    );
                }
            }
        }
    }
}
/*
#[derive(Debug, Clone, Component)]
pub enum FormationPivot {
    Craft { entt: Entity },
    Anchor { xform: Transform },
} */

#[derive(Bundle)]
pub struct FormationAnchorBundle {
    pub state: FormationAnchorState,
    pub directive: FormationAnchorDirectives,
    pub name: Name,
    pub tag: FormationAnchor,
    pub parent: Parent,
}

impl FormationAnchorBundle {
    pub const DEFAULT_NAME: &'static str = "flock_anchor";
    pub fn new(directive: FormationAnchorDirectives, formation_entt: Entity) -> Self {
        Self {
            directive,
            tag: FormationAnchor { formation_entt },
            parent: Parent(formation_entt),
            name: Self::DEFAULT_NAME.into(),
            state: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Component)]
pub struct FormationAnchor {
    formation_entt: Entity,
}

#[derive(Debug, Clone, Default, Component)]
pub struct FormationAnchorState {
    pos: TVec3,
    rot: TQuat,
    linvel: TVec3,
}

#[derive(Debug, Clone, Component)]
pub enum FormationAnchorDirectives {
    Shadow { boid: Entity },
    // Form { formation: Entity },
}

pub fn formation_anchor_motion(
    mut anchors: Query<(&mut FormationAnchorState, &FormationAnchorDirectives)>,
    boids: Query<(&GlobalTransform, &RigidBodyVelocityComponent)>,
) {
    for (mut state, directive) in anchors.iter_mut() {
        match directive {
            FormationAnchorDirectives::Shadow { boid } => {
                let (target_xform, vel) = boids.get(*boid).unwrap_or_log();
                state.pos = target_xform.translation;
                state.rot = target_xform.rotation;
                state.linvel = vel.linvel.into();
            }
        }
    }
}

/* #[derive(Debug, Clone, Component)]
#[component(storage = "SparseSet")]
pub struct Formant {
    pub formation: Entity,
} */

/*

/// A generic bundle for flock strategies.
#[derive(Bundle)]
pub struct FlockFormationBundle<P>
where
    P: Component,
{
    pub param: P,
    pub tag: FlockFormation,
}

impl<P> FlockFormationBundle<P>
where
    P: Component,
{
    pub fn new(param: P, flock_entt: Entity) -> Self {
        Self {
            param,
            tag: FlockFormation::new(flock_entt, FlockFormationKind::of::<P>()),
        }
    }
}

/// A variant of [`FlockFormationBundle`] with two parameter components.
#[derive(Bundle)]
pub struct FlockFormationBundleExtra<P, P2>
where
    P: Component,
    P2: Component,
{
    pub param: P,
    pub extra: P2,
    pub tag: FlockFormation,
}

impl<P, P2> FlockFormationBundleExtra<P, P2>
where
    P: Component,
    P2: Component,
{
    pub fn new(param: P, flock_entt: Entity, extra: P2) -> Self {
        Self {
            param,
            extra,
            tag: FlockFormation::new(flock_entt, FlockFormationKind::of::<P>()),
        }
    }
}

/// A variant of [`FlockFormationBundleExtra`] where the second component is also a bundle.
#[derive(Bundle)]
pub struct FlockFormationBundleJumbo<P, B>
where
    P: Component,
    B: Bundle,
{
    pub param: P,
    #[bundle]
    pub extra: B,
    pub tag: FlockFormation,
}

impl<P, B> FlockFormationBundleJumbo<P, B>
where
    P: Component,
    B: Bundle,
{
    pub fn new(param: P, flock_entt: Entity, extra: B) -> Self {
        Self {
            param,
            extra,
            tag: FlockFormation::new(flock_entt, FlockFormationKind::of::<P>()),
        }
    }
}

pub type FlockFormationKind = std::any::TypeId; */
