// SPDX-License-Identifier: MIT

//! Continuous-domain interval certification for magnetic functional margins.
//!
//! This crate validates supplied derivative supremum bounds and interval arithmetic.
//! It does not solve a magnetic field or authenticate the supplied evidence.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub const ALL: [Self; 3] = [Self::X, Self::Y, Self::Z];

    pub const fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
            Self::Z => "z",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisAlignedBox3 {
    pub minimum_m: [f64; 3],
    pub maximum_m: [f64; 3],
}

impl AxisAlignedBox3 {
    pub fn validate(self) -> Result<(), CertificationError> {
        for axis in Axis::ALL {
            let i = axis.index();
            let minimum = self.minimum_m[i];
            let maximum = self.maximum_m[i];
            if !minimum.is_finite() || !maximum.is_finite() || minimum >= maximum {
                return Err(CertificationError::InvalidBox);
            }
            let half_width = self.half_widths_m()[i];
            let centre = self.centre_m()[i];
            if !half_width.is_finite()
                || half_width <= 0.0
                || !centre.is_finite()
                || centre <= minimum
                || centre >= maximum
            {
                return Err(CertificationError::UnrepresentableBoxCentre);
            }
        }
        Ok(())
    }

    pub fn centre_m(self) -> [f64; 3] {
        std::array::from_fn(|index| {
            let minimum = self.minimum_m[index];
            let maximum = self.maximum_m[index];
            if minimum.is_sign_negative() == maximum.is_sign_negative() {
                minimum + (maximum - minimum) * 0.5
            } else {
                minimum * 0.5 + maximum * 0.5
            }
        })
    }

    pub fn half_widths_m(self) -> [f64; 3] {
        let centre = self.centre_m();
        std::array::from_fn(|index| {
            let left_distance = centre[index] - self.minimum_m[index];
            let right_distance = self.maximum_m[index] - centre[index];
            next_up(left_distance.max(right_distance))
        })
    }

    pub fn widths_m(self) -> Result<[f64; 3], CertificationError> {
        self.validate()?;
        let widths = std::array::from_fn(|index| {
            self.maximum_m[index] - self.minimum_m[index]
        });
        if widths.iter().any(|value| !value.is_finite() || *value <= 0.0) {
            return Err(CertificationError::InvalidBox);
        }
        Ok(widths)
    }

