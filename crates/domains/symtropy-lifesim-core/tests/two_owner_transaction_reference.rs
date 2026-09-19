use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PatchId(u32);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Revision(u64);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Generation(u64);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct EffectId(u64);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct TransactionId(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GroundPatch {
    revision: Revision,
    disturbance: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct VegetationPatch {
    revision: Revision,
    membership_generation: Generation,
    spatial_generation: Generation,
    present: bool,
    damage: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GroundOwner {
    patches: BTreeMap<PatchId, GroundPatch>,
    receipts: BTreeMap<EffectId, GroundReceipt>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct VegetationOwner {
    patches: BTreeMap<PatchId, VegetationPatch>,
    receipts: BTreeMap<EffectId, VegetationReceipt>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct World {
    ground: GroundOwner,
    vegetation: VegetationOwner,
    transactions: BTreeMap<TransactionId, TransactionReceipt>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum DependencyKey {
    GroundPatchRevision(PatchId),
    VegetationPatchRevision(PatchId),
    VegetationMembership(PatchId),
    VegetationSpatialBinding(PatchId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DependencySet(BTreeMap<DependencyKey, u64>);

impl DependencySet {
    fn new(entries: impl IntoIterator<Item = (DependencyKey, u64)>) -> Result<Self, TxError> {
        let mut out = BTreeMap::new();
        for (key, value) in entries {
            if let Some(existing) = out.insert(key, value) {
                if existing != value {
                    return Err(TxError::ConflictingDependency(key));
                }
            }
        }
        Ok(Self(out))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NegativeVegetationObservation {
    patch: PatchId,
    membership_generation: Generation,
    spatial_generation: Generation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VegetationRequirement {
    Required,
    OptionalWithExplicitAbsence,
    Prohibited,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TransactionProfile {
    vegetation: VegetationRequirement,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PreparedGround {
    effect_id: EffectId,
    patch: PatchId,
    before: GroundPatch,
    after: GroundPatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PreparedVegetation {
    Applied {
        effect_id: EffectId,
        patch: PatchId,
        before: VegetationPatch,
        after: VegetationPatch,
    },
    NoEffect {
        patch: PatchId,
        revision: Revision,
    },
    NoSubject(NegativeVegetationObservation),
    Prohibited,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum DomainKey {
    Ground,
    Vegetation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ChildIdentity {
    GroundApplied {
        effect_id: EffectId,
        patch: PatchId,
        before: Revision,
        after: Revision,
        disturbance: u32,
    },
    VegetationApplied {
        effect_id: EffectId,
        patch: PatchId,
        before: Revision,
        after: Revision,
        damage: u32,
    },
    VegetationNoEffect {
        patch: PatchId,
        revision: Revision,
    },
    VegetationNoSubject(NegativeVegetationObservation),
    VegetationProhibited,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TransactionIdentity {
    id: TransactionId,
    patch: PatchId,
    profile: TransactionProfile,
    children: BTreeMap<DomainKey, ChildIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PreparedTransaction {
    identity: TransactionIdentity,
    ground: PreparedGround,
    vegetation: PreparedVegetation,
    dependencies: DependencySet,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GroundReceipt {
    transaction_id: TransactionId,
    effect_id: EffectId,
    patch: PatchId,
    before: Revision,
    after: Revision,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct VegetationReceipt {
    transaction_id: TransactionId,
    effect_id: EffectId,
    patch: PatchId,
    before: Revision,
    after: Revision,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TransactionReceipt {
    identity: TransactionIdentity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TxError {
    UnknownPatch(PatchId),
    RequiredVegetationMissing(PatchId),
    ProhibitedVegetationChild(PatchId),
    StaleDependency(DependencyKey),
    ConflictingDependency(DependencyKey),
    ArithmeticOverflow,
    TransactionConflict(TransactionId),
    PartialHistoryDetected(TransactionId),
}

impl GroundOwner {
    fn prepare(
        &self,
        patch: PatchId,
        effect_id: EffectId,
        delta: u32,
    ) -> Result<PreparedGround, TxError> {
        let before = *self
            .patches
            .get(&patch)
            .ok_or(TxError::UnknownPatch(patch))?;
        let disturbance = before
            .disturbance
            .checked_add(delta)
            .ok_or(TxError::ArithmeticOverflow)?;
        let revision = before
            .revision
            .0
            .checked_add(1)
            .map(Revision)
            .ok_or(TxError::ArithmeticOverflow)?;
        Ok(PreparedGround {
            effect_id,
            patch,
            before,
            after: GroundPatch {
                revision,
                disturbance,
            },
        })
    }
}

impl VegetationOwner {
    fn prepare(
        &self,
        patch: PatchId,
        effect_id: EffectId,
        damage_delta: u32,
        requirement: VegetationRequirement,
    ) -> Result<PreparedVegetation, TxError> {
        let before = *self
            .patches
            .get(&patch)
            .ok_or(TxError::UnknownPatch(patch))?;
        match requirement {
            VegetationRequirement::Prohibited => {
                if before.present {
                    Err(TxError::ProhibitedVegetationChild(patch))
                } else {
                    Ok(PreparedVegetation::Prohibited)
                }
            }
            VegetationRequirement::Required => {
                if !before.present {
                    return Err(TxError::RequiredVegetationMissing(patch));
                }
                prepare_vegetation_damage(effect_id, patch, before, damage_delta)
            }
            VegetationRequirement::OptionalWithExplicitAbsence => {
                if !before.present {
                    return Ok(PreparedVegetation::NoSubject(
                        NegativeVegetationObservation {
                            patch,
                            membership_generation: before.membership_generation,
                            spatial_generation: before.spatial_generation,
                        },
                    ));
                }
                if damage_delta == 0 {
                    Ok(PreparedVegetation::NoEffect {
                        patch,
                        revision: before.revision,
                    })
                } else {
                    prepare_vegetation_damage(effect_id, patch, before, damage_delta)
                }
            }
        }
    }
}

fn prepare_vegetation_damage(
    effect_id: EffectId,
    patch: PatchId,
    before: VegetationPatch,
    damage_delta: u32,
) -> Result<PreparedVegetation, TxError> {
    let damage = before
        .damage
        .checked_add(damage_delta)
        .ok_or(TxError::ArithmeticOverflow)?;
    let revision = before
        .revision
        .0
        .checked_add(1)
        .map(Revision)
        .ok_or(TxError::ArithmeticOverflow)?;
    Ok(PreparedVegetation::Applied {
        effect_id,
        patch,
        before,
        after: VegetationPatch {
            revision,
            damage,
            ..before
        },
    })
}

fn ground_identity(child: PreparedGround) -> ChildIdentity {
    ChildIdentity::GroundApplied {
        effect_id: child.effect_id,
        patch: child.patch,
        before: child.before.revision,
        after: child.after.revision,
        disturbance: child.after.disturbance,
    }
}

fn vegetation_identity(child: PreparedVegetation) -> ChildIdentity {
    match child {
        PreparedVegetation::Applied {
            effect_id,
            patch,
            before,
            after,
        } => ChildIdentity::VegetationApplied {
            effect_id,
            patch,
            before: before.revision,
            after: after.revision,
            damage: after.damage,
        },
        PreparedVegetation::NoEffect { patch, revision } => {
            ChildIdentity::VegetationNoEffect { patch, revision }
        }
        PreparedVegetation::NoSubject(obs) => ChildIdentity::VegetationNoSubject(obs),
        PreparedVegetation::Prohibited => ChildIdentity::VegetationProhibited,
    }
}

fn canonical_child_set(
    children: impl IntoIterator<Item = (DomainKey, ChildIdentity)>,
) -> Result<BTreeMap<DomainKey, ChildIdentity>, TxError> {
    let mut out = BTreeMap::new();
    for (domain, child) in children {
        if out.insert(domain, child).is_some() {
            return Err(TxError::TransactionConflict(TransactionId(u64::MAX)));
        }
    }
    Ok(out)
}

fn prepare_transaction(
    world: &World,
    id: TransactionId,
    patch: PatchId,
    profile: TransactionProfile,
    ground_effect: EffectId,
    ground_delta: u32,
    vegetation_effect: EffectId,
    vegetation_damage: u32,
) -> Result<PreparedTransaction, TxError> {
    let ground = world.ground.prepare(patch, ground_effect, ground_delta)?;
    let vegetation = world.vegetation.prepare(
        patch,
        vegetation_effect,
        vegetation_damage,
        profile.vegetation,
    )?;

    let mut dependencies = vec![(
        DependencyKey::GroundPatchRevision(patch),
        ground.before.revision.0,
    )];
    match vegetation {
        PreparedVegetation::Applied { before, .. } => {
            dependencies.push((
                DependencyKey::VegetationPatchRevision(patch),
                before.revision.0,
            ));
            dependencies.push((
                DependencyKey::VegetationSpatialBinding(patch),
                before.spatial_generation.0,
            ));
        }
        PreparedVegetation::NoEffect { revision, .. } => {
            dependencies.push((
                DependencyKey::VegetationPatchRevision(patch),
                revision.0,
            ));
        }
        PreparedVegetation::NoSubject(obs) => {
            dependencies.push((
                DependencyKey::VegetationMembership(patch),
                obs.membership_generation.0,
            ));
            dependencies.push((
                DependencyKey::VegetationSpatialBinding(patch),
                obs.spatial_generation.0,
            ));
        }
        PreparedVegetation::Prohibited => {}
    }

    let children = canonical_child_set([
        (DomainKey::Ground, ground_identity(ground)),
        (DomainKey::Vegetation, vegetation_identity(vegetation)),
    ])?;

    Ok(PreparedTransaction {
        identity: TransactionIdentity {
            id,
            patch,
            profile,
            children,
        },
        ground,
        vegetation,
        dependencies: DependencySet::new(dependencies)?,
    })
}

impl World {
    fn dependency_value(&self, key: DependencyKey) -> Result<u64, TxError> {
        match key {
            DependencyKey::GroundPatchRevision(patch) => self
                .ground
                .patches
                .get(&patch)
                .map(|state| state.revision.0)
                .ok_or(TxError::UnknownPatch(patch)),
            DependencyKey::VegetationPatchRevision(patch) => self
                .vegetation
                .patches
                .get(&patch)
                .map(|state| state.revision.0)
                .ok_or(TxError::UnknownPatch(patch)),
            DependencyKey::VegetationMembership(patch) => self
                .vegetation
                .patches
                .get(&patch)
                .map(|state| state.membership_generation.0)
                .ok_or(TxError::UnknownPatch(patch)),
            DependencyKey::VegetationSpatialBinding(patch) => self
                .vegetation
                .patches
                .get(&patch)
                .map(|state| state.spatial_generation.0)
                .ok_or(TxError::UnknownPatch(patch)),
        }
    }

    fn commit(&mut self, prepared: &PreparedTransaction) -> Result<TransactionReceipt, TxError> {
        if let Some(receipt) = self.transactions.get(&prepared.identity.id) {
            return if receipt.identity == prepared.identity {
                Ok(receipt.clone())
            } else {
                Err(TxError::TransactionConflict(prepared.identity.id))
            };
        }

        if self
            .ground
            .receipts
            .values()
            .any(|receipt| receipt.transaction_id == prepared.identity.id)
            || self
                .vegetation
                .receipts
                .values()
                .any(|receipt| receipt.transaction_id == prepared.identity.id)
        {
            return Err(TxError::PartialHistoryDetected(prepared.identity.id));
        }

        for (&key, &expected) in &prepared.dependencies.0 {
            if self.dependency_value(key)? != expected {
                return Err(TxError::StaleDependency(key));
            }
        }

        let mut next_ground = self.ground.clone();
        let mut next_vegetation = self.vegetation.clone();

        let ground = prepared.ground;
        next_ground.patches.insert(ground.patch, ground.after);
        next_ground.receipts.insert(
            ground.effect_id,
            GroundReceipt {
                transaction_id: prepared.identity.id,
                effect_id: ground.effect_id,
                patch: ground.patch,
                before: ground.before.revision,
                after: ground.after.revision,
            },
        );

        match prepared.vegetation {
            PreparedVegetation::Applied {
                effect_id,
                patch,
                before,
                after,
            } => {
                next_vegetation.patches.insert(patch, after);
                next_vegetation.receipts.insert(
                    effect_id,
                    VegetationReceipt {
                        transaction_id: prepared.identity.id,
                        effect_id,
                        patch,
                        before: before.revision,
                        after: after.revision,
                    },
                );
            }
            PreparedVegetation::NoEffect { .. }
            | PreparedVegetation::NoSubject(_)
            | PreparedVegetation::Prohibited => {}
        }

        let receipt = TransactionReceipt {
            identity: prepared.identity.clone(),
        };
        self.ground = next_ground;
        self.vegetation = next_vegetation;
        self.transactions.insert(prepared.identity.id, receipt.clone());
        Ok(receipt)
    }
}

fn fixture_world() -> World {
    World {
        ground: GroundOwner {
            patches: BTreeMap::from([
                (
                    PatchId(1),
                    GroundPatch {
                        revision: Revision(10),
                        disturbance: 3,
                    },
                ),
                (
                    PatchId(2),
                    GroundPatch {
                        revision: Revision(20),
                        disturbance: 7,
                    },
                ),
            ]),
            receipts: BTreeMap::new(),
        },
        vegetation: VegetationOwner {
            patches: BTreeMap::from([
                (
                    PatchId(1),
                    VegetationPatch {
                        revision: Revision(30),
                        membership_generation: Generation(40),
                        spatial_generation: Generation(50),
                        present: true,
                        damage: 2,
                    },
                ),
                (
                    PatchId(2),
                    VegetationPatch {
                        revision: Revision(31),
                        membership_generation: Generation(41),
                        spatial_generation: Generation(51),
                        present: true,
                        damage: 4,
                    },
                ),
            ]),
            receipts: BTreeMap::new(),
        },
        transactions: BTreeMap::new(),
    }
}

fn profile(vegetation: VegetationRequirement) -> TransactionProfile {
    TransactionProfile { vegetation }
}

#[test]
fn required_children_publish_together() {
    let mut world = fixture_world();
    let prepared = prepare_transaction(
        &world,
        TransactionId(1),
        PatchId(1),
        profile(VegetationRequirement::Required),
        EffectId(10),
        5,
        EffectId(11),
        6,
    )
    .unwrap();
    world.commit(&prepared).unwrap();
    assert_eq!(world.ground.patches[&PatchId(1)].disturbance, 8);
    assert_eq!(world.vegetation.patches[&PatchId(1)].damage, 8);
}

#[test]
fn stale_vegetation_rejects_without_ground_publication() {
    let mut world = fixture_world();
    let prepared = prepare_transaction(
        &world,
        TransactionId(2),
        PatchId(1),
        profile(VegetationRequirement::Required),
        EffectId(20),
        5,
        EffectId(21),
        6,
    )
    .unwrap();
    let ground_before = world.ground.clone();
    world.vegetation.patches.get_mut(&PatchId(1)).unwrap().revision = Revision(99);
    assert_eq!(
        world.commit(&prepared),
        Err(TxError::StaleDependency(
            DependencyKey::VegetationPatchRevision(PatchId(1))
        ))
    );
    assert_eq!(world.ground, ground_before);
    assert!(world.transactions.is_empty());
}

#[test]
fn optional_current_absence_allows_ground_publication() {
    let mut world = fixture_world();
    let vegetation = world.vegetation.patches.get_mut(&PatchId(1)).unwrap();
    vegetation.present = false;
    vegetation.membership_generation = Generation(60);

    let prepared = prepare_transaction(
        &world,
        TransactionId(3),
        PatchId(1),
        profile(VegetationRequirement::OptionalWithExplicitAbsence),
        EffectId(30),
        2,
        EffectId(31),
        9,
    )
    .unwrap();
    assert!(matches!(
        prepared.vegetation,
        PreparedVegetation::NoSubject(_)
    ));
    world.commit(&prepared).unwrap();
    assert_eq!(world.ground.patches[&PatchId(1)].disturbance, 5);
    assert!(world.vegetation.receipts.is_empty());
}

#[test]
fn stale_absence_rejects_without_ground_publication() {
    let mut world = fixture_world();
    let vegetation = world.vegetation.patches.get_mut(&PatchId(1)).unwrap();
    vegetation.present = false;
    vegetation.membership_generation = Generation(60);

    let prepared = prepare_transaction(
        &world,
        TransactionId(4),
        PatchId(1),
        profile(VegetationRequirement::OptionalWithExplicitAbsence),
        EffectId(40),
        3,
        EffectId(41),
        0,
    )
    .unwrap();
    let ground_before = world.ground.clone();
    let vegetation = world.vegetation.patches.get_mut(&PatchId(1)).unwrap();
    vegetation.present = true;
    vegetation.membership_generation = Generation(61);

    assert_eq!(
        world.commit(&prepared),
        Err(TxError::StaleDependency(
            DependencyKey::VegetationMembership(PatchId(1))
        ))
    );
    assert_eq!(world.ground, ground_before);
}

#[test]
fn unrelated_patch_mutation_does_not_stale_local_transaction() {
    let mut world = fixture_world();
    let prepared = prepare_transaction(
        &world,
        TransactionId(5),
        PatchId(1),
        profile(VegetationRequirement::Required),
        EffectId(50),
        1,
        EffectId(51),
        1,
    )
    .unwrap();
    world.ground.patches.get_mut(&PatchId(2)).unwrap().revision = Revision(200);
    world
        .vegetation
        .patches
        .get_mut(&PatchId(2))
        .unwrap()
        .revision = Revision(300);
    world.commit(&prepared).unwrap();
    assert_eq!(world.ground.patches[&PatchId(1)].disturbance, 4);
    assert_eq!(world.vegetation.patches[&PatchId(1)].damage, 3);
}

#[test]
fn same_semantic_retry_is_idempotent() {
    let mut world = fixture_world();
    let prepared = prepare_transaction(
        &world,
        TransactionId(6),
        PatchId(1),
        profile(VegetationRequirement::Required),
        EffectId(60),
        2,
        EffectId(61),
        2,
    )
    .unwrap();
    let first = world.commit(&prepared).unwrap();
    let after_first = world.clone();
    let second = world.commit(&prepared).unwrap();
    assert_eq!(first, second);
    assert_eq!(world, after_first);
}

#[test]
fn partial_child_history_fails_closed() {
    let mut world = fixture_world();
    let prepared = prepare_transaction(
        &world,
        TransactionId(7),
        PatchId(1),
        profile(VegetationRequirement::Required),
        EffectId(70),
        2,
        EffectId(71),
        2,
    )
    .unwrap();
    world.ground.receipts.insert(
        EffectId(999),
        GroundReceipt {
            transaction_id: TransactionId(7),
            effect_id: EffectId(999),
            patch: PatchId(1),
            before: Revision(10),
            after: Revision(11),
        },
    );
    let ground_before = world.ground.patches[&PatchId(1)];
    assert_eq!(
        world.commit(&prepared),
        Err(TxError::PartialHistoryDetected(TransactionId(7)))
    );
    assert_eq!(world.ground.patches[&PatchId(1)], ground_before);
}

#[test]
fn child_enumeration_order_is_canonical() {
    let world = fixture_world();
    let prepared = prepare_transaction(
        &world,
        TransactionId(8),
        PatchId(1),
        profile(VegetationRequirement::Required),
        EffectId(80),
        2,
        EffectId(81),
        2,
    )
    .unwrap();
    let ground = ground_identity(prepared.ground);
    let vegetation = vegetation_identity(prepared.vegetation);
    let forward = canonical_child_set([
        (DomainKey::Ground, ground.clone()),
        (DomainKey::Vegetation, vegetation.clone()),
    ])
    .unwrap();
    let reverse = canonical_child_set([
        (DomainKey::Vegetation, vegetation),
        (DomainKey::Ground, ground),
    ])
    .unwrap();
    assert_eq!(forward, reverse);
}

#[test]
fn required_vegetation_absence_rejects_before_mutation() {
    let mut world = fixture_world();
    world.vegetation.patches.get_mut(&PatchId(1)).unwrap().present = false;
    let before = world.clone();
    assert_eq!(
        prepare_transaction(
            &world,
            TransactionId(9),
            PatchId(1),
            profile(VegetationRequirement::Required),
            EffectId(90),
            1,
            EffectId(91),
            1,
        ),
        Err(TxError::RequiredVegetationMissing(PatchId(1)))
    );
    assert_eq!(world, before);
}

#[test]
fn arithmetic_failure_happens_before_world_mutation() {
    let mut world = fixture_world();
    world.ground.patches.get_mut(&PatchId(1)).unwrap().disturbance = u32::MAX;
    let before = world.clone();
    assert_eq!(
        prepare_transaction(
            &world,
            TransactionId(10),
            PatchId(1),
            profile(VegetationRequirement::Required),
            EffectId(100),
            1,
            EffectId(101),
            1,
        ),
        Err(TxError::ArithmeticOverflow)
    );
    assert_eq!(world, before);
}
