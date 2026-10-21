// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Provenance-compatible hierarchical summaries for large astronomical catalogs.
//!
//! Summary counts mean payload records in one declared schema and evidence layer.
//! They are not population estimates. Coverage describes the selected data product,
//! not the unknowable completeness of the real Milky Way. Parent summaries may be
//! built from direct child tiles only, and remain partial/unknown until the coverage
//! contract supports a stronger claim.

use std::error::Error;
use std::fmt;

use crate::galaxy_spatial::GalaxyCellId;

/// Epistemic layer is part of summary identity; layers are never silently blended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GalaxyEvidenceLayer {
    /// Records directly supplied by a named observational catalog.
    ObservedCatalog,
    /// Outputs from a declared stellar-population or density model.
    InferredPopulationModel,
    /// Deterministic or authored visual content with no observational claim.
    SyntheticPresentation,
}

/// Completeness of this tile within its declared data-product selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GalaxyCoverageState {
    /// All content in the declared selection for this spatial tile is represented.
    CompleteWithinSelection,
    /// Some content is represented, but completeness is not established.
    Partial,
    /// The data product cannot establish whether the tile is complete or empty.
    Unknown,
}

/// Digests that define exactly which records may be aggregated together.
///
/// A context belongs to one immutable catalog snapshot, frame/epoch transform,
/// selection function, payload schema, license/attribution bundle and evidence
/// layer. Non-zero digests are required so missing lineage cannot masquerade as
/// a real all-zero identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GalaxyCatalogContext {
    catalog_snapshot_digest: [u8; 32],
    frame_epoch_transform_digest: [u8; 32],
    source_selection_digest: [u8; 32],
    payload_schema_digest: [u8; 32],
    license_attribution_digest: [u8; 32],
    evidence_layer: GalaxyEvidenceLayer,
}

impl GalaxyCatalogContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        catalog_snapshot_digest: [u8; 32],
        frame_epoch_transform_digest: [u8; 32],
        source_selection_digest: [u8; 32],
        payload_schema_digest: [u8; 32],
        license_attribution_digest: [u8; 32],
        evidence_layer: GalaxyEvidenceLayer,
    ) -> Result<Self, GalaxyCatalogError> {
        let digests = [
            catalog_snapshot_digest,
            frame_epoch_transform_digest,
            source_selection_digest,
            payload_schema_digest,
            license_attribution_digest,
        ];
        if digests.iter().any(|digest| digest.iter().all(|byte| *byte == 0)) {
            return Err(GalaxyCatalogError::MissingContextDigest);
        }
        Ok(Self {
            catalog_snapshot_digest,
            frame_epoch_transform_digest,
            source_selection_digest,
            payload_schema_digest,
            license_attribution_digest,
            evidence_layer,
        })
    }

    pub const fn evidence_layer(self) -> GalaxyEvidenceLayer {
        self.evidence_layer
    }

    pub const fn catalog_snapshot_digest(self) -> [u8; 32] {
        self.catalog_snapshot_digest
    }

    pub const fn frame_epoch_transform_digest(self) -> [u8; 32] {
        self.frame_epoch_transform_digest
    }

    pub const fn source_selection_digest(self) -> [u8; 32] {
        self.source_selection_digest
    }

    pub const fn payload_schema_digest(self) -> [u8; 32] {
        self.payload_schema_digest
    }

    pub const fn license_attribution_digest(self) -> [u8; 32] {
        self.license_attribution_digest
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SummaryKind {
    Leaf,
    Aggregate,
}

/// Count summary for one cell and one evidence/provenance context.
///
/// A leaf count is the number of rows actually admitted to that tile under the
/// declared payload schema. An aggregate count is the checked sum of its direct
/// child summaries. Neither is automatically an estimate of all physical stars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalaxyTileSummary {
    cell: GalaxyCellId,
    context: GalaxyCatalogContext,
    payload_record_count: u64,
    coverage: GalaxyCoverageState,
    kind: SummaryKind,
    child_mask: u8,
    complete_child_mask: u8,
    known_child_mask: u8,
}

