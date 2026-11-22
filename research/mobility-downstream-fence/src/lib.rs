// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounded consistency checks for a declared downstream command-holding path.
//!
//! This crate does not stop hardware, cancel queues, authenticate receipts,
//! prove inventory completeness, classify physical effects, or authorize a
//! fresh action. It checks that one supplied receipt exists for each declared
//! command-holding layer and that identities, epochs, counters and declared
//! failure-domain references are internally consistent.
//!
//! A successful result means only that the supplied evidence packet is
//! structurally consistent for the exact declared inventory and transition.
//! It is not a hardware quiescence proof or a NoEffect classification.

pub const CONTRACT_VERSION: &str = "0.1";
pub const MAX_COMMAND_HOLDING_LAYERS: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Commitment(pub [u8; 32]);

impl Commitment {
    pub fn is_nonzero(&self) -> bool {
        self.0.iter().any(|byte| *byte != 0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandHoldingLayer {
    /// Stable identifier from the platform's frozen command-path inventory.
    pub layer_id: String,
    /// Zero-based order from native acceptance toward physical execution.
    pub path_index: u16,
    /// Vendor/platform-specific layer kind; deliberately not a closed enum.
    pub kind: String,
    /// Commitment to this layer's exact configuration and firmware profile.
    pub configuration_commitment: Commitment,
    /// Monotonic counter generation. A reset/wrap requires a new generation.
    pub sequence_generation_id: String,
    /// Declared owner failure domain for this command-holding layer.
    pub owner_failure_domain_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueuePathInventory {
    pub stack_id: String,
    pub profile_revision: String,
    pub stack_configuration_commitment: Commitment,
    /// Commitment over the complete declared path inventory, computed and
    /// authenticated by its owner outside this reference checker.
    pub inventory_commitment: Commitment,
    pub layers: Vec<CommandHoldingLayer>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestartTransition {
    pub logical_effect_id: String,
    /// The old attempt whose retained work must be reconciled.
    pub old_attempt_id: String,
    pub old_boot_epoch_id: String,
    pub new_boot_epoch_id: String,
    pub old_controller_epoch: u64,
    pub new_controller_epoch: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueueDisposition {
    Quiescent,
    Cancelled,
    Drained,
    NotQuiescent,
    Unknown,
    Disputed,
    Unsupported,
}

impl QueueDisposition {
    fn is_admissible(self) -> bool {
        matches!(self, Self::Quiescent | Self::Cancelled | Self::Drained)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayerQuiescenceReceipt {
    pub receipt_id: String,
    pub evidence_id: String,
    pub layer_id: String,
    pub layer_configuration_commitment: Commitment,
    pub stack_configuration_commitment: Commitment,
    pub inventory_commitment: Commitment,
    pub logical_effect_id: String,
    pub old_attempt_id: String,
    pub old_boot_epoch_id: String,
    pub new_boot_epoch_id: String,
    pub old_controller_epoch: u64,
    pub new_controller_epoch: u64,
    pub sequence_generation_id: String,
    /// High-water sequence of commands accepted by this layer in this
    /// generation before the fence transition.
    pub last_accepted_sequence: u64,
    /// High-water sequence known to be retired/cancelled in the same
    /// generation. Equality is required for this v0.1 profile.
    pub last_retired_sequence: u64,
    pub outstanding_commands: u32,
    pub disposition: QueueDisposition,
    /// Issuer must match the layer owner declared in the inventory.
    pub issuer_failure_domain_id: String,
    /// A separately declared verifier. Identifier inequality is only a
    /// structural check; it does not prove physical or statistical
    /// independence, honesty, or absence of common-cause failures.
    pub verifier_failure_domain_id: String,
    /// Digest of the retained raw evidence bytes, computed externally.
    pub raw_evidence_commitment: Commitment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DownstreamFencePacket {
    pub inventory: QueuePathInventory,
    pub transition: RestartTransition,
    pub receipts: Vec<LayerQuiescenceReceipt>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HoldReason {
    MissingIdentity,
    InvalidCommitment,
    EmptyInventory,
    TooManyLayers,
    DuplicateLayerId,
    DuplicatePathIndex,
    NonContiguousPath,
    LayerProfileIncomplete,
    BootEpochUnchanged,
    ControllerEpochNotAdvanced,
    ReceiptCountMismatch,
    DuplicateReceipt,
    UnexpectedReceipt,
    MissingReceipt,
    ReceiptEvidenceMissing,
    LayerIdentityMismatch,
    LayerConfigurationMismatch,
    StackConfigurationMismatch,
    InventoryCommitmentMismatch,
    EffectIdentityMismatch,
    AttemptIdentityMismatch,
    BootEpochMismatch,
    ControllerEpochMismatch,
    SequenceGenerationMismatch,
    IssuerFailureDomainMismatch,
    VerifierFailureDomainNotDistinct,
    LayerNotQuiescent,
    OutstandingCommands,
    SequenceHighWaterMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FenceDecision {
    /// All layers in the supplied declared inventory have structurally
    /// consistent current-epoch receipts. This is not action authority, a
    /// physical NoEffect result, or proof that the inventory is complete.
    AllDeclaredLayersHaveCurrentQuiescenceEvidence {
        qualified_layers: usize,
    },
    Hold(HoldReason),
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn hold(reason: HoldReason) -> FenceDecision {
    FenceDecision::Hold(reason)
}

fn valid_inventory(inventory: &QueuePathInventory) -> Result<(), HoldReason> {
    if !nonempty(&inventory.stack_id) || !nonempty(&inventory.profile_revision) {
        return Err(HoldReason::MissingIdentity);
    }
    if !inventory.stack_configuration_commitment.is_nonzero()
        || !inventory.inventory_commitment.is_nonzero()
    {
        return Err(HoldReason::InvalidCommitment);
    }
    if inventory.layers.is_empty() {
        return Err(HoldReason::EmptyInventory);
    }
    if inventory.layers.len() > MAX_COMMAND_HOLDING_LAYERS {
        return Err(HoldReason::TooManyLayers);
    }

    for (index, layer) in inventory.layers.iter().enumerate() {
        if !nonempty(&layer.layer_id)
            || !nonempty(&layer.kind)
            || !nonempty(&layer.sequence_generation_id)
            || !nonempty(&layer.owner_failure_domain_id)
            || !layer.configuration_commitment.is_nonzero()
        {
            return Err(HoldReason::LayerProfileIncomplete);
        }
        if inventory.layers[..index]
            .iter()
            .any(|prior| prior.layer_id == layer.layer_id)
        {
            return Err(HoldReason::DuplicateLayerId);
        }
        if inventory.layers[..index]
            .iter()
            .any(|prior| prior.path_index == layer.path_index)
        {
            return Err(HoldReason::DuplicatePathIndex);
        }
    }

    let mut indexes: Vec<u16> = inventory
        .layers
        .iter()
        .map(|layer| layer.path_index)
        .collect();
    indexes.sort_unstable();
    if indexes
        .iter()
        .enumerate()
        .any(|(expected, actual)| usize::from(*actual) != expected)
    {
        return Err(HoldReason::NonContiguousPath);
    }

    Ok(())
}

/// Assess supplied downstream queue receipts against the exact declared path.
///
/// The function is deterministic and dependency-free. It performs shape and
/// binding checks only; no cryptographic verification, persistence access,
/// device query, cancellation, STOP, queue operation, or dispatch is present.
pub fn assess_downstream_quiescence(packet: &DownstreamFencePacket) -> FenceDecision {
    let inventory = &packet.inventory;
    let transition = &packet.transition;

    if let Err(reason) = valid_inventory(inventory) {
        return hold(reason);
    }

    if !nonempty(&transition.logical_effect_id)
        || !nonempty(&transition.old_attempt_id)
        || !nonempty(&transition.old_boot_epoch_id)
        || !nonempty(&transition.new_boot_epoch_id)
    {
        return hold(HoldReason::MissingIdentity);
    }
    if transition.old_boot_epoch_id == transition.new_boot_epoch_id {
        return hold(HoldReason::BootEpochUnchanged);
    }
    if transition.new_controller_epoch <= transition.old_controller_epoch {
        return hold(HoldReason::ControllerEpochNotAdvanced);
    }

    if packet.receipts.len() != inventory.layers.len() {
        return hold(HoldReason::ReceiptCountMismatch);
    }

    for (index, receipt) in packet.receipts.iter().enumerate() {
        if packet.receipts[..index]
            .iter()
            .any(|prior| prior.layer_id == receipt.layer_id)
        {
            return hold(HoldReason::DuplicateReceipt);
        }
        if !inventory
            .layers
            .iter()
            .any(|layer| layer.layer_id == receipt.layer_id)
        {
            return hold(HoldReason::UnexpectedReceipt);
        }
    }

    for layer in &inventory.layers {
        let Some(receipt) = packet
            .receipts
            .iter()
            .find(|receipt| receipt.layer_id == layer.layer_id)
        else {
            return hold(HoldReason::MissingReceipt);
        };

        if !nonempty(&receipt.receipt_id)
            || !nonempty(&receipt.evidence_id)
            || !receipt.raw_evidence_commitment.is_nonzero()
        {
            return hold(HoldReason::ReceiptEvidenceMissing);
        }
        if receipt.layer_id != layer.layer_id {
            return hold(HoldReason::LayerIdentityMismatch);
        }
        if receipt.layer_configuration_commitment != layer.configuration_commitment {
            return hold(HoldReason::LayerConfigurationMismatch);
        }
        if receipt.stack_configuration_commitment != inventory.stack_configuration_commitment {
            return hold(HoldReason::StackConfigurationMismatch);
        }
        if receipt.inventory_commitment != inventory.inventory_commitment {
            return hold(HoldReason::InventoryCommitmentMismatch);
        }
        if receipt.logical_effect_id != transition.logical_effect_id {
            return hold(HoldReason::EffectIdentityMismatch);
        }
        if receipt.old_attempt_id != transition.old_attempt_id {
            return hold(HoldReason::AttemptIdentityMismatch);
        }
        if receipt.old_boot_epoch_id != transition.old_boot_epoch_id
            || receipt.new_boot_epoch_id != transition.new_boot_epoch_id
        {
            return hold(HoldReason::BootEpochMismatch);
        }
        if receipt.old_controller_epoch != transition.old_controller_epoch
            || receipt.new_controller_epoch != transition.new_controller_epoch
        {
            return hold(HoldReason::ControllerEpochMismatch);
        }
        if receipt.sequence_generation_id != layer.sequence_generation_id {
            return hold(HoldReason::SequenceGenerationMismatch);
        }
        if receipt.issuer_failure_domain_id != layer.owner_failure_domain_id {
            return hold(HoldReason::IssuerFailureDomainMismatch);
        }
        if !nonempty(&receipt.verifier_failure_domain_id)
            || receipt.verifier_failure_domain_id == layer.owner_failure_domain_id
        {
            return hold(HoldReason::VerifierFailureDomainNotDistinct);
        }
        if !receipt.disposition.is_admissible() {
            return hold(HoldReason::LayerNotQuiescent);
        }
        if receipt.outstanding_commands != 0 {
            return hold(HoldReason::OutstandingCommands);
        }
        if receipt.last_accepted_sequence != receipt.last_retired_sequence {
            return hold(HoldReason::SequenceHighWaterMismatch);
        }
    }

    FenceDecision::AllDeclaredLayersHaveCurrentQuiescenceEvidence {
        qualified_layers: inventory.layers.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commitment(byte: u8) -> Commitment {
        Commitment([byte; 32])
    }

    fn layer(id: &str, path_index: u16, owner: &str) -> CommandHoldingLayer {
        CommandHoldingLayer {
            layer_id: id.to_owned(),
            path_index,
            kind: "robotics_driver_queue".to_owned(),
            configuration_commitment: commitment(path_index as u8 + 1),
            sequence_generation_id: format!("seq-generation-{id}"),
            owner_failure_domain_id: owner.to_owned(),
        }
    }

    fn receipt(
        layer: &CommandHoldingLayer,
        disposition: QueueDisposition,
    ) -> LayerQuiescenceReceipt {
        LayerQuiescenceReceipt {
            receipt_id: format!("receipt-{}", layer.layer_id),
            evidence_id: format!("evidence-{}", layer.layer_id),
            layer_id: layer.layer_id.clone(),
            layer_configuration_commitment: layer.configuration_commitment,
            stack_configuration_commitment: commitment(9),
            inventory_commitment: commitment(8),
            logical_effect_id: "logical-effect-1".to_owned(),
            old_attempt_id: "attempt-old".to_owned(),
            old_boot_epoch_id: "boot-old".to_owned(),
            new_boot_epoch_id: "boot-new".to_owned(),
            old_controller_epoch: 7,
            new_controller_epoch: 8,
            sequence_generation_id: layer.sequence_generation_id.clone(),
            last_accepted_sequence: 42,
            last_retired_sequence: 42,
            outstanding_commands: 0,
            disposition,
            issuer_failure_domain_id: layer.owner_failure_domain_id.clone(),
            verifier_failure_domain_id: "independent-verifier-domain".to_owned(),
            raw_evidence_commitment: commitment(6),
        }
    }

    fn packet() -> DownstreamFencePacket {
        let layers = vec![
            layer("controller-queue", 0, "controller-domain"),
            layer("driver-queue", 1, "driver-domain"),
            layer("firmware-queue", 2, "firmware-domain"),
        ];
        let receipts = layers
            .iter()
            .map(|item| receipt(item, QueueDisposition::Quiescent))
            .collect();
        DownstreamFencePacket {
            inventory: QueuePathInventory {
                stack_id: "robot-stack-a".to_owned(),
                profile_revision: "profile-v0.1".to_owned(),
                stack_configuration_commitment: commitment(9),
                inventory_commitment: commitment(8),
                layers,
            },
            transition: RestartTransition {
                logical_effect_id: "logical-effect-1".to_owned(),
                old_attempt_id: "attempt-old".to_owned(),
                old_boot_epoch_id: "boot-old".to_owned(),
                new_boot_epoch_id: "boot-new".to_owned(),
                old_controller_epoch: 7,
                new_controller_epoch: 8,
            },
            receipts,
        }
    }

    fn assert_hold(packet: &DownstreamFencePacket, expected: HoldReason) {
        assert_eq!(
            assess_downstream_quiescence(packet),
            FenceDecision::Hold(expected)
        );
    }

    #[test]
    fn complete_declared_path_is_only_a_quiescence_evidence_candidate() {
        assert_eq!(
            assess_downstream_quiescence(&packet()),
            FenceDecision::AllDeclaredLayersHaveCurrentQuiescenceEvidence {
                qualified_layers: 3
            }
        );
    }

    #[test]
    fn missing_layer_receipt_holds() {
        let mut input = packet();
        input.receipts.pop();
        assert_hold(&input, HoldReason::ReceiptCountMismatch);
    }

    #[test]
    fn duplicate_receipt_holds() {
        let mut input = packet();
        input.receipts[1].layer_id = input.receipts[0].layer_id.clone();
        assert_hold(&input, HoldReason::DuplicateReceipt);
    }

    #[test]
    fn unexpected_receipt_holds() {
        let mut input = packet();
        input.receipts[2].layer_id = "hidden-actuator-queue".to_owned();
        assert_hold(&input, HoldReason::UnexpectedReceipt);
    }

    #[test]
    fn duplicate_inventory_layer_holds() {
        let mut input = packet();
        input.inventory.layers[2].layer_id = input.inventory.layers[0].layer_id.clone();
        assert_hold(&input, HoldReason::DuplicateLayerId);
    }

    #[test]
    fn path_order_must_be_contiguous() {
        let mut input = packet();
        input.inventory.layers[2].path_index = 4;
        assert_hold(&input, HoldReason::NonContiguousPath);
    }

    #[test]
    fn old_boot_or_non_advancing_controller_epoch_holds() {
        let mut input = packet();
        input.transition.new_boot_epoch_id = "boot-old".to_owned();
        assert_hold(&input, HoldReason::BootEpochUnchanged);

        let mut input = packet();
        input.transition.new_controller_epoch = 7;
        assert_hold(&input, HoldReason::ControllerEpochNotAdvanced);
    }

    #[test]
    fn receipt_must_bind_exact_effect_and_old_attempt() {
        let mut input = packet();
        input.receipts[0].logical_effect_id = "other-effect".to_owned();
        assert_hold(&input, HoldReason::EffectIdentityMismatch);

        let mut input = packet();
        input.receipts[0].old_attempt_id = "other-attempt".to_owned();
        assert_hold(&input, HoldReason::AttemptIdentityMismatch);
    }

    #[test]
    fn receipt_must_bind_both_boots_and_controller_epochs() {
        let mut input = packet();
        input.receipts[0].new_boot_epoch_id = "boot-stale".to_owned();
        assert_hold(&input, HoldReason::BootEpochMismatch);

        let mut input = packet();
        input.receipts[0].new_controller_epoch = 7;
        assert_hold(&input, HoldReason::ControllerEpochMismatch);
    }

    #[test]
    fn every_receipt_must_match_the_declared_configuration() {
        let mut input = packet();
        input.receipts[0].layer_configuration_commitment = commitment(55);
        assert_hold(&input, HoldReason::LayerConfigurationMismatch);

        let mut input = packet();
        input.receipts[0].stack_configuration_commitment = commitment(55);
        assert_hold(&input, HoldReason::StackConfigurationMismatch);

        let mut input = packet();
        input.receipts[0].inventory_commitment = commitment(55);
        assert_hold(&input, HoldReason::InventoryCommitmentMismatch);
    }

    #[test]
    fn sequence_reset_requires_a_new_inventory_generation() {
        let mut input = packet();
        input.receipts[1].sequence_generation_id = "after-reset".to_owned();
        assert_hold(&input, HoldReason::SequenceGenerationMismatch);
    }

    #[test]
    fn not_quiescent_unknown_disputed_and_unsupported_layers_hold() {
        for disposition in [
            QueueDisposition::NotQuiescent,
            QueueDisposition::Unknown,
            QueueDisposition::Disputed,
            QueueDisposition::Unsupported,
        ] {
            let mut input = packet();
            input.receipts[1].disposition = disposition;
            assert_hold(&input, HoldReason::LayerNotQuiescent);
        }
    }

    #[test]
    fn successful_cancel_or_drain_still_requires_zero_outstanding_and_equal_high_water() {
        for disposition in [QueueDisposition::Cancelled, QueueDisposition::Drained] {
            let mut input = packet();
            input.receipts[2].disposition = disposition;
            assert_eq!(
                assess_downstream_quiescence(&input),
                FenceDecision::AllDeclaredLayersHaveCurrentQuiescenceEvidence {
                    qualified_layers: 3
                }
            );
        }

        let mut input = packet();
        input.receipts[0].outstanding_commands = 1;
        assert_hold(&input, HoldReason::OutstandingCommands);

        let mut input = packet();
        input.receipts[0].last_retired_sequence = 41;
        assert_hold(&input, HoldReason::SequenceHighWaterMismatch);
    }

    #[test]
    fn receipt_issuer_and_declared_verifier_bind_separately() {
        let mut input = packet();
        input.receipts[0].issuer_failure_domain_id = "wrong-owner".to_owned();
        assert_hold(&input, HoldReason::IssuerFailureDomainMismatch);

        let mut input = packet();
        input.receipts[0].verifier_failure_domain_id =
            input.inventory.layers[0].owner_failure_domain_id.clone();
        assert_hold(&input, HoldReason::VerifierFailureDomainNotDistinct);
    }

    #[test]
    fn missing_evidence_and_zero_commitments_hold() {
        let mut input = packet();
        input.receipts[0].evidence_id.clear();
        assert_hold(&input, HoldReason::ReceiptEvidenceMissing);

        let mut input = packet();
        input.inventory.inventory_commitment = commitment(0);
        assert_hold(&input, HoldReason::InvalidCommitment);

        let mut input = packet();
        input.receipts[0].raw_evidence_commitment = commitment(0);
        assert_hold(&input, HoldReason::ReceiptEvidenceMissing);
    }

    #[test]
    fn empty_or_incomplete_inventories_hold() {
        let mut input = packet();
        input.inventory.layers.clear();
        assert_hold(&input, HoldReason::EmptyInventory);

        let mut input = packet();
        input.inventory.layers[1].kind.clear();
        assert_hold(&input, HoldReason::LayerProfileIncomplete);
    }

    #[test]
    fn layer_order_does_not_change_the_identity_join() {
        let mut input = packet();
        input.receipts.swap(0, 2);
        assert_eq!(
            assess_downstream_quiescence(&input),
            FenceDecision::AllDeclaredLayersHaveCurrentQuiescenceEvidence {
                qualified_layers: 3
            }
        );
    }

    #[test]
    fn inventory_is_bounded_to_64_layers() {
        let mut input = packet();
        for index in 3..=64 {
            let item = layer(&format!("layer-{index}"), index, &format!("owner-{index}"));
            input
                .receipts
                .push(receipt(&item, QueueDisposition::Quiescent));
            input.inventory.layers.push(item);
        }
        assert_hold(&input, HoldReason::TooManyLayers);
    }
}
