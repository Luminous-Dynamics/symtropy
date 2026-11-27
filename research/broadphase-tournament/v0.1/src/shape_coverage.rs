//! Independent transformed-OBB shape-contact coverage oracle.
//!
//! This module composes exact 2D/3D OBB SAT with conservative transformed
//! world-AABBs. It checks the broadphase safety relation:
//!
//!     ExactShapeContacts ⊆ BroadphaseAabbCandidates
//!
//! The candidate set is obtained from the tournament's independent exact
//! AABB-pair oracle on the conservative transformed bounds. It therefore
//! qualifies shape-to-AABB coverage and candidate amplification; it does not
//! claim a timed tree-backend result or full-engine integration. The existing
//! tournament separately qualifies tree backends against the same AABB oracle.

use incremental_aabb_tree_reference::{Aabb, ProxyId, ProxyPair};
use std::collections::BTreeSet;
use symtropy_math::{Bivector, Point, Rotor};

use super::{
    BackendKind, ProxyFrame, SceneFamily, TemporalScene, TournamentConfig, TournamentError,
    oracle_pairs, run_backend,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShapeCoverageAudit {
    pub exact_shape_contacts: u64,
    pub aabb_candidates: u64,
    pub candidate_only_false_positives: u64,
    pub missed_shape_contacts: u64,
}

impl ShapeCoverageAudit {
    #[must_use]
    pub fn recall(&self) -> Option<f64> {
        (self.exact_shape_contacts > 0).then(|| {
            (self.exact_shape_contacts - self.missed_shape_contacts) as f64
                / self.exact_shape_contacts as f64
        })
    }

    #[must_use]
    pub fn candidate_amplification(&self) -> Option<f64> {
        (self.exact_shape_contacts > 0)
            .then(|| self.aabb_candidates as f64 / self.exact_shape_contacts as f64)
    }
}

/// Audit the shape-contact subset property for two canonical pair relations.
#[must_use]
pub fn audit_shape_contact_coverage(
    exact_shape_contacts: &BTreeSet<ProxyPair>,
    aabb_candidates: &BTreeSet<ProxyPair>,
) -> ShapeCoverageAudit {
    ShapeCoverageAudit {
        exact_shape_contacts: exact_shape_contacts.len() as u64,
        aabb_candidates: aabb_candidates.len() as u64,
        candidate_only_false_positives: aabb_candidates
            .difference(exact_shape_contacts)
            .count() as u64,
        missed_shape_contacts: exact_shape_contacts
            .difference(aabb_candidates)
            .count() as u64,
    }
}

fn multiply_upper_nonnegative(
    left: f64,
    right: f64,
) -> Result<f64, TournamentError> {
    if !left.is_finite() || !right.is_finite() || left < 0.0 || right < 0.0 {
        return Err(TournamentError::InvalidBounds);
    }
    let product = left * right;
    if !product.is_finite() {
        return Err(TournamentError::InvalidBounds);
    }
    let upper = product.next_up();
    if upper.is_finite() {
        Ok(upper)
    } else {
        Err(TournamentError::InvalidBounds)
    }
}

fn add_upper_nonnegative(
    left: f64,
    right: f64,
) -> Result<f64, TournamentError> {
    if !left.is_finite() || !right.is_finite() || left < 0.0 || right < 0.0 {
        return Err(TournamentError::InvalidBounds);
    }
    let sum = left + right;
    if !sum.is_finite() {
        return Err(TournamentError::InvalidBounds);
    }
    let upper = sum.next_up();
    if upper.is_finite() {
        Ok(upper)
    } else {
        Err(TournamentError::InvalidBounds)
    }
}

fn lower_bound(center: f64, radius_upper: f64) -> Result<f64, TournamentError> {
    if !center.is_finite() || !radius_upper.is_finite() || radius_upper < 0.0 {
        return Err(TournamentError::InvalidBounds);
    }
    let value = center - radius_upper;
    if !value.is_finite() {
        return Err(TournamentError::InvalidBounds);
    }
    let lower = value.next_down();
    if lower.is_finite() {
        Ok(lower)
    } else {
        Err(TournamentError::InvalidBounds)
    }
}

fn upper_bound(center: f64, radius_upper: f64) -> Result<f64, TournamentError> {
    if !center.is_finite() || !radius_upper.is_finite() || radius_upper < 0.0 {
        return Err(TournamentError::InvalidBounds);
    }
    let value = center + radius_upper;
    if !value.is_finite() {
        return Err(TournamentError::InvalidBounds);
    }
    let upper = value.next_up();
    if upper.is_finite() {
        Ok(upper)
    } else {
        Err(TournamentError::InvalidBounds)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Obb2 {
    center: [f64; 2],
    half: [f64; 2],
    angle: f64,
}

impl Obb2 {
    fn axes(self) -> [[f64; 2]; 2] {
        let rotor = Rotor::<2>::from_plane_angle(
            &Bivector::<2>::unit_plane(0, 1),
            self.angle,
        );
        let axis_x = rotor.rotate_point(&Point::<2>::new([1.0, 0.0]));
        let axis_y = rotor.rotate_point(&Point::<2>::new([0.0, 1.0]));
        [
            [axis_x.coord(0), axis_x.coord(1)],
            [axis_y.coord(0), axis_y.coord(1)],
        ]
    }

    fn world_aabb(self) -> Result<Aabb<2>, TournamentError> {
        let axes = self.axes();
        let radius_x = add_upper_nonnegative(
            multiply_upper_nonnegative(self.half[0], axes[0][0].abs())?,
            multiply_upper_nonnegative(self.half[1], axes[1][0].abs())?,
        )?;
        let radius_y = add_upper_nonnegative(
            multiply_upper_nonnegative(self.half[0], axes[0][1].abs())?,
            multiply_upper_nonnegative(self.half[1], axes[1][1].abs())?,
        )?;
        Aabb::new(
            [
                lower_bound(self.center[0], radius_x)?,
                lower_bound(self.center[1], radius_y)?,
            ],
            [
                upper_bound(self.center[0], radius_x)?,
                upper_bound(self.center[1], radius_y)?,
            ],
        )
        .map_err(TournamentError::from)
    }
}

fn dot2(left: [f64; 2], right: [f64; 2]) -> f64 {
    left[0] * right[0] + left[1] * right[1]
}

/// Independent four-axis separating-axis test for two 2D oriented boxes.
fn obb2_intersects(left: Obb2, right: Obb2) -> bool {
    let left_axes = left.axes();
    let right_axes = right.axes();
    let delta = [
        right.center[0] - left.center[0],
        right.center[1] - left.center[1],
    ];

    for axis in left_axes.into_iter().chain(right_axes) {
        let left_radius = left.half[0] * dot2(left_axes[0], axis).abs()
            + left.half[1] * dot2(left_axes[1], axis).abs();
        let right_radius = right.half[0] * dot2(right_axes[0], axis).abs()
            + right.half[1] * dot2(right_axes[1], axis).abs();
        if dot2(delta, axis).abs() > left_radius + right_radius + 1.0e-12 {
            return false;
        }
    }
    true
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Obb3 {
    center: [f64; 3],
    half: [f64; 3],
    plane_angles_xy_yz_xz: [f64; 3],
}

impl Obb3 {
    /// Construct the exact XY -> YZ -> XZ rotor composition used by #7622.
    fn rotor(self) -> Rotor<3> {
        let [xy, yz, xz] = self.plane_angles_xy_yz_xz;
        let rotate_xy =
            Rotor::<3>::from_plane_angle(&Bivector::<3>::unit_plane(0, 1), xy);
        let rotate_yz =
            Rotor::<3>::from_plane_angle(&Bivector::<3>::unit_plane(1, 2), yz);
        let rotate_xz =
            Rotor::<3>::from_plane_angle(&Bivector::<3>::unit_plane(0, 2), xz);
        rotate_xz.compose(&rotate_yz.compose(&rotate_xy))
    }

    /// World-space local axes from the production Rotor implementation.
    fn axes(self) -> [[f64; 3]; 3] {
        let rotor = self.rotor();
        std::array::from_fn(|local_axis| {
            let mut basis = [0.0; 3];
            basis[local_axis] = 1.0;
            let world = rotor.rotate_point(&Point::<3>::new(basis));
            [world.coord(0), world.coord(1), world.coord(2)]
        })
    }

    fn world_aabb(self) -> Result<Aabb<3>, TournamentError> {
        let axes = self.axes();
        let mut radius = [0.0; 3];
        for world_axis in 0..3 {
            for local_axis in 0..3 {
                let term = multiply_upper_nonnegative(
                    self.half[local_axis],
                    axes[local_axis][world_axis].abs(),
                )?;
                radius[world_axis] = add_upper_nonnegative(radius[world_axis], term)?;
            }
        }
        Aabb::new(
            [
                lower_bound(self.center[0], radius[0])?,
                lower_bound(self.center[1], radius[1])?,
                lower_bound(self.center[2], radius[2])?,
            ],
            [
                upper_bound(self.center[0], radius[0])?,
                upper_bound(self.center[1], radius[1])?,
                upper_bound(self.center[2], radius[2])?,
            ],
        )
        .map_err(TournamentError::from)
    }
}

fn dot3(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross3(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

/// Independent 15-axis separating-axis test for two 3D oriented boxes.
fn obb3_intersects(left: Obb3, right: Obb3) -> bool {
    let left_axes = left.axes();
    let right_axes = right.axes();
    let delta = [
        right.center[0] - left.center[0],
        right.center[1] - left.center[1],
        right.center[2] - left.center[2],
    ];

    let mut candidates = Vec::with_capacity(15);
    candidates.extend(left_axes);
    candidates.extend(right_axes);
    for left_axis in left_axes {
        for right_axis in right_axes {
            candidates.push(cross3(left_axis, right_axis));
        }
    }

    for axis in candidates {
        let norm = dot3(axis, axis).sqrt();
        if norm < 1.0e-12 {
            continue;
        }
        let left_radius = left.half[0] * dot3(left_axes[0], axis).abs()
            + left.half[1] * dot3(left_axes[1], axis).abs()
            + left.half[2] * dot3(left_axes[2], axis).abs();
        let right_radius = right.half[0] * dot3(right_axes[0], axis).abs()
            + right.half[1] * dot3(right_axes[1], axis).abs()
            + right.half[2] * dot3(right_axes[2], axis).abs();
        if dot3(delta, axis).abs() > left_radius + right_radius + 1.0e-12 * norm {
            return false;
        }
    }
    true
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeCoverageRow {
    pub dimension: u8,
    pub requested_pairs: u64,
    pub exact_shape_contacts: u64,
    pub aabb_candidates: u64,
    pub candidate_only_false_positives: u64,
    pub missed_shape_contacts: u64,
    pub candidate_recall: Option<f64>,
    pub candidate_amplification: Option<f64>,
    pub seed: u64,
}

impl ShapeCoverageRow {
    #[must_use]
    pub const fn csv_header() -> &'static str {
        "dimension,requested_pairs,exact_shape_contacts,aabb_candidates,candidate_only_false_positives,missed_shape_contacts,candidate_recall,candidate_amplification,seed"
    }

    #[must_use]
    pub fn to_csv(self) -> String {
        let recall = self.candidate_recall.map_or_else(String::new, |value| format!("{value:.9}"));
        let amplification = self
            .candidate_amplification
            .map_or_else(String::new, |value| format!("{value:.6}"));
        format!(
            "{},{},{},{},{},{},{},{},{}",
            self.dimension,
            self.requested_pairs,
            self.exact_shape_contacts,
            self.aabb_candidates,
            self.candidate_only_false_positives,
            self.missed_shape_contacts,
            recall,
            amplification,
            self.seed
        )
    }
}

fn update_pair_audit(
    audit: &mut ShapeCoverageAudit,
    exact_contact: bool,
    aabb_candidate: bool,
) {
    if exact_contact {
        audit.exact_shape_contacts += 1;
    }
    if aabb_candidate {
        audit.aabb_candidates += 1;
    }
    match (exact_contact, aabb_candidate) {
        (true, false) => audit.missed_shape_contacts += 1,
        (false, true) => audit.candidate_only_false_positives += 1,
        _ => {}
    }
}

fn row_from_audit(
    dimension: u8,
    requested_pairs: u64,
    seed: u64,
    audit: ShapeCoverageAudit,
) -> ShapeCoverageRow {
    ShapeCoverageRow {
        dimension,
        requested_pairs,
        exact_shape_contacts: audit.exact_shape_contacts,
        aabb_candidates: audit.aabb_candidates,
        candidate_only_false_positives: audit.candidate_only_false_positives,
        missed_shape_contacts: audit.missed_shape_contacts,
        candidate_recall: audit.recall(),
        candidate_amplification: audit.candidate_amplification(),
        seed,
    }
}

/// Run independent 2D and 3D transformed-OBB shape-contact coverage campaigns.
///
/// Each OBB pair is placed in a two-proxy frame and checked by the existing
/// exact AABB oracle. The shape-contact set is independently classified by
/// the corresponding SAT reference. This deliberately isolates the safety
/// implication from broadphase candidate discovery to exact shape contacts.
pub fn run_shape_coverage_campaign(
    cases_per_dimension: usize,
) -> Result<Vec<ShapeCoverageRow>, TournamentError> {
    if cases_per_dimension == 0 {
        return Err(TournamentError::EmptyScene);
    }
    // These are the frozen independent-reference seeds in physics VVUQ PR #7622.
    const SEED_2D: u64 = 0x4551_4356_5655_3031;
    const SEED_3D: u64 = 0x3344_4f42_4256_5655;
    Ok(vec![
        run_2d(cases_per_dimension, SEED_2D)?,
        run_3d(cases_per_dimension, SEED_3D)?,
    ])
}

fn run_2d(cases: usize, seed: u64) -> Result<ShapeCoverageRow, TournamentError> {
    let mut state = seed;
    let half_a = [1.0, 0.5];
    let half_b = [0.75, 0.35];
    let mut audit = ShapeCoverageAudit::default();

    for index in 0..cases {
        let center_a = [0.0, 0.0];
        let angle_a =
            super::next_between(&mut state, -std::f64::consts::PI, std::f64::consts::PI);
        let angle_b =
            super::next_between(&mut state, -std::f64::consts::PI, std::f64::consts::PI);
        let center_b = [
            super::next_between(&mut state, -3.0, 3.0),
            super::next_between(&mut state, -2.0, 2.0),
        ];
        let left = Obb2 {
            center: center_a,
            half: half_a,
            angle: angle_a,
        };
        let right = Obb2 {
            center: center_b,
            half: half_b,
            angle: angle_b,
        };
        let id = u64::try_from(index).map_err(|_| TournamentError::CounterOverflow)?;
        let left_id = id.checked_mul(2).ok_or(TournamentError::CounterOverflow)?;
        let right_id = left_id.checked_add(1).ok_or(TournamentError::CounterOverflow)?;
        let pair = ProxyPair::new(ProxyId(left_id), ProxyId(right_id))
            .ok_or(TournamentError::DuplicateProxyId)?;
        let frame = [
            ProxyFrame { id: pair.0, bounds: left.world_aabb()? },
            ProxyFrame { id: pair.1, bounds: right.world_aabb()? },
        ];
        let aabb_pairs = oracle_pairs(&frame)?;
        let exact_contact = obb2_intersects(left, right);
        let aabb_candidate = aabb_pairs.contains(&pair);
        update_pair_audit(&mut audit, exact_contact, aabb_candidate);
    }

    Ok(row_from_audit(2, cases as u64, seed, audit))
}

fn run_3d(cases: usize, seed: u64) -> Result<ShapeCoverageRow, TournamentError> {
    let mut state = seed;
    let half_a = [1.0, 0.55, 0.3];
    let half_b = [0.8, 0.45, 0.25];
    let mut audit = ShapeCoverageAudit::default();

    for index in 0..cases {
        let center_a = [0.0, 0.0, 0.0];
        let plane_angles_a = [
            super::next_between(&mut state, -std::f64::consts::PI, std::f64::consts::PI),
            super::next_between(&mut state, -std::f64::consts::PI, std::f64::consts::PI),
            super::next_between(&mut state, -std::f64::consts::PI, std::f64::consts::PI),
        ];
        let plane_angles_b = [
            super::next_between(&mut state, -std::f64::consts::PI, std::f64::consts::PI),
            super::next_between(&mut state, -std::f64::consts::PI, std::f64::consts::PI),
            super::next_between(&mut state, -std::f64::consts::PI, std::f64::consts::PI),
        ];
        let center_b = [
            super::next_between(&mut state, -1.62, 1.62),
            super::next_between(&mut state, -0.90, 0.90),
            super::next_between(&mut state, -0.495, 0.495),
        ];
        let left = Obb3 {
            center: center_a,
            half: half_a,
            plane_angles_xy_yz_xz: plane_angles_a,
        };
        let right = Obb3 {
            center: center_b,
            half: half_b,
            plane_angles_xy_yz_xz: plane_angles_b,
        };
        let id = u64::try_from(index).map_err(|_| TournamentError::CounterOverflow)?;
        let left_id = id.checked_mul(2).ok_or(TournamentError::CounterOverflow)?;
        let right_id = left_id.checked_add(1).ok_or(TournamentError::CounterOverflow)?;
        let pair = ProxyPair::new(ProxyId(left_id), ProxyId(right_id))
            .ok_or(TournamentError::DuplicateProxyId)?;
        let frame = [
            ProxyFrame { id: pair.0, bounds: left.world_aabb()? },
            ProxyFrame { id: pair.1, bounds: right.world_aabb()? },
        ];
        let aabb_pairs = oracle_pairs(&frame)?;
        let exact_contact = obb3_intersects(left, right);
        let aabb_candidate = aabb_pairs.contains(&pair);
        update_pair_audit(&mut audit, exact_contact, aabb_candidate);
    }

    Ok(row_from_audit(3, cases as u64, seed, audit))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeCoverageBackendRow {
    pub dimension: u8,
    pub backend: &'static str,
    pub requested_obb_pairs: u64,
    pub exact_shape_contacts: u64,
    pub backend_candidate_pairs: u64,
    pub backend_aabb_false_negatives: u64,
    /// A value of 1.0 is derived only when the backend has zero AABB false
    /// negatives and the exact shape-contact-to-AABB oracle gate is zero-FN.
    pub derived_shape_contact_recall: Option<f64>,
    pub candidate_amplification: Option<f64>,
    /// Single cold-frame timing; not a p50/p95/p99 distribution.
    pub cold_frame_ns: u128,
    pub seed: u64,
}

impl ShapeCoverageBackendRow {
    #[must_use]
    pub const fn csv_header() -> &'static str {
        "dimension,backend,requested_obb_pairs,exact_shape_contacts,backend_candidate_pairs,backend_aabb_false_negatives,derived_shape_contact_recall,candidate_amplification,cold_frame_ns,seed"
    }

    #[must_use]
    pub fn to_csv(self) -> String {
        let recall = self
            .derived_shape_contact_recall
            .map_or_else(String::new, |value| format!("{value:.9}"));
        let amplification = self
            .candidate_amplification
            .map_or_else(String::new, |value| format!("{value:.6}"));
        format!(
            "{},{},{},{},{},{},{},{},{},{}",
            self.dimension,
            self.backend,
            self.requested_obb_pairs,
            self.exact_shape_contacts,
            self.backend_candidate_pairs,
            self.backend_aabb_false_negatives,
            recall,
            amplification,
            self.cold_frame_ns,
            self.seed
        )
    }
}

fn build_2d_backend_frame(
    cases: usize,
    seed: u64,
) -> Result<(Vec<ProxyFrame<2>>, BTreeSet<ProxyPair>), TournamentError> {
    let mut state = seed;
    let half_a = [1.0, 0.5];
    let half_b = [0.75, 0.35];
    let mut frame = Vec::with_capacity(cases.saturating_mul(2));
    let mut exact_contacts = BTreeSet::new();

    for index in 0..cases {
        // Preserve the random draw order used by PR #7622.
        let angle_a =
            super::next_between(&mut state, -std::f64::consts::PI, std::f64::consts::PI);
        let angle_b =
            super::next_between(&mut state, -std::f64::consts::PI, std::f64::consts::PI);
        let delta_x = super::next_between(&mut state, -3.0, 3.0);
        let delta_y = super::next_between(&mut state, -2.0, 2.0);
        let offset = index as f64 * 10.0;
        let left = Obb2 {
            center: [offset, 0.0],
            half: half_a,
            angle: angle_a,
        };
        let right = Obb2 {
            center: [offset + delta_x, delta_y],
            half: half_b,
            angle: angle_b,
        };
        let id = u64::try_from(index).map_err(|_| TournamentError::CounterOverflow)?;
        let left_id = id.checked_mul(2).ok_or(TournamentError::CounterOverflow)?;
        let right_id = left_id.checked_add(1).ok_or(TournamentError::CounterOverflow)?;
        let pair = ProxyPair::new(ProxyId(left_id), ProxyId(right_id))
            .ok_or(TournamentError::DuplicateProxyId)?;
        frame.push(ProxyFrame {
            id: pair.0,
            bounds: left.world_aabb()?,
        });
        frame.push(ProxyFrame {
            id: pair.1,
            bounds: right.world_aabb()?,
        });
        if obb2_intersects(left, right) {
            exact_contacts.insert(pair);
        }
    }
    Ok((frame, exact_contacts))
}

fn build_3d_backend_frame(
    cases: usize,
    seed: u64,
) -> Result<(Vec<ProxyFrame<3>>, BTreeSet<ProxyPair>), TournamentError> {
    let mut state = seed;
    let half_a = [1.0, 0.55, 0.3];
    let half_b = [0.8, 0.45, 0.25];
    let mut frame = Vec::with_capacity(cases.saturating_mul(2));
    let mut exact_contacts = BTreeSet::new();

    for index in 0..cases {
        // Preserve the #7622 XY, YZ, XZ rotation and translation draw order.
        let plane_angles_a = [
            super::next_between(&mut state, -std::f64::consts::PI, std::f64::consts::PI),
            super::next_between(&mut state, -std::f64::consts::PI, std::f64::consts::PI),
            super::next_between(&mut state, -std::f64::consts::PI, std::f64::consts::PI),
        ];
        let plane_angles_b = [
            super::next_between(&mut state, -std::f64::consts::PI, std::f64::consts::PI),
            super::next_between(&mut state, -std::f64::consts::PI, std::f64::consts::PI),
            super::next_between(&mut state, -std::f64::consts::PI, std::f64::consts::PI),
        ];
        let delta_x = super::next_between(&mut state, -1.62, 1.62);
        let delta_y = super::next_between(&mut state, -0.90, 0.90);
        let delta_z = super::next_between(&mut state, -0.495, 0.495);
        let offset = index as f64 * 10.0;
        let left = Obb3 {
            center: [offset, 0.0, 0.0],
            half: half_a,
            plane_angles_xy_yz_xz: plane_angles_a,
        };
        let right = Obb3 {
            center: [offset + delta_x, delta_y, delta_z],
            half: half_b,
            plane_angles_xy_yz_xz: plane_angles_b,
        };
        let id = u64::try_from(index).map_err(|_| TournamentError::CounterOverflow)?;
        let left_id = id.checked_mul(2).ok_or(TournamentError::CounterOverflow)?;
        let right_id = left_id.checked_add(1).ok_or(TournamentError::CounterOverflow)?;
        let pair = ProxyPair::new(ProxyId(left_id), ProxyId(right_id))
            .ok_or(TournamentError::DuplicateProxyId)?;
        frame.push(ProxyFrame {
            id: pair.0,
            bounds: left.world_aabb()?,
        });
        frame.push(ProxyFrame {
            id: pair.1,
            bounds: right.world_aabb()?,
        });
        if obb3_intersects(left, right) {
            exact_contacts.insert(pair);
        }
    }
    Ok((frame, exact_contacts))
}

/// Run all nine existing broadphase backends on the exact same OBB-derived
/// proxy frame. Backend false negatives are measured against the independent
/// AABB oracle. When that count is zero, the separately-qualified
/// ExactOBBContacts subset AabbOraclePairs property composes to prove
/// ExactOBBContacts subset BackendCandidates for that row.
pub fn run_shape_coverage_backend_campaign(
    cases_per_dimension: usize,
) -> Result<Vec<ShapeCoverageBackendRow>, TournamentError> {
    if cases_per_dimension == 0 {
        return Err(TournamentError::EmptyScene);
    }
    let seed_2d = 0x4551_4356_5655_3031;
    let seed_3d = 0x3344_4f42_4256_5655;
    let (frame_2d, contacts_2d) = build_2d_backend_frame(cases_per_dimension, seed_2d)?;
    let (frame_3d, contacts_3d) = build_3d_backend_frame(cases_per_dimension, seed_3d)?;
    let mut rows = Vec::with_capacity(BackendKind::ALL.len() * 2);
    rows.extend(run_backend_coverage::<2>(
        frame_2d,
        contacts_2d.len() as u64,
        cases_per_dimension,
        seed_2d,
    )?);
    rows.extend(run_backend_coverage::<3>(
        frame_3d,
        contacts_3d.len() as u64,
        cases_per_dimension,
        seed_3d,
    )?);
    Ok(rows)
}

fn run_backend_coverage<const D: usize>(
    frame: Vec<ProxyFrame<D>>,
    exact_shape_contacts: u64,
    requested_pairs: usize,
    seed: u64,
) -> Result<Vec<ShapeCoverageBackendRow>, TournamentError> {
    let scene = TemporalScene {
        family: SceneFamily::SparseCoherent,
        seed,
        declared_proxy_count: frame.len(),
        frames: vec![frame],
    };
    let oracle = scene
        .frames
        .iter()
        .map(|frame| oracle_pairs(frame))
        .collect::<Result<Vec<_>, _>>()?;
    let mut rows = Vec::with_capacity(BackendKind::ALL.len());
    for backend in BackendKind::ALL {
        let result = run_backend(
            &scene,
            &oracle,
            backend,
            "brute_force_all_pairs",
            1,
        )?;
        let recall = (result.false_negatives == 0 && exact_shape_contacts > 0)
            .then_some(1.0);
        let amplification = (exact_shape_contacts > 0).then(|| {
            result.candidate_pairs as f64 / exact_shape_contacts as f64
        });
        rows.push(ShapeCoverageBackendRow {
            dimension: u8::try_from(D).map_err(|_| TournamentError::CounterOverflow)?,
            backend: result.backend,
            requested_obb_pairs: u64::try_from(requested_pairs)
                .map_err(|_| TournamentError::CounterOverflow)?,
            exact_shape_contacts,
            backend_candidate_pairs: result.candidate_pairs,
            backend_aabb_false_negatives: result.false_negatives,
            derived_shape_contact_recall: recall,
            candidate_amplification: amplification,
            cold_frame_ns: result.p50_frame_ns,
            seed,
        });
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_2d_contacts_are_subset_of_conservative_aabb_candidates() {
        let row = run_2d(4096, 0x4551_4356_5655_3031).expect("2D shape coverage");
        assert!(row.exact_shape_contacts > 0);
        assert_eq!(row.missed_shape_contacts, 0);
        assert_eq!(row.candidate_recall, Some(1.0));
    }

    #[test]
    fn seeded_3d_contacts_are_subset_of_conservative_aabb_candidates() {
        let row = run_3d(4096, 0x3344_4f42_4256_5655).expect("3D shape coverage");
        assert!(row.exact_shape_contacts > 0);
        assert_eq!(row.missed_shape_contacts, 0);
        assert_eq!(row.candidate_recall, Some(1.0));
    }

    #[test]
    fn underbounded_candidate_relation_is_detected() {
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("pair");
        let exact = BTreeSet::from([pair]);
        let candidates = BTreeSet::new();
        let audit = audit_shape_contact_coverage(&exact, &candidates);
        assert_eq!(audit.missed_shape_contacts, 1);
        assert_eq!(audit.recall(), Some(0.0));
    }

    #[test]
    fn zero_contact_denominator_is_reported_as_undefined() {
        let audit = ShapeCoverageAudit {
            exact_shape_contacts: 0,
            aabb_candidates: 4,
            candidate_only_false_positives: 4,
            missed_shape_contacts: 0,
        };
        assert_eq!(audit.recall(), None);
        assert_eq!(audit.candidate_amplification(), None);
    }

    #[test]
    fn same_frame_backend_coverage_includes_all_nine_backends() {
        let rows = run_shape_coverage_backend_campaign(32)
            .expect("same-frame backend shape coverage");
        assert_eq!(rows.len(), 18);
        for row in rows {
            assert!(row.exact_shape_contacts > 0);
            assert_eq!(row.backend_aabb_false_negatives, 0, "{}", row.backend);
            assert_eq!(row.derived_shape_contact_recall, Some(1.0), "{}", row.backend);
        }
    }

}