    pub fn volume_m3(self) -> Result<f64, CertificationError> {
        let widths = self.widths_m()?;
        let volume = widths[0] * widths[1] * widths[2];
        if !volume.is_finite() || volume <= 0.0 {
            return Err(CertificationError::InvalidPartitionVolume);
        }
        Ok(volume)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceRole {
    AdaptiveExploration,
    ModelCalibration,
    BlindHoldout,
    PostSelectionQualification,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellClassification {
    CertifiedSafe,
    CertifiedFailing,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CellMarginEvidence {
    pub domain_id: String,
    pub cell_id: String,
    pub bounds: AxisAlignedBox3,
    pub centre_sample_id: String,
    pub centre_sample_sha256: String,
    pub centre_sample_position_m: [f64; 3],
    pub centre_margin_lower_bound: f64,
    pub centre_margin_upper_bound: f64,
    pub margin_unit: String,
    /// Component-wise absolute derivative supremum over the complete cell.
    pub derivative_abs_upper_by_axis_per_m: [f64; 3],
    pub derivative_unit: String,
    pub derivative_bound_cell_id: String,
    pub derivative_bound_record_ids: [String; 3],
    pub coordinate_frame_id: String,
    pub operating_condition_id: String,
    pub probe_calibration_id: String,
    pub model_set_id: String,
    pub solver_run_id: String,
    pub geometry_sha256: String,
    pub material_sha256: String,
    pub requirement_id: String,
    pub evidence_role: EvidenceRole,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CellCertificate {
    pub domain_id: String,
    pub cell_id: String,
    pub classification: CellClassification,
    pub derivative_contribution_by_axis: [f64; 3],
    pub maximum_spatial_variation: f64,
    pub margin_lower_enclosure: f64,
    pub margin_upper_enclosure: f64,
    pub dominant_refinement_axis: Option<Axis>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RefinementPolicy {
    pub minimum_child_width_m: [f64; 3],
    pub maximum_depth: u32,
    pub maximum_leaf_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefinementDisposition {
    AlreadyCertified,
    Split,
    MaximumDepthReached,
    LeafBudgetReached,
    MinimumWidthReached,
    NoSpatialSensitivity,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CellSplit {
    pub parent_cell_id: String,
    pub axis: Axis,
    pub split_coordinate_m: f64,
    pub left_child_id: String,
    pub right_child_id: String,
    pub left_bounds: AxisAlignedBox3,
    pub right_bounds: AxisAlignedBox3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RefinementPlan {
    pub certificate: CellCertificate,
    pub disposition: RefinementDisposition,
    pub split: Option<CellSplit>,
    pub suggested_probe_positions_m: Vec<[f64; 3]>,
    pub resulting_leaf_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartitionMetrics {
    pub domain_volume_m3: f64,
    pub summed_leaf_volume_m3: f64,
    pub leaf_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DomainCertificate {
    pub domain_id: String,
    pub coordinate_frame_id: String,
    pub operating_condition_id: String,
    pub probe_calibration_id: String,
    pub model_set_id: String,
    pub solver_run_id: String,
    pub geometry_sha256: String,
    pub material_sha256: String,
    pub requirement_id: String,
    pub margin_unit: String,
    pub derivative_unit: String,
    pub evidence_role: EvidenceRole,
    pub partition: PartitionMetrics,
    pub certified_safe_cell_count: usize,
    pub certified_failing_cell_count: usize,
    pub unresolved_cell_count: usize,
    pub all_domain_safe: bool,
    pub cells: Vec<CellCertificate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CertificationError {
    InvalidIdentity,
    InvalidSha256 { field: &'static str },
    InvalidBox,
    UnrepresentableBoxCentre,
    CentreSampleNotAtCellCentre,
    InvalidMarginInterval,
    InvalidDerivativeBound { axis: Axis },
    DerivativeBoundCellMismatch,
    NonFiniteDerivedMargin,
    InvalidRefinementPolicy,
    ZeroLeafCount,
    ProtectedEvidenceRole,
    NoRepresentableSplitMidpoint,
    InvalidSplit,
    EmptyPartition,
    CellOutsideDomain,
    OverlappingPartitionCells,
    InvalidPartitionVolume,
    PartitionVolumeMismatch,
    MixedPartitionEvidence,
    DuplicateCellId,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_digest(value: &str, field: &'static str) -> Result<(), CertificationError> {
    if valid_sha256(value) {
        Ok(())
    } else {
        Err(CertificationError::InvalidSha256 { field })
    }
}

fn same_point_exact(left: [f64; 3], right: [f64; 3]) -> bool {
    left.iter()
        .zip(right.iter())
        .all(|(left, right)| left.to_bits() == right.to_bits())
}

fn next_up(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    if value.is_sign_negative() {
        f64::from_bits(bits - 1)
    } else {
        f64::from_bits(bits + 1)
    }
}

fn next_down(value: f64) -> f64 {
    if value.is_nan() || value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits((1_u64 << 63) | 1);
    }
    let bits = value.to_bits();
    if value.is_sign_negative() {
        f64::from_bits(bits + 1)
    } else {
        f64::from_bits(bits - 1)
    }
}

fn multiply_up(left: f64, right: f64) -> f64 {
    if left == 0.0 || right == 0.0 {
        0.0
    } else {
        next_up(left * right)
    }
}

fn add_up(left: f64, right: f64) -> f64 {
    if left == 0.0 && right == 0.0 {
        0.0
    } else {
        next_up(left + right)
    }
}

fn subtract_down(left: f64, right: f64) -> f64 {
    next_down(left - right)
}

fn validate_evidence(evidence: &CellMarginEvidence) -> Result<(), CertificationError> {
    let identities = [
        evidence.domain_id.as_str(),
        evidence.cell_id.as_str(),
        evidence.centre_sample_id.as_str(),
        evidence.margin_unit.as_str(),
        evidence.derivative_unit.as_str(),
        evidence.derivative_bound_cell_id.as_str(),
        evidence.coordinate_frame_id.as_str(),
        evidence.operating_condition_id.as_str(),
        evidence.probe_calibration_id.as_str(),
        evidence.model_set_id.as_str(),
        evidence.solver_run_id.as_str(),
        evidence.requirement_id.as_str(),
    ];
    if identities.into_iter().any(|value| !nonempty(value))
        || evidence
            .derivative_bound_record_ids
            .iter()
            .any(|value| !nonempty(value))
    {
        return Err(CertificationError::InvalidIdentity);
    }
    validate_digest(
        &evidence.centre_sample_sha256,
        "centre_sample_sha256",
    )?;
    validate_digest(&evidence.geometry_sha256, "geometry_sha256")?;
    validate_digest(&evidence.material_sha256, "material_sha256")?;
    evidence.bounds.validate()?;

    if evidence.derivative_bound_cell_id != evidence.cell_id {
        return Err(CertificationError::DerivativeBoundCellMismatch);
    }
    if !same_point_exact(
        evidence.centre_sample_position_m,
        evidence.bounds.centre_m(),
    ) {
        return Err(CertificationError::CentreSampleNotAtCellCentre);
    }
    if !evidence.centre_margin_lower_bound.is_finite()
        || !evidence.centre_margin_upper_bound.is_finite()
        || evidence.centre_margin_lower_bound > evidence.centre_margin_upper_bound
    {
        return Err(CertificationError::InvalidMarginInterval);
    }
    for axis in Axis::ALL {
        let value = evidence.derivative_abs_upper_by_axis_per_m[axis.index()];
        if !value.is_finite() || value < 0.0 {
            return Err(CertificationError::InvalidDerivativeBound { axis });
        }
    }
    Ok(())
}

fn dominant_axis(contributions: [f64; 3]) -> Option<Axis> {
    let mut best_axis = Axis::X;
    let mut best_value = contributions[0];
    for axis in [Axis::Y, Axis::Z] {
        let value = contributions[axis.index()];
        if value > best_value {
            best_axis = axis;
            best_value = value;
        }
    }
    (best_value > 0.0).then_some(best_axis)
}

/// Enclose the named functional margin over the complete cell.
///
/// The result is conditional on the supplied component-wise derivative
/// suprema being valid everywhere inside this exact box.
pub fn certify_cell(
    evidence: &CellMarginEvidence,
) -> Result<CellCertificate, CertificationError> {
    validate_evidence(evidence)?;
    let half_widths = evidence.bounds.half_widths_m();
    let mut contributions = [0.0; 3];
    let mut maximum_spatial_variation = 0.0;

    for axis in Axis::ALL {
        let index = axis.index();
        contributions[index] = multiply_up(
            evidence.derivative_abs_upper_by_axis_per_m[index],
            half_widths[index],
        );
        if !contributions[index].is_finite() {
            return Err(CertificationError::NonFiniteDerivedMargin);
        }
        maximum_spatial_variation = add_up(maximum_spatial_variation, contributions[index]);
        if !maximum_spatial_variation.is_finite() {
            return Err(CertificationError::NonFiniteDerivedMargin);
        }
    }

    let margin_lower_enclosure =
        subtract_down(evidence.centre_margin_lower_bound, maximum_spatial_variation);
    let margin_upper_enclosure =
        add_up(evidence.centre_margin_upper_bound, maximum_spatial_variation);
    if !margin_lower_enclosure.is_finite() || !margin_upper_enclosure.is_finite() {
        return Err(CertificationError::NonFiniteDerivedMargin);
    }

    let classification = if margin_lower_enclosure > 0.0 {
        CellClassification::CertifiedSafe
    } else if margin_upper_enclosure < 0.0 {
        CellClassification::CertifiedFailing
    } else {
        CellClassification::Unresolved
    };

    Ok(CellCertificate {
        domain_id: evidence.domain_id.clone(),
        cell_id: evidence.cell_id.clone(),
        classification,
        derivative_contribution_by_axis: contributions,
        maximum_spatial_variation,
        margin_lower_enclosure,
        margin_upper_enclosure,
        dominant_refinement_axis: dominant_axis(contributions),
    })
}

fn stable_midpoint(minimum: f64, maximum: f64) -> Option<f64> {
    let midpoint = if minimum.is_sign_negative() == maximum.is_sign_negative() {
        minimum + (maximum - minimum) * 0.5
    } else {
        minimum * 0.5 + maximum * 0.5
    };
    (midpoint.is_finite() && midpoint > minimum && midpoint < maximum)
        .then_some(midpoint)
}

pub fn validate_split(
    parent_cell_id: &str,
    parent: AxisAlignedBox3,
    split: &CellSplit,
) -> Result<(), CertificationError> {
    parent.validate()?;
    split.left_bounds.validate()?;
    split.right_bounds.validate()?;
    let axis_index = split.axis.index();
    let axis_label = split.axis.label();
    let expected_left_id = format!("{parent_cell_id}/{axis_label}-0");
    let expected_right_id = format!("{parent_cell_id}/{axis_label}-1");

    if !nonempty(parent_cell_id)
        || split.parent_cell_id != parent_cell_id
        || split.left_child_id != expected_left_id
        || split.right_child_id != expected_right_id
        || split.left_child_id == split.right_child_id
        || !split.split_coordinate_m.is_finite()
        || split.split_coordinate_m <= parent.minimum_m[axis_index]
        || split.split_coordinate_m >= parent.maximum_m[axis_index]
    {
        return Err(CertificationError::InvalidSplit);
    }

    for axis in Axis::ALL {
        let index = axis.index();
        if axis == split.axis {
            if split.left_bounds.minimum_m[index] != parent.minimum_m[index]
                || split.left_bounds.maximum_m[index] != split.split_coordinate_m
                || split.right_bounds.minimum_m[index] != split.split_coordinate_m
                || split.right_bounds.maximum_m[index] != parent.maximum_m[index]
            {
                return Err(CertificationError::InvalidSplit);
            }
        } else if split.left_bounds.minimum_m[index] != parent.minimum_m[index]
            || split.left_bounds.maximum_m[index] != parent.maximum_m[index]
            || split.right_bounds.minimum_m[index] != parent.minimum_m[index]
            || split.right_bounds.maximum_m[index] != parent.maximum_m[index]
        {
            return Err(CertificationError::InvalidSplit);
        }
    }
    Ok(())
}

fn build_split(
    parent_cell_id: &str,
    parent: AxisAlignedBox3,
    axis: Axis,
) -> Result<CellSplit, CertificationError> {
    let index = axis.index();
    let midpoint = stable_midpoint(parent.minimum_m[index], parent.maximum_m[index])
        .ok_or(CertificationError::NoRepresentableSplitMidpoint)?;
    let mut left = parent;
    let mut right = parent;
    left.maximum_m[index] = midpoint;
    right.minimum_m[index] = midpoint;
    let axis_label = axis.label();

    let split = CellSplit {
        parent_cell_id: parent_cell_id.to_owned(),
        axis,
        split_coordinate_m: midpoint,
        left_child_id: format!("{parent_cell_id}/{axis_label}-0"),
        right_child_id: format!("{parent_cell_id}/{axis_label}-1"),
        left_bounds: left,
        right_bounds: right,
    };
    validate_split(parent_cell_id, parent, &split)?;
    Ok(split)
}

/// Plan one deterministic binary refinement for an unresolved adaptive cell.
/// The leaf count increases by one because one leaf becomes two.
pub fn plan_refinement(
    evidence: &CellMarginEvidence,
    current_depth: u32,
    current_leaf_count: usize,
    policy: RefinementPolicy,
) -> Result<RefinementPlan, CertificationError> {
    let certificate = certify_cell(evidence)?;
    if evidence.evidence_role != EvidenceRole::AdaptiveExploration {
        return Err(CertificationError::ProtectedEvidenceRole);
    }
    if current_leaf_count == 0 {
        return Err(CertificationError::ZeroLeafCount);
    }
    if policy
        .minimum_child_width_m
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
        || policy.maximum_leaf_count == 0
    {
        return Err(CertificationError::InvalidRefinementPolicy);
    }

    let finish = |disposition, split: Option<CellSplit>, resulting_leaf_count| {
        let suggested_probe_positions_m = split
            .as_ref()
            .map(|value| {
                vec![
                    value.left_bounds.centre_m(),
                    value.right_bounds.centre_m(),
                ]
            })
            .unwrap_or_default();
        RefinementPlan {
            certificate: certificate.clone(),
            disposition,
            split,
            suggested_probe_positions_m,
            resulting_leaf_count,
        }
    };

    if certificate.classification != CellClassification::Unresolved {
        return Ok(finish(
            RefinementDisposition::AlreadyCertified,
            None,
            current_leaf_count,
        ));
    }
    if current_depth >= policy.maximum_depth {
        return Ok(finish(
            RefinementDisposition::MaximumDepthReached,
            None,
            current_leaf_count,
        ));
    }
    if current_leaf_count >= policy.maximum_leaf_count {
        return Ok(finish(
            RefinementDisposition::LeafBudgetReached,
            None,
            current_leaf_count,
        ));
    }

    let mut selected_axis: Option<Axis> = None;
    let mut selected_contribution = 0.0;
    let mut any_axis_wide_enough = false;
    for axis in Axis::ALL {
        let index = axis.index();
        let Some(midpoint) = stable_midpoint(
            evidence.bounds.minimum_m[index],
            evidence.bounds.maximum_m[index],
        ) else {
            continue;
        };
        let left_child_width = midpoint - evidence.bounds.minimum_m[index];
        let right_child_width = evidence.bounds.maximum_m[index] - midpoint;
        if left_child_width < policy.minimum_child_width_m[index]
            || right_child_width < policy.minimum_child_width_m[index]
        {
            continue;
        }
        any_axis_wide_enough = true;
        let contribution = certificate.derivative_contribution_by_axis[index];
        if contribution > selected_contribution {
            selected_axis = Some(axis);
            selected_contribution = contribution;
        }
    }

    if !any_axis_wide_enough {
        return Ok(finish(
            RefinementDisposition::MinimumWidthReached,
            None,
            current_leaf_count,
        ));
    }
    let Some(axis) = selected_axis else {
        return Ok(finish(
            RefinementDisposition::NoSpatialSensitivity,
            None,
            current_leaf_count,
        ));
    };

    let split = build_split(&evidence.cell_id, evidence.bounds, axis)?;
    Ok(finish(
        RefinementDisposition::Split,
        Some(split),
        current_leaf_count + 1,
    ))
}

/// Validate that boxes are within a root domain, have no positive-volume
/// pairwise overlaps, and cover the root volume within floating-point summation
/// tolerance. Repeated validated parent/child splits provide the stronger
/// construction guarantee for exact boundary adjacency.
pub fn validate_leaf_partition(
    root: AxisAlignedBox3,
    leaves: &[AxisAlignedBox3],
) -> Result<PartitionMetrics, CertificationError> {
    if leaves.is_empty() {
        return Err(CertificationError::EmptyPartition);
    }
    root.validate()?;
    let domain_volume = root.volume_m3()?;
    let mut summed_volume = 0.0;

    for leaf in leaves {
        leaf.validate()?;
        for axis in Axis::ALL {
            let index = axis.index();
            if leaf.minimum_m[index] < root.minimum_m[index]
                || leaf.maximum_m[index] > root.maximum_m[index]
            {
                return Err(CertificationError::CellOutsideDomain);
            }
        }
        summed_volume += leaf.volume_m3()?;
        if !summed_volume.is_finite() {
            return Err(CertificationError::InvalidPartitionVolume);
        }
    }

    for (left_index, left) in leaves.iter().enumerate() {
        for right in leaves.iter().skip(left_index + 1) {
            let overlap_interior = Axis::ALL.into_iter().all(|axis| {
                let index = axis.index();
                left.maximum_m[index].min(right.maximum_m[index])
                    > left.minimum_m[index].max(right.minimum_m[index])
            });
            if overlap_interior {
                return Err(CertificationError::OverlappingPartitionCells);
            }
        }
    }

    let tolerance = domain_volume.max(summed_volume)
        * f64::EPSILON
        * 32.0
        * leaves.len() as f64;
    if (domain_volume - summed_volume).abs() > tolerance {
        return Err(CertificationError::PartitionVolumeMismatch);
    }

    Ok(PartitionMetrics {
        domain_volume_m3: domain_volume,
        summed_leaf_volume_m3: summed_volume,
        leaf_count: leaves.len(),
    })
}

/// Certify every leaf in a complete spatial partition under one shared
/// physical/model/evidence scope. The domain is certified safe only when every
/// cell enclosure is strictly positive and the leaves cover the root volume.
pub fn certify_domain(
    root: AxisAlignedBox3,
    evidence: &[CellMarginEvidence],
) -> Result<DomainCertificate, CertificationError> {
    if evidence.is_empty() {
        return Err(CertificationError::EmptyPartition);
    }

    let first = &evidence[0];
    let mut cell_ids = std::collections::HashSet::with_capacity(evidence.len());
    let mut cell_certificates = Vec::with_capacity(evidence.len());

    for cell in evidence {
        if !cell_ids.insert(cell.cell_id.as_str()) {
            return Err(CertificationError::DuplicateCellId);
        }
        if cell.domain_id != first.domain_id
            || cell.coordinate_frame_id != first.coordinate_frame_id
            || cell.operating_condition_id != first.operating_condition_id
            || cell.probe_calibration_id != first.probe_calibration_id
            || cell.model_set_id != first.model_set_id
            || cell.solver_run_id != first.solver_run_id
            || cell.geometry_sha256 != first.geometry_sha256
            || cell.material_sha256 != first.material_sha256
            || cell.requirement_id != first.requirement_id
            || cell.margin_unit != first.margin_unit
            || cell.derivative_unit != first.derivative_unit
            || cell.evidence_role != first.evidence_role
        {
            return Err(CertificationError::MixedPartitionEvidence);
        }
        cell_certificates.push(certify_cell(cell)?);
    }

    let leaves = evidence
        .iter()
        .map(|cell| cell.bounds)
        .collect::<Vec<_>>();
    let partition = validate_leaf_partition(root, &leaves)?;
    let certified_safe_cell_count = cell_certificates
        .iter()
        .filter(|cell| cell.classification == CellClassification::CertifiedSafe)
        .count();
    let certified_failing_cell_count = cell_certificates
        .iter()
        .filter(|cell| cell.classification == CellClassification::CertifiedFailing)
        .count();
    let unresolved_cell_count = cell_certificates
        .iter()
        .filter(|cell| cell.classification == CellClassification::Unresolved)
        .count();

    Ok(DomainCertificate {
        domain_id: first.domain_id.clone(),
        coordinate_frame_id: first.coordinate_frame_id.clone(),
        operating_condition_id: first.operating_condition_id.clone(),
        probe_calibration_id: first.probe_calibration_id.clone(),
        model_set_id: first.model_set_id.clone(),
        solver_run_id: first.solver_run_id.clone(),
        geometry_sha256: first.geometry_sha256.clone(),
        material_sha256: first.material_sha256.clone(),
        requirement_id: first.requirement_id.clone(),
        margin_unit: first.margin_unit.clone(),
        derivative_unit: first.derivative_unit.clone(),
        evidence_role: first.evidence_role,
        partition,
        certified_safe_cell_count,
        certified_failing_cell_count,
        unresolved_cell_count,
        all_domain_safe: certified_safe_cell_count == evidence.len(),
        cells: cell_certificates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn box3() -> AxisAlignedBox3 {
        AxisAlignedBox3 {
            minimum_m: [-1.0, -1.0, -1.0],
            maximum_m: [1.0, 1.0, 1.0],
        }
    }

    fn evidence() -> CellMarginEvidence {
        CellMarginEvidence {
            domain_id: "domain-main".to_owned(),
            cell_id: "cell-root".to_owned(),
            bounds: box3(),
            centre_sample_id: "sample-centre".to_owned(),
            centre_sample_sha256: DIGEST_A.to_owned(),
            centre_sample_position_m: [0.0, 0.0, 0.0],
            centre_margin_lower_bound: 1.0,
            centre_margin_upper_bound: 1.2,
            margin_unit: "radian".to_owned(),
            derivative_abs_upper_by_axis_per_m: [0.1, 0.4, 0.2],
            derivative_unit: "radian_per_metre".to_owned(),
            derivative_bound_cell_id: "cell-root".to_owned(),
            derivative_bound_record_ids: [
                "bound-x-v1".to_owned(),
                "bound-y-v1".to_owned(),
                "bound-z-v1".to_owned(),
            ],
            coordinate_frame_id: "sensor-frame-v1".to_owned(),
            operating_condition_id: "temperature-room-gap-nominal".to_owned(),
            probe_calibration_id: "probe-calibration-001".to_owned(),
            model_set_id: "model-set-001".to_owned(),
            solver_run_id: "solver-run-001".to_owned(),
            geometry_sha256: DIGEST_B.to_owned(),
            material_sha256: DIGEST_C.to_owned(),
            requirement_id: "minimum-phase-margin".to_owned(),
            evidence_role: EvidenceRole::AdaptiveExploration,
        }
    }

    fn policy() -> RefinementPolicy {
        RefinementPolicy {
            minimum_child_width_m: [0.1, 0.1, 0.1],
            maximum_depth: 8,
            maximum_leaf_count: 64,
        }
    }

    #[test]
    fn positive_lower_enclosure_certifies_the_entire_cell_safe() {
        let result = certify_cell(&evidence()).expect("valid cell evidence");
        assert_eq!(result.classification, CellClassification::CertifiedSafe);
        assert!((result.maximum_spatial_variation - 0.7).abs() < 1.0e-12);
        assert!((result.margin_lower_enclosure - 0.3).abs() < 1.0e-12);
    }

    #[test]
    fn negative_upper_enclosure_certifies_the_entire_cell_failing() {
        let mut input = evidence();
        input.centre_margin_lower_bound = -2.0;
        input.centre_margin_upper_bound = -1.0;
        let result = certify_cell(&input).expect("valid failing interval");
        assert_eq!(result.classification, CellClassification::CertifiedFailing);
        assert!(result.margin_upper_enclosure < 0.0);
    }

    #[test]
    fn touching_the_zero_boundary_is_unresolved() {
        let mut input = evidence();
        input.centre_margin_lower_bound = 0.75;
        input.centre_margin_upper_bound = 0.9;
        input.derivative_abs_upper_by_axis_per_m = [0.125, 0.25, 0.375];
        let result = certify_cell(&input).expect("valid boundary interval");
        assert!(result.margin_lower_enclosure <= 0.0);
        assert_eq!(result.classification, CellClassification::Unresolved);
    }

    #[test]
    fn refinement_splits_the_axis_with_the_largest_bound_contribution() {
        let mut input = evidence();
        input.centre_margin_lower_bound = 0.5;
        input.centre_margin_upper_bound = 0.8;
        let result = plan_refinement(&input, 0, 1, policy()).expect("valid refinement");
        assert_eq!(result.disposition, RefinementDisposition::Split);
        let split = result.split.expect("unresolved cell is split");
        assert_eq!(split.axis, Axis::Y);
        assert_eq!(split.split_coordinate_m, 0.0);
        assert_eq!(result.suggested_probe_positions_m.len(), 2);
        validate_split(&input.cell_id, input.bounds, &split).expect("children tile parent");
    }

    #[test]
    fn exact_split_children_cover_parent_without_gap_or_overlap() {
        let input = evidence();
        let split = build_split(&input.cell_id, input.bounds, Axis::Y).expect("representable");
        let metrics = validate_leaf_partition(
            input.bounds,
            &[split.left_bounds, split.right_bounds],
        )
        .expect("complete partition");
        assert_eq!(metrics.leaf_count, 2);
        assert!((metrics.domain_volume_m3 - metrics.summed_leaf_volume_m3).abs() < 1.0e-12);
    }

    #[test]
    fn a_gap_in_leaf_partition_is_rejected() {
        let root = box3();
        let mut left = root;
        let mut right = root;
        left.maximum_m[0] = -0.1;
        right.minimum_m[0] = 0.1;
        assert_eq!(
            validate_leaf_partition(root, &[left, right])
                .expect_err("missing slab is not covered"),
            CertificationError::PartitionVolumeMismatch
        );
    }

    #[test]
    fn a_positive_volume_overlap_is_rejected() {
        let root = box3();
        let mut left = root;
        let mut right = root;
        left.maximum_m[0] = 0.2;
        right.minimum_m[0] = -0.2;
        assert_eq!(
            validate_leaf_partition(root, &[left, right])
                .expect_err("overlap is invalid"),
            CertificationError::OverlappingPartitionCells
        );
    }

    #[test]
    fn protected_holdout_evidence_cannot_drive_adaptive_refinement() {
        let mut input = evidence();
        input.evidence_role = EvidenceRole::BlindHoldout;
        assert_eq!(
            plan_refinement(&input, 0, 1, policy())
                .expect_err("blind data must not guide adaptation"),
            CertificationError::ProtectedEvidenceRole
        );
    }

    #[test]
    fn zero_spatial_sensitivity_remains_unresolved_without_fake_refinement() {
        let mut input = evidence();
        input.centre_margin_lower_bound = -0.1;
        input.centre_margin_upper_bound = 0.1;
        input.derivative_abs_upper_by_axis_per_m = [0.0; 3];
        let result = plan_refinement(&input, 0, 1, policy()).expect("valid unresolved cell");
        assert_eq!(result.disposition, RefinementDisposition::NoSpatialSensitivity);
        assert!(result.split.is_none());
        assert_eq!(result.certificate.classification, CellClassification::Unresolved);
    }

    #[test]
    fn exhausted_budget_keeps_the_cell_unresolved() {
        let mut input = evidence();
        input.centre_margin_lower_bound = 0.5;
        input.centre_margin_upper_bound = 0.8;
        let mut bounded = policy();
        bounded.maximum_leaf_count = 1;
        let result = plan_refinement(&input, 0, 1, bounded).expect("valid budget");
        assert_eq!(result.disposition, RefinementDisposition::LeafBudgetReached);
        assert_eq!(result.certificate.classification, CellClassification::Unresolved);
    }

    #[test]
    fn increasing_derivative_bounds_cannot_improve_the_margin_enclosure() {
        let original = certify_cell(&evidence()).expect("original");
        let mut larger = evidence();
        larger.derivative_abs_upper_by_axis_per_m = [0.2, 0.8, 0.4];
        let conservative = certify_cell(&larger).expect("larger bounds");
        assert!(conservative.margin_lower_enclosure <= original.margin_lower_enclosure);
        assert!(conservative.margin_upper_enclosure >= original.margin_upper_enclosure);
    }

    #[test]
    fn centre_sample_must_be_at_exact_box_centre() {
        let mut input = evidence();
        input.centre_sample_position_m[0] = 0.01;
        assert_eq!(
            certify_cell(&input).expect_err("wrong sample location"),
            CertificationError::CentreSampleNotAtCellCentre
        );
    }

    #[test]
    fn derivative_bounds_must_be_bound_to_the_exact_cell() {
        let mut input = evidence();
        input.derivative_bound_cell_id = "another-cell".to_owned();
        assert_eq!(
            certify_cell(&input).expect_err("bound record belongs to another cell"),
            CertificationError::DerivativeBoundCellMismatch
        );
    }

    #[test]
    fn midpoint_that_cannot_be_represented_is_rejected() {
        let minimum = 1.0_f64;
        let maximum = f64::from_bits(minimum.to_bits() + 1);
        let narrow = AxisAlignedBox3 {
            minimum_m: [minimum, -1.0, -1.0],
            maximum_m: [maximum, 1.0, 1.0],
        };
        assert_eq!(
            narrow.validate().expect_err("no representable interior centre"),
            CertificationError::UnrepresentableBoxCentre
        );
    }

    #[test]
    fn translating_the_domain_and_sample_preserves_the_certificate() {
        let original = certify_cell(&evidence()).expect("original");
        let mut translated = evidence();
        let offset = [10.0, -4.0, 25.0];
        for axis in Axis::ALL {
            let index = axis.index();
            translated.bounds.minimum_m[index] += offset[index];
            translated.bounds.maximum_m[index] += offset[index];
            translated.centre_sample_position_m[index] += offset[index];
        }
        let shifted = certify_cell(&translated).expect("translated");
        assert_eq!(shifted.classification, original.classification);
        assert_eq!(shifted.maximum_spatial_variation, original.maximum_spatial_variation);
        assert_eq!(shifted.margin_lower_enclosure, original.margin_lower_enclosure);
        assert_eq!(shifted.margin_upper_enclosure, original.margin_upper_enclosure);
    }

    #[test]
    fn domain_certificate_requires_complete_safe_cells_under_one_scope() {
        let root = box3();
        let parent = evidence();
        let split = build_split(&parent.cell_id, root, Axis::Y).expect("valid split");
        let mut left = parent.clone();
        left.cell_id = split.left_child_id.clone();
        left.bounds = split.left_bounds;
        left.centre_sample_id = "left-centre".to_owned();
        left.centre_sample_position_m = left.bounds.centre_m();
        left.centre_sample_sha256 = DIGEST_A.to_owned();
        left.derivative_bound_cell_id = left.cell_id.clone();
        left.derivative_bound_record_ids = [
            "left-bound-x".to_owned(),
            "left-bound-y".to_owned(),
            "left-bound-z".to_owned(),
        ];
        let mut right = parent;
        right.cell_id = split.right_child_id.clone();
        right.bounds = split.right_bounds;
        right.centre_sample_id = "right-centre".to_owned();
        right.centre_sample_position_m = right.bounds.centre_m();
        right.centre_sample_sha256 = DIGEST_C.to_owned();
        right.derivative_bound_cell_id = right.cell_id.clone();
        right.derivative_bound_record_ids = [
            "right-bound-x".to_owned(),
            "right-bound-y".to_owned(),
            "right-bound-z".to_owned(),
        ];

        let result = certify_domain(root, &[left, right]).expect("complete evidence partition");
        assert!(result.all_domain_safe);
        assert_eq!(result.certified_safe_cell_count, 2);
        assert_eq!(result.certified_failing_cell_count, 0);
        assert_eq!(result.unresolved_cell_count, 0);
        assert_eq!(result.partition.leaf_count, 2);
    }

    #[test]
    fn domain_certificate_rejects_mixed_coordinate_frames() {
        let root = box3();
        let parent = evidence();
        let split = build_split(&parent.cell_id, root, Axis::Y).expect("valid split");
        let mut left = parent.clone();
        left.cell_id = split.left_child_id;
        left.bounds = split.left_bounds;
        left.centre_sample_position_m = left.bounds.centre_m();
        left.derivative_bound_cell_id = left.cell_id.clone();
        let mut right = parent;
        right.cell_id = split.right_child_id;
        right.bounds = split.right_bounds;
        right.centre_sample_position_m = right.bounds.centre_m();
        right.derivative_bound_cell_id = right.cell_id.clone();
        right.coordinate_frame_id = "different-frame".to_owned();

        assert_eq!(
            certify_domain(root, &[left, right])
                .expect_err("mixed frames cannot form one certificate"),
            CertificationError::MixedPartitionEvidence
        );
    }

    #[test]
    fn domain_certificate_rejects_duplicate_cell_ids() {
        let cell = evidence();
        assert_eq!(
            certify_domain(box3(), &[cell.clone(), cell])
                .expect_err("duplicate cell id"),
            CertificationError::DuplicateCellId
        );
    }

    #[test]
    fn domain_certificate_is_not_safe_when_any_cell_is_unresolved() {
        let root = box3();
        let parent = evidence();
        let split = build_split(&parent.cell_id, root, Axis::Y).expect("valid split");
        let mut left = parent.clone();
        left.cell_id = split.left_child_id;
        left.bounds = split.left_bounds;
        left.centre_sample_id = "left-centre".to_owned();
        left.centre_sample_position_m = left.bounds.centre_m();
        left.derivative_bound_cell_id = left.cell_id.clone();
        let mut right = parent;
        right.cell_id = split.right_child_id;
        right.bounds = split.right_bounds;
        right.centre_sample_id = "right-centre".to_owned();
        right.centre_sample_position_m = right.bounds.centre_m();
        right.centre_margin_lower_bound = -0.1;
        right.centre_margin_upper_bound = 0.1;
        right.derivative_bound_cell_id = right.cell_id.clone();

        let result = certify_domain(root, &[left, right]).expect("valid but unresolved domain");
        assert!(!result.all_domain_safe);
        assert_eq!(result.certified_safe_cell_count, 1);
        assert_eq!(result.unresolved_cell_count, 1);
    }

}
