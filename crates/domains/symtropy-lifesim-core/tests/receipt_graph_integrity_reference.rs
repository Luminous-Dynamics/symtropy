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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ProfileId(u64);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum DomainKey {
    Ground,
    Vegetation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NegativeVegetationObservation {
    patch: PatchId,
    authority_id: u64,
    membership_generation: Generation,
    spatial_generation: Generation,
    observation_profile: ProfileId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ChildIdentity {
    GroundApplied {
        effect_id: EffectId,
        patch: PatchId,
        before: Revision,
        after: Revision,
    },
    VegetationApplied {
        effect_id: EffectId,
        patch: PatchId,
        before: Revision,
        after: Revision,
    },
    VegetationNoEffect {
        patch: PatchId,
        revision: Revision,
    },
    VegetationNoSubject(NegativeVegetationObservation),
    VegetationProhibited,
}

impl ChildIdentity {
    fn effect_id(&self) -> Option<EffectId> {
        match self {
            Self::GroundApplied { effect_id, .. } | Self::VegetationApplied { effect_id, .. } => {
                Some(*effect_id)
            }
            Self::VegetationNoEffect { .. }
            | Self::VegetationNoSubject(_)
            | Self::VegetationProhibited => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TransactionIdentity {
    id: TransactionId,
    patch: PatchId,
    transaction_profile: ProfileId,
    currentness_profile: ProfileId,
    ground_response_profile: ProfileId,
    vegetation_response_profile: ProfileId,
    children: BTreeMap<DomainKey, ChildIdentity>,
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ReceiptGraph {
    ground: BTreeMap<EffectId, GroundReceipt>,
    vegetation: BTreeMap<EffectId, VegetationReceipt>,
    transactions: BTreeMap<TransactionId, TransactionReceipt>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GraphError {
    TransactionConflict(TransactionId),
    PartialHistoryDetected(TransactionId),
    EffectIdConflict(EffectId),
    MissingRequiredChild(DomainKey),
    UnexpectedChildReceipt(DomainKey),
    ChildReceiptMismatch(DomainKey),
}

impl ReceiptGraph {
    fn verify_transaction(&self, identity: &TransactionIdentity) -> Result<(), GraphError> {
        for (domain, child) in &identity.children {
            match (domain, child) {
                (
                    DomainKey::Ground,
                    ChildIdentity::GroundApplied {
                        effect_id,
                        patch,
                        before,
                        after,
                    },
                ) => {
                    let receipt = self
                        .ground
                        .get(effect_id)
                        .ok_or(GraphError::PartialHistoryDetected(identity.id))?;
                    if receipt
                        != &(GroundReceipt {
                            transaction_id: identity.id,
                            effect_id: *effect_id,
                            patch: *patch,
                            before: *before,
                            after: *after,
                        })
                    {
                        return Err(GraphError::ChildReceiptMismatch(DomainKey::Ground));
                    }
                }
                (
                    DomainKey::Vegetation,
                    ChildIdentity::VegetationApplied {
                        effect_id,
                        patch,
                        before,
                        after,
                    },
                ) => {
                    let receipt = self
                        .vegetation
                        .get(effect_id)
                        .ok_or(GraphError::PartialHistoryDetected(identity.id))?;
                    if receipt
                        != &(VegetationReceipt {
                            transaction_id: identity.id,
                            effect_id: *effect_id,
                            patch: *patch,
                            before: *before,
                            after: *after,
                        })
                    {
                        return Err(GraphError::ChildReceiptMismatch(DomainKey::Vegetation));
                    }
                }
                (DomainKey::Vegetation, ChildIdentity::VegetationNoEffect { .. })
                | (DomainKey::Vegetation, ChildIdentity::VegetationNoSubject(_))
                | (DomainKey::Vegetation, ChildIdentity::VegetationProhibited) => {}
                (DomainKey::Ground, _) | (DomainKey::Vegetation, ChildIdentity::GroundApplied { .. }) => {
                    return Err(GraphError::ChildReceiptMismatch(*domain));
                }
            }
        }

        if !identity.children.contains_key(&DomainKey::Ground) {
            return Err(GraphError::MissingRequiredChild(DomainKey::Ground));
        }

        Ok(())
    }

    fn exact_retry(&self, identity: &TransactionIdentity) -> Result<TransactionReceipt, GraphError> {
        let receipt = self
            .transactions
            .get(&identity.id)
            .ok_or(GraphError::PartialHistoryDetected(identity.id))?;
        if receipt.identity != *identity {
            return Err(GraphError::TransactionConflict(identity.id));
        }
        self.verify_transaction(identity)?;
        Ok(receipt.clone())
    }

    fn validate_effect_id_availability(
        &self,
        domain: DomainKey,
        transaction_id: TransactionId,
        child: &ChildIdentity,
    ) -> Result<(), GraphError> {
        let Some(effect_id) = child.effect_id() else {
            return Ok(());
        };
        match domain {
            DomainKey::Ground => {
                if let Some(existing) = self.ground.get(&effect_id) {
                    if existing.transaction_id != transaction_id {
                        return Err(GraphError::EffectIdConflict(effect_id));
                    }
                }
            }
            DomainKey::Vegetation => {
                if let Some(existing) = self.vegetation.get(&effect_id) {
                    if existing.transaction_id != transaction_id {
                        return Err(GraphError::EffectIdConflict(effect_id));
                    }
                }
            }
        }
        Ok(())
    }

    fn record_committed(
        &mut self,
        identity: TransactionIdentity,
        ground_receipt: GroundReceipt,
        vegetation_receipt: Option<VegetationReceipt>,
    ) -> Result<TransactionReceipt, GraphError> {
        if let Some(existing) = self.transactions.get(&identity.id) {
            return if existing.identity == identity {
                self.exact_retry(&identity)
            } else {
                Err(GraphError::TransactionConflict(identity.id))
            };
        }

        let ground_child = identity
            .children
            .get(&DomainKey::Ground)
            .ok_or(GraphError::MissingRequiredChild(DomainKey::Ground))?;
        self.validate_effect_id_availability(DomainKey::Ground, identity.id, ground_child)?;

        let vegetation_child = identity.children.get(&DomainKey::Vegetation);
        if let Some(child) = vegetation_child {
            self.validate_effect_id_availability(DomainKey::Vegetation, identity.id, child)?;
        }

        let expected_ground = match ground_child {
            ChildIdentity::GroundApplied {
                effect_id,
                patch,
                before,
                after,
            } => GroundReceipt {
                transaction_id: identity.id,
                effect_id: *effect_id,
                patch: *patch,
                before: *before,
                after: *after,
            },
            _ => return Err(GraphError::ChildReceiptMismatch(DomainKey::Ground)),
        };
        if ground_receipt != expected_ground {
            return Err(GraphError::ChildReceiptMismatch(DomainKey::Ground));
        }

        match (vegetation_child, vegetation_receipt) {
            (
                Some(ChildIdentity::VegetationApplied {
                    effect_id,
                    patch,
                    before,
                    after,
                }),
                Some(receipt),
            ) => {
                let expected = VegetationReceipt {
                    transaction_id: identity.id,
                    effect_id: *effect_id,
                    patch: *patch,
                    before: *before,
                    after: *after,
                };
                if receipt != expected {
                    return Err(GraphError::ChildReceiptMismatch(DomainKey::Vegetation));
                }
            }
            (
                Some(
                    ChildIdentity::VegetationNoEffect { .. }
                    | ChildIdentity::VegetationNoSubject(_)
                    | ChildIdentity::VegetationProhibited,
                ),
                None,
            )
            | (None, None) => {}
            (_, Some(_)) => return Err(GraphError::UnexpectedChildReceipt(DomainKey::Vegetation)),
            (Some(ChildIdentity::VegetationApplied { .. }), None) => {
                return Err(GraphError::MissingRequiredChild(DomainKey::Vegetation));
            }
            (Some(ChildIdentity::GroundApplied { .. }), _) => {
                return Err(GraphError::ChildReceiptMismatch(DomainKey::Vegetation));
            }
        }

        let mut next = self.clone();
        next.ground.insert(ground_receipt.effect_id, ground_receipt);
        if let Some(receipt) = vegetation_receipt {
            next.vegetation.insert(receipt.effect_id, receipt);
        }
        let transaction = TransactionReceipt {
            identity: identity.clone(),
        };
        next.transactions.insert(identity.id, transaction.clone());
        next.verify_transaction(&identity)?;
        *self = next;
        Ok(transaction)
    }
}

fn applied_identity(id: TransactionId, ground_effect: EffectId, vegetation_effect: EffectId) -> TransactionIdentity {
    TransactionIdentity {
        id,
        patch: PatchId(1),
        transaction_profile: ProfileId(10),
        currentness_profile: ProfileId(20),
        ground_response_profile: ProfileId(30),
        vegetation_response_profile: ProfileId(40),
        children: BTreeMap::from([
            (
                DomainKey::Ground,
                ChildIdentity::GroundApplied {
                    effect_id: ground_effect,
                    patch: PatchId(1),
                    before: Revision(4),
                    after: Revision(5),
                },
            ),
            (
                DomainKey::Vegetation,
                ChildIdentity::VegetationApplied {
                    effect_id: vegetation_effect,
                    patch: PatchId(1),
                    before: Revision(7),
                    after: Revision(8),
                },
            ),
        ]),
    }
}

fn no_subject_identity(id: TransactionId, ground_effect: EffectId) -> TransactionIdentity {
    TransactionIdentity {
        id,
        patch: PatchId(2),
        transaction_profile: ProfileId(11),
        currentness_profile: ProfileId(21),
        ground_response_profile: ProfileId(31),
        vegetation_response_profile: ProfileId(41),
        children: BTreeMap::from([
            (
                DomainKey::Ground,
                ChildIdentity::GroundApplied {
                    effect_id: ground_effect,
                    patch: PatchId(2),
                    before: Revision(10),
                    after: Revision(11),
                },
            ),
            (
                DomainKey::Vegetation,
                ChildIdentity::VegetationNoSubject(NegativeVegetationObservation {
                    patch: PatchId(2),
                    authority_id: 700,
                    membership_generation: Generation(9),
                    spatial_generation: Generation(12),
                    observation_profile: ProfileId(55),
                }),
            ),
        ]),
    }
}

fn ground_receipt(identity: &TransactionIdentity) -> GroundReceipt {
    match identity.children.get(&DomainKey::Ground).unwrap() {
        ChildIdentity::GroundApplied {
            effect_id,
            patch,
            before,
            after,
        } => GroundReceipt {
            transaction_id: identity.id,
            effect_id: *effect_id,
            patch: *patch,
            before: *before,
            after: *after,
        },
        _ => unreachable!(),
    }
}

fn vegetation_receipt(identity: &TransactionIdentity) -> Option<VegetationReceipt> {
    match identity.children.get(&DomainKey::Vegetation) {
        Some(ChildIdentity::VegetationApplied {
            effect_id,
            patch,
            before,
            after,
        }) => Some(VegetationReceipt {
            transaction_id: identity.id,
            effect_id: *effect_id,
            patch: *patch,
            before: *before,
            after: *after,
        }),
        _ => None,
    }
}

#[test]
fn exact_retry_revalidates_required_child_receipts() {
    let identity = applied_identity(TransactionId(1), EffectId(100), EffectId(200));
    let mut graph = ReceiptGraph::default();
    let receipt = graph
        .record_committed(
            identity.clone(),
            ground_receipt(&identity),
            vegetation_receipt(&identity),
        )
        .unwrap();
    assert_eq!(graph.exact_retry(&identity).unwrap(), receipt);

    graph.ground.remove(&EffectId(100));
    assert_eq!(
        graph.exact_retry(&identity),
        Err(GraphError::PartialHistoryDetected(TransactionId(1)))
    );
}

#[test]
fn mismatched_child_receipt_fails_closed() {
    let identity = applied_identity(TransactionId(2), EffectId(101), EffectId(201));
    let mut graph = ReceiptGraph::default();
    graph
        .record_committed(
            identity.clone(),
            ground_receipt(&identity),
            vegetation_receipt(&identity),
        )
        .unwrap();

    graph.vegetation.get_mut(&EffectId(201)).unwrap().after = Revision(99);
    assert_eq!(
        graph.exact_retry(&identity),
        Err(GraphError::ChildReceiptMismatch(DomainKey::Vegetation))
    );
}

#[test]
fn child_effect_id_collision_across_transactions_rejects_without_mutation() {
    let first = applied_identity(TransactionId(3), EffectId(102), EffectId(202));
    let mut graph = ReceiptGraph::default();
    graph
        .record_committed(
            first.clone(),
            ground_receipt(&first),
            vegetation_receipt(&first),
        )
        .unwrap();
    let before = graph.clone();

    let second = applied_identity(TransactionId(4), EffectId(102), EffectId(203));
    assert_eq!(
        graph.record_committed(
            second.clone(),
            ground_receipt(&second),
            vegetation_receipt(&second),
        ),
        Err(GraphError::EffectIdConflict(EffectId(102)))
    );
    assert_eq!(graph, before);
}

#[test]
fn semantic_profile_drift_is_a_transaction_conflict() {
    let identity = applied_identity(TransactionId(5), EffectId(103), EffectId(204));
    let mut graph = ReceiptGraph::default();
    graph
        .record_committed(
            identity.clone(),
            ground_receipt(&identity),
            vegetation_receipt(&identity),
        )
        .unwrap();

    let mut drifted = identity.clone();
    drifted.currentness_profile = ProfileId(999);
    assert_eq!(
        graph.exact_retry(&drifted),
        Err(GraphError::TransactionConflict(TransactionId(5)))
    );
}

#[test]
fn response_profile_drift_is_a_transaction_conflict() {
    let identity = applied_identity(TransactionId(6), EffectId(104), EffectId(205));
    let mut graph = ReceiptGraph::default();
    graph
        .record_committed(
            identity.clone(),
            ground_receipt(&identity),
            vegetation_receipt(&identity),
        )
        .unwrap();

    let mut drifted = identity.clone();
    drifted.vegetation_response_profile = ProfileId(777);
    assert_eq!(
        graph.exact_retry(&drifted),
        Err(GraphError::TransactionConflict(TransactionId(6)))
    );
}

#[test]
fn explicit_no_subject_is_part_of_top_level_identity_without_child_receipt() {
    let identity = no_subject_identity(TransactionId(7), EffectId(105));
    let mut graph = ReceiptGraph::default();
    let receipt = graph
        .record_committed(identity.clone(), ground_receipt(&identity), None)
        .unwrap();
    assert!(graph.vegetation.is_empty());
    assert_eq!(graph.exact_retry(&identity).unwrap(), receipt);

    let mut drifted = identity.clone();
    if let Some(ChildIdentity::VegetationNoSubject(obs)) =
        drifted.children.get_mut(&DomainKey::Vegetation)
    {
        obs.membership_generation = Generation(10);
    }
    assert_eq!(
        graph.exact_retry(&drifted),
        Err(GraphError::TransactionConflict(TransactionId(7)))
    );
}

#[test]
fn orphan_child_receipt_without_top_level_receipt_is_partial_history() {
    let identity = applied_identity(TransactionId(8), EffectId(106), EffectId(206));
    let mut graph = ReceiptGraph::default();
    let ground = ground_receipt(&identity);
    graph.ground.insert(ground.effect_id, ground);

    assert_eq!(
        graph.exact_retry(&identity),
        Err(GraphError::PartialHistoryDetected(TransactionId(8)))
    );
}

#[test]
fn unexpected_child_receipt_for_no_subject_rejects_before_publication() {
    let identity = no_subject_identity(TransactionId(9), EffectId(107));
    let mut graph = ReceiptGraph::default();
    let bogus_vegetation = VegetationReceipt {
        transaction_id: identity.id,
        effect_id: EffectId(999),
        patch: PatchId(2),
        before: Revision(1),
        after: Revision(2),
    };
    let before = graph.clone();

    assert_eq!(
        graph.record_committed(
            identity.clone(),
            ground_receipt(&identity),
            Some(bogus_vegetation),
        ),
        Err(GraphError::UnexpectedChildReceipt(DomainKey::Vegetation))
    );
    assert_eq!(graph, before);
}

#[test]
fn transaction_identity_is_independent_of_child_insertion_order() {
    let a = applied_identity(TransactionId(10), EffectId(108), EffectId(208));
    let mut reversed = BTreeMap::new();
    reversed.insert(
        DomainKey::Vegetation,
        a.children.get(&DomainKey::Vegetation).unwrap().clone(),
    );
    reversed.insert(
        DomainKey::Ground,
        a.children.get(&DomainKey::Ground).unwrap().clone(),
    );
    let b = TransactionIdentity {
        children: reversed,
        ..a.clone()
    };
    assert_eq!(a, b);
}