impl GalaxyTileSummary {
    /// Build a leaf summary for data already deduplicated and assigned to this cell.
    pub const fn leaf(
        cell: GalaxyCellId,
        context: GalaxyCatalogContext,
        payload_record_count: u64,
        coverage: GalaxyCoverageState,
    ) -> Self {
        Self {
            cell,
            context,
            payload_record_count,
            coverage,
            kind: SummaryKind::Leaf,
            child_mask: 0,
            complete_child_mask: 0,
            known_child_mask: 0,
        }
    }

    /// Create an empty parent summary that will be populated from direct children.
    pub const fn aggregate(cell: GalaxyCellId, context: GalaxyCatalogContext) -> Self {
        Self {
            cell,
            context,
            payload_record_count: 0,
            coverage: GalaxyCoverageState::Unknown,
            kind: SummaryKind::Aggregate,
            child_mask: 0,
            complete_child_mask: 0,
            known_child_mask: 0,
        }
    }

    pub const fn cell(&self) -> GalaxyCellId {
        self.cell
    }

    pub const fn context(&self) -> GalaxyCatalogContext {
        self.context
    }

    pub const fn payload_record_count(&self) -> u64 {
        self.payload_record_count
    }

    pub const fn coverage(&self) -> GalaxyCoverageState {
        self.coverage
    }

    pub const fn child_mask(&self) -> u8 {
        self.child_mask
    }

    /// Merge one direct child exactly once. Context mismatch, non-child tiles,
    /// duplicate octants, and arithmetic overflow leave this parent unchanged.
    pub fn merge_child(&mut self, child: &Self) -> Result<(), GalaxyCatalogError> {
        if self.kind != SummaryKind::Aggregate {
            return Err(GalaxyCatalogError::DestinationIsNotAggregate);
        }
        if child.cell.parent() != Some(self.cell) {
            return Err(GalaxyCatalogError::NotDirectChild);
        }
        if self.context.evidence_layer != child.context.evidence_layer {
            return Err(GalaxyCatalogError::EvidenceLayerMismatch);
        }
        if self.context != child.context {
            return Err(GalaxyCatalogError::LineageMismatch);
        }

        let octant = (child.cell.prefix() & 0b111) as u32;
        let bit = 1_u8 << octant;
        if self.child_mask & bit != 0 {
            return Err(GalaxyCatalogError::DuplicateChild);
        }
        let next_count = self
            .payload_record_count
            .checked_add(child.payload_record_count)
            .ok_or(GalaxyCatalogError::RecordCountOverflow)?;

        self.payload_record_count = next_count;
        self.child_mask |= bit;
        if child.coverage == GalaxyCoverageState::CompleteWithinSelection {
            self.complete_child_mask |= bit;
        }
        if child.coverage != GalaxyCoverageState::Unknown || child.payload_record_count > 0 {
            self.known_child_mask |= bit;
        }
        self.recompute_coverage();
        Ok(())
    }

    fn recompute_coverage(&mut self) {
        if self.child_mask == u8::MAX && self.complete_child_mask == u8::MAX {
            self.coverage = GalaxyCoverageState::CompleteWithinSelection;
        } else if self.known_child_mask != 0 {
            self.coverage = GalaxyCoverageState::Partial;
        } else {
            self.coverage = GalaxyCoverageState::Unknown;
        }
    }
}

/// Errors that prevent a spatial summary from overstating its evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GalaxyCatalogError {
    MissingContextDigest,
    DestinationIsNotAggregate,
    NotDirectChild,
    EvidenceLayerMismatch,
    LineageMismatch,
    DuplicateChild,
    RecordCountOverflow,
}

impl fmt::Display for GalaxyCatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for GalaxyCatalogError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(layer: GalaxyEvidenceLayer, snapshot_byte: u8) -> GalaxyCatalogContext {
        GalaxyCatalogContext::new(
            [snapshot_byte; 32],
            [2; 32],
            [3; 32],
            [4; 32],
            [5; 32],
            layer,
        )
        .expect("non-empty lineage digests")
    }

    fn leaf(
        parent: GalaxyCellId,
        octant: u8,
        count: u64,
        coverage: GalaxyCoverageState,
        context: GalaxyCatalogContext,
    ) -> GalaxyTileSummary {
        GalaxyTileSummary::leaf(
            parent.child(octant).expect("valid direct child"),
            context,
            count,
            coverage,
        )
    }

    #[test]
    fn eight_complete_child_tiles_produce_complete_parent_and_exact_count_sum() {
        let context = context(GalaxyEvidenceLayer::ObservedCatalog, 1);
        let mut parent = GalaxyTileSummary::aggregate(GalaxyCellId::root(), context);
        for octant in 0..8 {
            parent
                .merge_child(&leaf(
                    GalaxyCellId::root(),
                    octant,
                    u64::from(octant),
                    GalaxyCoverageState::CompleteWithinSelection,
                    context,
                ))
                .expect("unique direct child");
        }
        assert_eq!(parent.payload_record_count(), 28);
        assert_eq!(parent.child_mask(), u8::MAX);
        assert_eq!(
            parent.coverage(),
            GalaxyCoverageState::CompleteWithinSelection
        );
    }

    #[test]
    fn missing_child_prevents_false_complete_claim() {
        let context = context(GalaxyEvidenceLayer::ObservedCatalog, 1);
        let mut parent = GalaxyTileSummary::aggregate(GalaxyCellId::root(), context);
        for octant in 0..7 {
            parent
                .merge_child(&leaf(
                    GalaxyCellId::root(),
                    octant,
                    1,
                    GalaxyCoverageState::CompleteWithinSelection,
                    context,
                ))
                .expect("unique direct child");
        }
        assert_eq!(parent.payload_record_count(), 7);
        assert_eq!(parent.coverage(), GalaxyCoverageState::Partial);
    }

    #[test]
    fn empty_and_unknown_are_not_equivalent() {
        let context = context(GalaxyEvidenceLayer::ObservedCatalog, 1);
        let mut complete_empty =
            GalaxyTileSummary::aggregate(GalaxyCellId::root(), context);
        for octant in 0..8 {
            complete_empty
                .merge_child(&leaf(
                    GalaxyCellId::root(),
                    octant,
                    0,
                    GalaxyCoverageState::CompleteWithinSelection,
                    context,
                ))
                .expect("known complete empty child");
        }
        assert_eq!(complete_empty.payload_record_count(), 0);
        assert_eq!(
            complete_empty.coverage(),
            GalaxyCoverageState::CompleteWithinSelection
        );

        let unknown = GalaxyTileSummary::leaf(
            GalaxyCellId::root().child(0).unwrap(),
            context,
            0,
            GalaxyCoverageState::Unknown,
        );
        assert_eq!(unknown.payload_record_count(), 0);
        assert_eq!(unknown.coverage(), GalaxyCoverageState::Unknown);
    }

    #[test]
    fn duplicate_child_is_rejected_without_mutation() {
        let context = context(GalaxyEvidenceLayer::ObservedCatalog, 1);
        let child = leaf(
            GalaxyCellId::root(),
            3,
            11,
            GalaxyCoverageState::Partial,
            context,
        );
        let mut parent = GalaxyTileSummary::aggregate(GalaxyCellId::root(), context);
        parent.merge_child(&child).unwrap();
        let before = parent.clone();
        assert_eq!(
            parent.merge_child(&child),
            Err(GalaxyCatalogError::DuplicateChild)
        );
        assert_eq!(parent, before);
    }

    #[test]
    fn different_catalog_snapshot_cannot_be_merged() {
        let left = context(GalaxyEvidenceLayer::ObservedCatalog, 1);
        let right = context(GalaxyEvidenceLayer::ObservedCatalog, 9);
        let child = leaf(
            GalaxyCellId::root(),
            0,
            1,
            GalaxyCoverageState::CompleteWithinSelection,
            right,
        );
        let mut parent = GalaxyTileSummary::aggregate(GalaxyCellId::root(), left);
        assert_eq!(
            parent.merge_child(&child),
            Err(GalaxyCatalogError::LineageMismatch)
        );
        assert_eq!(parent.payload_record_count(), 0);
    }

    #[test]
    fn observed_inferred_and_synthetic_layers_cannot_be_blended() {
        let observed = context(GalaxyEvidenceLayer::ObservedCatalog, 1);
        let inferred = context(GalaxyEvidenceLayer::InferredPopulationModel, 1);
        let child = leaf(
            GalaxyCellId::root(),
            0,
            1,
            GalaxyCoverageState::CompleteWithinSelection,
            inferred,
        );
        let mut parent = GalaxyTileSummary::aggregate(GalaxyCellId::root(), observed);
        assert_eq!(
            parent.merge_child(&child),
            Err(GalaxyCatalogError::EvidenceLayerMismatch)
        );
    }

    #[test]
    fn child_from_another_frame_epoch_transform_cannot_be_merged() {
        let parent_context = context(GalaxyEvidenceLayer::ObservedCatalog, 1);
        let mut mismatched = parent_context;
        mismatched.frame_epoch_transform_digest = [8; 32];
        let child = leaf(
            GalaxyCellId::root(),
            0,
            10,
            GalaxyCoverageState::Partial,
            mismatched,
        );
        let mut parent =
            GalaxyTileSummary::aggregate(GalaxyCellId::root(), parent_context);
        assert_eq!(
            parent.merge_child(&child),
            Err(GalaxyCatalogError::LineageMismatch)
        );
    }

    #[test]
    fn overflow_does_not_partially_update_aggregate() {
        let context = context(GalaxyEvidenceLayer::ObservedCatalog, 1);
        let mut parent = GalaxyTileSummary::aggregate(GalaxyCellId::root(), context);
        parent
            .merge_child(&leaf(
                GalaxyCellId::root(),
                0,
                u64::MAX,
                GalaxyCoverageState::Partial,
                context,
            ))
            .unwrap();
        let before = parent.clone();
        assert_eq!(
            parent.merge_child(&leaf(
                GalaxyCellId::root(),
                1,
                1,
                GalaxyCoverageState::Partial,
                context,
            )),
            Err(GalaxyCatalogError::RecordCountOverflow)
        );
        assert_eq!(parent, before);
    }

    #[test]
    fn non_direct_child_and_leaf_destination_reject() {
        let context = context(GalaxyEvidenceLayer::ObservedCatalog, 1);
        let direct = GalaxyCellId::root().child(0).unwrap();
        let grandchild = direct.child(1).unwrap();
        let grandchild_summary = GalaxyTileSummary::leaf(
            grandchild,
            context,
            1,
            GalaxyCoverageState::CompleteWithinSelection,
        );
        let mut root = GalaxyTileSummary::aggregate(GalaxyCellId::root(), context);
        assert_eq!(
            root.merge_child(&grandchild_summary),
            Err(GalaxyCatalogError::NotDirectChild)
        );

        let mut leaf = GalaxyTileSummary::leaf(
            GalaxyCellId::root(),
            context,
            0,
            GalaxyCoverageState::Unknown,
        );
        assert_eq!(
            leaf.merge_child(&GalaxyTileSummary::leaf(
                direct,
                context,
                1,
                GalaxyCoverageState::Partial,
            )),
            Err(GalaxyCatalogError::DestinationIsNotAggregate)
        );
    }

    #[test]
    fn missing_lineage_digest_rejects() {
        assert_eq!(
            GalaxyCatalogContext::new(
                [0; 32],
                [2; 32],
                [3; 32],
                [4; 32],
                [5; 32],
                GalaxyEvidenceLayer::ObservedCatalog
            ),
            Err(GalaxyCatalogError::MissingContextDigest)
        );
    }
}
