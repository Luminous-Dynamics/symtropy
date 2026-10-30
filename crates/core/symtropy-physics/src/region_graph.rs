// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Reference model for explicitly mapped local-coordinate region transitions.
//!
//! This is a graph/state-transfer model only. It is not wired into PhysicsWorld,
//! collision detection, or production movement. It cannot establish the
//! physical existence of pocket universes or exotic spacetime topology.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use nalgebra::{SMatrix, SVector};

const DEFAULT_MAP_TOLERANCE: f64 = 1.0e-9;

/// Stable region identity; never reuse an ID inside a replay lineage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegionId(pub u64);

/// Stable interface identity; never reuse an ID inside a replay lineage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InterfaceId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossingDirection {
    NegativeToPositive,
    PositiveToNegative,
    Either,
}

/// Policy for a quantity crossing a region interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuantityPolicy {
    /// Quantity is required to remain invariant within the declared tolerance.
    Preserve,
    /// A caller-supplied exchange value must close the before/after ledger.
    Exchange,
    /// A named transform is declared, but no conservation claim is made.
    ExplicitTransform,
    /// This interface is not authorized to transfer this quantity.
    Reject,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegionGraphError {
    InvalidDimension,
    InvalidRegion {
        field: &'static str,
    },
    DuplicateRegion {
        id: RegionId,
    },
    UnknownRegion {
        id: RegionId,
    },
    DuplicateInterface {
        id: InterfaceId,
    },
    UnknownInterface {
        id: InterfaceId,
    },
    EmptyInterfacePath,
    DisconnectedInterfacePath {
        previous_target: RegionId,
        next_source: RegionId,
    },
    StalePathInterfaceRevision {
        interface: InterfaceId,
        expected: u64,
        actual: u64,
    },
    StalePathRegionModelRevision {
        region: RegionId,
        expected: u64,
        actual: u64,
    },
    ComposedPathMismatch,
    SelfInterface,
    InvalidBoundary {
        field: &'static str,
    },
    InvalidMap {
        field: &'static str,
    },
    SingularMap,
    InverseRequired,
    InverseMismatch,
    MissingTransformIdentity,
    UnitMismatchForConservation,
    MetricMismatchForConservation,
    UnsupportedMetricForConservation,
    NonIsometricMapForPreservation,
    BoundaryOutsideRegion,
}

impl fmt::Display for RegionGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimension => write!(f, "region graph requires D >= 1"),
            Self::InvalidRegion { field } => write!(f, "invalid region field {field}"),
            Self::DuplicateRegion { id } => write!(f, "duplicate region ID {}", id.0),
            Self::UnknownRegion { id } => write!(f, "unknown region ID {}", id.0),
            Self::DuplicateInterface { id } => write!(f, "duplicate interface ID {}", id.0),
            Self::UnknownInterface { id } => write!(f, "unknown interface ID {}", id.0),
            Self::EmptyInterfacePath => write!(f, "interface path must contain at least one edge"),
            Self::DisconnectedInterfacePath {
                previous_target,
                next_source,
            } => write!(
                f,
                "interface path is disconnected: {} does not match {}",
                previous_target.0, next_source.0
            ),
            Self::StalePathInterfaceRevision {
                interface,
                expected,
                actual,
            } => write!(
                f,
                "stale composed path interface {}: expected revision {expected}, actual {actual}",
                interface.0
            ),
            Self::StalePathRegionModelRevision {
                region,
                expected,
                actual,
            } => write!(
                f,
                "stale composed path region {}: expected model revision {expected}, actual {actual}",
                region.0
            ),
            Self::ComposedPathMismatch => {
                write!(f, "composed path no longer matches its declared interface lineage")
            }
            Self::SelfInterface => write!(f, "region interface must connect distinct region IDs"),
            Self::InvalidBoundary { field } => {
                write!(f, "invalid interface boundary field {field}")
            }
            Self::InvalidMap { field } => write!(f, "invalid affine map field {field}"),
            Self::SingularMap => write!(f, "affine region map is singular"),
            Self::InverseRequired => write!(f, "reversible interface requires an inverse map"),
            Self::InverseMismatch => {
                write!(f, "declared inverse map does not invert the forward map")
            }
            Self::MissingTransformIdentity => {
                write!(f, "explicit transform policy requires a transform identity")
            }
            Self::UnitMismatchForConservation => {
                write!(
                    f,
                    "preserve/exchange policies require compatible coordinate and time units"
                )
            }
            Self::MetricMismatchForConservation => {
                write!(
                    f,
                    "preserve/exchange policies require a compatible metric profile"
                )
            }
            Self::UnsupportedMetricForConservation => {
                write!(
                    f,
                    "the reference conservation ledger supports only the Euclidean metric profile"
                )
            }
            Self::NonIsometricMapForPreservation => {
                write!(
                    f,
                    "preserve policy requires an isometry with unchanged time scale"
                )
            }
            Self::BoundaryOutsideRegion => {
                write!(
                    f,
                    "interface boundary does not intersect its declared region domain"
                )
            }
        }
    }
}

impl Error for RegionGraphError {}

#[derive(Clone, Debug, PartialEq)]
pub struct RegionSpec<const D: usize> {
    pub id: RegionId,
    pub lower: SVector<f64, D>,
    pub upper: SVector<f64, D>,
    pub metric_profile: String,
    pub coordinate_unit: String,
    pub time_unit: String,
    /// Logical clock identity; different clocks require an explicit time map.
    pub clock_id: String,
    pub model_revision: u64,
}

impl<const D: usize> RegionSpec<D> {
    pub fn validate(&self) -> Result<(), RegionGraphError> {
        if D == 0 {
            return Err(RegionGraphError::InvalidDimension);
        }
        if self.metric_profile.trim().is_empty() {
            return Err(RegionGraphError::InvalidRegion {
                field: "metric_profile",
            });
        }
        if self.coordinate_unit.trim().is_empty() {
            return Err(RegionGraphError::InvalidRegion {
                field: "coordinate_unit",
            });
        }
        if self.time_unit.trim().is_empty() {
            return Err(RegionGraphError::InvalidRegion { field: "time_unit" });
        }
        if self.clock_id.trim().is_empty() {
            return Err(RegionGraphError::InvalidRegion { field: "clock_id" });
        }
        for axis in 0..D {
            if !self.lower[axis].is_finite() || !self.upper[axis].is_finite() {
                return Err(RegionGraphError::InvalidRegion {
                    field: "non-finite domain bound",
                });
            }
            if self.lower[axis] > self.upper[axis] {
                return Err(RegionGraphError::InvalidRegion {
                    field: "lower bound exceeds upper bound",
                });
            }
        }
        Ok(())
    }

    pub fn contains(&self, point: &SVector<f64, D>, tolerance: f64) -> bool {
        if !tolerance.is_finite() || tolerance < 0.0 {
            return false;
        }
        for axis in 0..D {
            if !point[axis].is_finite()
                || point[axis] < self.lower[axis] - tolerance
                || point[axis] > self.upper[axis] + tolerance
            {
                return false;
            }
        }
        true
    }
}

/// Normalized plane n·x = offset defining one interface boundary chart.
#[derive(Clone, Debug, PartialEq)]
pub struct PlaneBoundary<const D: usize> {
    pub normal: SVector<f64, D>,
    pub offset: f64,
    pub tolerance: f64,
}

impl<const D: usize> PlaneBoundary<D> {
    pub fn new(
        normal: SVector<f64, D>,
        offset: f64,
        tolerance: f64,
    ) -> Result<Self, RegionGraphError> {
        if D == 0 {
            return Err(RegionGraphError::InvalidDimension);
        }
        if !offset.is_finite() {
            return Err(RegionGraphError::InvalidBoundary { field: "offset" });
        }
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(RegionGraphError::InvalidBoundary { field: "tolerance" });
        }
        if !normal.iter().all(|value| value.is_finite()) {
            return Err(RegionGraphError::InvalidBoundary { field: "normal" });
        }
        let length = normal.norm();
        if !length.is_finite() || length <= 1.0e-12 {
            return Err(RegionGraphError::InvalidBoundary {
                field: "zero normal",
            });
        }
        Ok(Self {
            normal: normal / length,
            offset: offset / length,
            tolerance,
        })
    }

    pub fn signed_distance(&self, point: &SVector<f64, D>) -> Option<f64> {
        if !point.iter().all(|value| value.is_finite()) {
            return None;
        }
        let distance = self.normal.dot(point) - self.offset;
        distance.is_finite().then_some(distance)
    }

    pub fn contains(&self, point: &SVector<f64, D>) -> bool {
        self.signed_distance(point)
            .is_some_and(|distance| distance.abs() <= self.tolerance)
    }

    pub fn validate(&self) -> Result<(), RegionGraphError> {
        if D == 0 {
            return Err(RegionGraphError::InvalidDimension);
        }
        if !self.normal.iter().all(|value| value.is_finite())
            || !self.offset.is_finite()
            || !self.tolerance.is_finite()
            || self.tolerance <= 0.0
            || !scalar_close(self.normal.norm(), 1.0, 1e-8)
        {
            return Err(RegionGraphError::InvalidBoundary {
                field: "malformed plane",
            });
        }
        Ok(())
    }
}

/// Conservative test that the normalized interface plane intersects a
/// region's axis-aligned bounds.
fn boundary_intersects_region<const D: usize>(
    boundary: &PlaneBoundary<D>,
    region: &RegionSpec<D>,
) -> bool {
    let mut minimum = 0.0;
    let mut maximum = 0.0;
    for axis in 0..D {
        let normal = boundary.normal[axis];
        if normal >= 0.0 {
            minimum += normal * region.lower[axis];
            maximum += normal * region.upper[axis];
        } else {
            minimum += normal * region.upper[axis];
            maximum += normal * region.lower[axis];
        }
    }
    minimum.is_finite()
        && maximum.is_finite()
        && boundary.offset >= minimum - boundary.tolerance
        && boundary.offset <= maximum + boundary.tolerance
}

/// Affine coordinate and velocity map between same-dimensional local charts.
///
/// Position: x_dst = A x_src + b.
/// Time: t_dst = time_scale * t_src + time_offset.
/// Velocity: v_dst = A v_src / time_scale.
/// time_scale means dt_dst = time_scale * dt_src and must be finite and > 0.
#[derive(Clone, Debug, PartialEq)]
pub struct AffineRegionMap<const D: usize> {
    pub linear: SMatrix<f64, D, D>,
    pub translation: SVector<f64, D>,
    pub time_scale: f64,
    pub time_offset: f64,
}

impl<const D: usize> AffineRegionMap<D> {
    pub fn new(
        linear: SMatrix<f64, D, D>,
        translation: SVector<f64, D>,
        time_scale: f64,
        time_offset: f64,
    ) -> Result<Self, RegionGraphError> {
        if D == 0 {
            return Err(RegionGraphError::InvalidDimension);
        }
        if !linear.iter().all(|value| value.is_finite()) {
            return Err(RegionGraphError::InvalidMap {
                field: "linear matrix",
            });
        }
        if !translation.iter().all(|value| value.is_finite()) {
            return Err(RegionGraphError::InvalidMap {
                field: "translation",
            });
        }
        if !time_scale.is_finite() || time_scale <= 0.0 {
            return Err(RegionGraphError::InvalidMap {
                field: "time_scale",
            });
        }
        if !time_offset.is_finite() {
            return Err(RegionGraphError::InvalidMap {
                field: "time_offset",
            });
        }
        Ok(Self {
            linear,
            translation,
            time_scale,
            time_offset,
        })
    }

    pub fn identity() -> Self {
        Self {
            linear: SMatrix::<f64, D, D>::identity(),
            translation: SVector::<f64, D>::zeros(),
            time_scale: 1.0,
            time_offset: 0.0,
        }
    }

    pub fn map_position(
        &self,
        position: &SVector<f64, D>,
    ) -> Result<SVector<f64, D>, RegionGraphError> {
        if !position.iter().all(|value| value.is_finite()) {
            return Err(RegionGraphError::InvalidMap {
                field: "input position",
            });
        }
        let mapped = self.linear * position + self.translation;
        if !mapped.iter().all(|value| value.is_finite()) {
            return Err(RegionGraphError::InvalidMap {
                field: "mapped position",
            });
        }
        Ok(mapped)
    }

    pub fn map_time(&self, time: f64) -> Result<f64, RegionGraphError> {
        if !time.is_finite() {
            return Err(RegionGraphError::InvalidMap {
                field: "input time",
            });
        }
        let mapped = self.time_scale * time + self.time_offset;
        if !mapped.is_finite() {
            return Err(RegionGraphError::InvalidMap {
                field: "mapped time",
            });
        }
        Ok(mapped)
    }

    pub fn map_velocity(
        &self,
        velocity: &SVector<f64, D>,
    ) -> Result<SVector<f64, D>, RegionGraphError> {
        if !velocity.iter().all(|value| value.is_finite()) {
            return Err(RegionGraphError::InvalidMap {
                field: "input velocity",
            });
        }
        let mapped = self.linear * velocity / self.time_scale;
        if !mapped.iter().all(|value| value.is_finite()) {
            return Err(RegionGraphError::InvalidMap {
                field: "mapped velocity",
            });
        }
        Ok(mapped)
    }

    /// Compose this source-to-intermediate map with an intermediate-to-
    /// destination map. The returned map is equivalent to applying this map
    /// first and the next map second, including the local-clock transform.
    pub fn then(&self, next: &Self) -> Result<Self, RegionGraphError> {
        self.validate()?;
        next.validate()?;
        let linear = next.linear * self.linear;
        let translation = next.linear * self.translation + next.translation;
        let time_scale = next.time_scale * self.time_scale;
        let time_offset = next.time_scale * self.time_offset + next.time_offset;
        Self::new(linear, translation, time_scale, time_offset)
    }

    pub fn inverse(&self) -> Result<Self, RegionGraphError> {
        let linear = self
            .linear
            .try_inverse()
            .ok_or(RegionGraphError::SingularMap)?;
        let translation = -(linear * self.translation);
        Self::new(
            linear,
            translation,
            1.0 / self.time_scale,
            -self.time_offset / self.time_scale,
        )
    }

    pub fn is_isometry(&self, tolerance: f64) -> bool {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return false;
        }
        let gram = self.linear.transpose() * self.linear;
        // A time-origin shift is a coordinate synchronization change, not a
        // change in the time rate; it does not by itself break isometry.
        (gram - SMatrix::<f64, D, D>::identity()).norm() <= tolerance
            && (self.time_scale - 1.0).abs() <= tolerance
    }

    pub fn validate(&self) -> Result<(), RegionGraphError> {
        if D == 0 {
            return Err(RegionGraphError::InvalidDimension);
        }
        if !self.linear.iter().all(|value| value.is_finite()) {
            return Err(RegionGraphError::InvalidMap {
                field: "linear matrix",
            });
        }
        if !self.translation.iter().all(|value| value.is_finite()) {
            return Err(RegionGraphError::InvalidMap {
                field: "translation",
            });
        }
        if !self.time_scale.is_finite() || self.time_scale <= 0.0 {
            return Err(RegionGraphError::InvalidMap {
                field: "time_scale",
            });
        }
        if !self.time_offset.is_finite() {
            return Err(RegionGraphError::InvalidMap {
                field: "time_offset",
            });
        }
        Ok(())
    }

    pub fn validate_inverse(&self, inverse: &Self, tolerance: f64) -> Result<(), RegionGraphError> {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(RegionGraphError::InvalidMap {
                field: "inverse tolerance",
            });
        }
        let expected = self.inverse()?;
        if !matrix_close(&expected.linear, &inverse.linear, tolerance)
            || !vector_close(&expected.translation, &inverse.translation, tolerance)
            || !scalar_close(expected.time_scale, inverse.time_scale, tolerance)
            || !scalar_close(expected.time_offset, inverse.time_offset, tolerance)
        {
            return Err(RegionGraphError::InverseMismatch);
        }
        Ok(())
    }
}

fn scalar_close(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance * a.abs().max(b.abs()).max(1.0)
}

fn vector_close<const D: usize>(a: &SVector<f64, D>, b: &SVector<f64, D>, tolerance: f64) -> bool {
    (a - b).norm() <= tolerance * a.norm().max(b.norm()).max(1.0)
}

fn matrix_close<const R: usize, const C: usize>(
    a: &SMatrix<f64, R, C>,
    b: &SMatrix<f64, R, C>,
    tolerance: f64,
) -> bool {
    (a - b).norm() <= tolerance * a.norm().max(b.norm()).max(1.0)
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegionInterface<const D: usize> {
    pub id: InterfaceId,
    pub source: RegionId,
    pub target: RegionId,
    pub revision: u64,
    pub source_boundary: PlaneBoundary<D>,
    pub target_boundary: PlaneBoundary<D>,
    pub map: AffineRegionMap<D>,
    pub inverse_map: Option<AffineRegionMap<D>>,
    pub reversible: bool,
    pub allowed_direction: CrossingDirection,
    pub momentum_policy: QuantityPolicy,
    pub energy_policy: QuantityPolicy,
    pub transform_identity: Option<String>,
}

impl<const D: usize> RegionInterface<D> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: InterfaceId,
        source: RegionId,
        target: RegionId,
        revision: u64,
        source_boundary: PlaneBoundary<D>,
        target_boundary: PlaneBoundary<D>,
        map: AffineRegionMap<D>,
        inverse_map: Option<AffineRegionMap<D>>,
        reversible: bool,
        allowed_direction: CrossingDirection,
        momentum_policy: QuantityPolicy,
        energy_policy: QuantityPolicy,
        transform_identity: Option<String>,
    ) -> Result<Self, RegionGraphError> {
        if D == 0 {
            return Err(RegionGraphError::InvalidDimension);
        }
        if source == target {
            return Err(RegionGraphError::SelfInterface);
        }
        source_boundary.validate()?;
        target_boundary.validate()?;
        map.validate()?;
        if reversible && inverse_map.is_none() {
            return Err(RegionGraphError::InverseRequired);
        }
        if let Some(inverse) = &inverse_map {
            map.validate_inverse(inverse, DEFAULT_MAP_TOLERANCE)?;
        }
        if (momentum_policy == QuantityPolicy::ExplicitTransform
            || energy_policy == QuantityPolicy::ExplicitTransform)
            && transform_identity
                .as_ref()
                .is_none_or(|identity| identity.trim().is_empty())
        {
            return Err(RegionGraphError::MissingTransformIdentity);
        }

        Ok(Self {
            id,
            source,
            target,
            revision,
            source_boundary,
            target_boundary,
            map,
            inverse_map,
            reversible,
            allowed_direction,
            momentum_policy,
            energy_policy,
            transform_identity,
        })
    }

    /// Revalidate public fields at graph insertion; callers can construct or
    /// deserialize this type without going through the constructor.
    pub fn validate(&self) -> Result<(), RegionGraphError> {
        if D == 0 {
            return Err(RegionGraphError::InvalidDimension);
        }
        if self.source == self.target {
            return Err(RegionGraphError::SelfInterface);
        }
        self.source_boundary.validate()?;
        self.target_boundary.validate()?;
        self.map.validate()?;
        if self.reversible && self.inverse_map.is_none() {
            return Err(RegionGraphError::InverseRequired);
        }
        if let Some(inverse) = &self.inverse_map {
            inverse.validate()?;
            self.map.validate_inverse(inverse, DEFAULT_MAP_TOLERANCE)?;
        }
        if (self.momentum_policy == QuantityPolicy::ExplicitTransform
            || self.energy_policy == QuantityPolicy::ExplicitTransform)
            && self
                .transform_identity
                .as_ref()
                .is_none_or(|identity| identity.trim().is_empty())
        {
            return Err(RegionGraphError::MissingTransformIdentity);
        }
        Ok(())
    }
}

/// Deterministic composition of an explicitly ordered interface path.
/// A mathematically composed map for a caller-declared directed region path.
///
/// This object is not an authorization to skip intermediate boundary checks,
/// crossing policies, or per-interface conservation receipts. It binds the
/// exact interface and region-model revisions used to compose the map.
#[derive(Clone, Debug, PartialEq)]
pub struct ComposedRegionPath<const D: usize> {
    pub source_region: RegionId,
    pub target_region: RegionId,
    pub interface_ids: Vec<InterfaceId>,
    pub interface_revisions: Vec<(InterfaceId, u64)>,
    pub region_model_revisions: Vec<(RegionId, u64)>,
    pub map: AffineRegionMap<D>,
}

impl<const D: usize> ComposedRegionPath<D> {
    /// Reject use after any interface or region model in the path has changed.
    ///
    /// A caller must still execute each interface's own boundary, direction,
    /// quantity-policy and receipt checks when performing a physical crossing.
    pub fn validate_current(
        &self,
        graph: &RegionGraph<D>,
    ) -> Result<(), RegionGraphError> {
        if self.interface_ids.is_empty()
            || self.interface_ids.len() != self.interface_revisions.len()
            || self.region_model_revisions.len() != self.interface_ids.len() + 1
        {
            return Err(RegionGraphError::ComposedPathMismatch);
        }

        for (interface_id, expected_revision) in &self.interface_revisions {
            let interface = graph
                .interface(*interface_id)
                .ok_or(RegionGraphError::UnknownInterface { id: *interface_id })?;
            if interface.revision != *expected_revision {
                return Err(RegionGraphError::StalePathInterfaceRevision {
                    interface: *interface_id,
                    expected: *expected_revision,
                    actual: interface.revision,
                });
            }
        }

        for (region_id, expected_revision) in &self.region_model_revisions {
            let region = graph
                .region(*region_id)
                .ok_or(RegionGraphError::UnknownRegion { id: *region_id })?;
            if region.model_revision != *expected_revision {
                return Err(RegionGraphError::StalePathRegionModelRevision {
                    region: *region_id,
                    expected: *expected_revision,
                    actual: region.model_revision,
                });
            }
        }

        let current = graph.compose_interface_path(&self.interface_ids)?;
        if current.source_region != self.source_region
            || current.target_region != self.target_region
            || current.map != self.map
            || current.interface_revisions != self.interface_revisions
            || current.region_model_revisions != self.region_model_revisions
        {
            return Err(RegionGraphError::ComposedPathMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegionGraph<const D: usize> {
    regions: BTreeMap<RegionId, RegionSpec<D>>,
    interfaces: BTreeMap<InterfaceId, RegionInterface<D>>,
}

impl<const D: usize> Default for RegionGraph<D> {
    fn default() -> Self {
        Self {
            regions: BTreeMap::new(),
            interfaces: BTreeMap::new(),
        }
    }
}

impl<const D: usize> RegionGraph<D> {
    pub fn new() -> Result<Self, RegionGraphError> {
        if D == 0 {
            return Err(RegionGraphError::InvalidDimension);
        }
        Ok(Self::default())
    }

    pub fn add_region(&mut self, region: RegionSpec<D>) -> Result<(), RegionGraphError> {
        region.validate()?;
        if self.regions.contains_key(&region.id) {
            return Err(RegionGraphError::DuplicateRegion { id: region.id });
        }
        self.regions.insert(region.id, region);
        Ok(())
    }

    pub fn add_interface(&mut self, interface: RegionInterface<D>) -> Result<(), RegionGraphError> {
        if self.interfaces.contains_key(&interface.id) {
            return Err(RegionGraphError::DuplicateInterface { id: interface.id });
        }
        interface.validate()?;
        let source =
            self.regions
                .get(&interface.source)
                .ok_or(RegionGraphError::UnknownRegion {
                    id: interface.source,
                })?;
        let target =
            self.regions
                .get(&interface.target)
                .ok_or(RegionGraphError::UnknownRegion {
                    id: interface.target,
                })?;

        if !boundary_intersects_region(&interface.source_boundary, source)
            || !boundary_intersects_region(&interface.target_boundary, target)
        {
            return Err(RegionGraphError::BoundaryOutsideRegion);
        }

        let uses_quantity_ledger = matches!(
            interface.momentum_policy,
            QuantityPolicy::Preserve | QuantityPolicy::Exchange
        ) || matches!(
            interface.energy_policy,
            QuantityPolicy::Preserve | QuantityPolicy::Exchange
        );
        let preserves_quantity = interface.momentum_policy == QuantityPolicy::Preserve
            || interface.energy_policy == QuantityPolicy::Preserve;

        if uses_quantity_ledger {
            if source.coordinate_unit != target.coordinate_unit
                || source.time_unit != target.time_unit
                || source.clock_id != target.clock_id
            {
                return Err(RegionGraphError::UnitMismatchForConservation);
            }
            if source.metric_profile != target.metric_profile {
                return Err(RegionGraphError::MetricMismatchForConservation);
            }
            if source.metric_profile != "euclidean" {
                return Err(RegionGraphError::UnsupportedMetricForConservation);
            }
        }

        // A non-isometric affine map can still be accepted under Exchange:
        // the declared momentum and energy ledger must close at transition
        // time. Preserve has no such reservoir term, so it requires an
        // isometry and unchanged clock rate.
        if preserves_quantity && !interface.map.is_isometry(DEFAULT_MAP_TOLERANCE) {
            return Err(RegionGraphError::NonIsometricMapForPreservation);
        }

        self.interfaces.insert(interface.id, interface);
        Ok(())
    }

    /// Compose a caller-specified interface path in traversal order.
    ///
    /// This does not search for a path or infer a route from coincident local
    /// coordinates. Every adjacent pair must share the same intermediate
    /// RegionId. The returned interface list preserves the exact lineage.
    pub fn compose_interface_path(
        &self,
        interface_ids: &[InterfaceId],
    ) -> Result<ComposedRegionPath<D>, RegionGraphError> {
        let Some(first_id) = interface_ids.first().copied() else {
            return Err(RegionGraphError::EmptyInterfacePath);
        };
        let first = self
            .interface(first_id)
            .ok_or(RegionGraphError::UnknownInterface { id: first_id })?;
        let source_region = first.source;
        let source_model = self
            .region(source_region)
            .ok_or(RegionGraphError::UnknownRegion { id: source_region })?;
        let mut target_region = first.target;
        let mut map = first.map.clone();
        let mut interface_revisions = vec![(first.id, first.revision)];
        let mut region_model_revisions = vec![(source_region, source_model.model_revision)];
        let first_target = self
            .region(target_region)
            .ok_or(RegionGraphError::UnknownRegion { id: target_region })?;
        region_model_revisions.push((target_region, first_target.model_revision));

        for interface_id in interface_ids.iter().copied().skip(1) {
            let next = self
                .interface(interface_id)
                .ok_or(RegionGraphError::UnknownInterface { id: interface_id })?;
            if target_region != next.source {
                return Err(RegionGraphError::DisconnectedInterfacePath {
                    previous_target: target_region,
                    next_source: next.source,
                });
            }
            map = map.then(&next.map)?;
            target_region = next.target;
            interface_revisions.push((next.id, next.revision));
            let target = self
                .region(target_region)
                .ok_or(RegionGraphError::UnknownRegion { id: target_region })?;
            region_model_revisions.push((target_region, target.model_revision));
        }

        Ok(ComposedRegionPath {
            source_region,
            target_region,
            interface_ids: interface_ids.to_vec(),
            interface_revisions,
            region_model_revisions,
            map,
        })
    }

    pub fn region(&self, id: RegionId) -> Option<&RegionSpec<D>> {
        self.regions.get(&id)
    }

    pub fn interface(&self, id: InterfaceId) -> Option<&RegionInterface<D>> {
        self.interfaces.get(&id)
    }

    pub fn regions(&self) -> impl Iterator<Item = &RegionSpec<D>> {
        self.regions.values()
    }

    pub fn interfaces(&self) -> impl Iterator<Item = &RegionInterface<D>> {
        self.interfaces.values()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegionState<const D: usize> {
    pub body_id: u64,
    pub region: RegionId,
    pub revision: u64,
    pub position: SVector<f64, D>,
    pub velocity: SVector<f64, D>,
    /// Time in this region's declared local clock and time unit.
    pub local_time: f64,
    pub mass: f64,
}

impl<const D: usize> RegionState<D> {
    pub fn validate(&self) -> Result<(), RegionTransitionError> {
        if !self.mass.is_finite() || self.mass <= 0.0 {
            return Err(RegionTransitionError::InvalidState { field: "mass" });
        }
        if !self.position.iter().all(|value| value.is_finite()) {
            return Err(RegionTransitionError::InvalidState { field: "position" });
        }
        if !self.velocity.iter().all(|value| value.is_finite()) {
            return Err(RegionTransitionError::InvalidState { field: "velocity" });
        }
        if !self.local_time.is_finite() {
            return Err(RegionTransitionError::InvalidState {
                field: "local_time",
            });
        }
        Ok(())
    }

    pub fn momentum(&self) -> SVector<f64, D> {
        self.velocity * self.mass
    }

    pub fn kinetic_energy(&self) -> f64 {
        0.5 * self.mass * self.velocity.norm_squared()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransitionRequest<const D: usize> {
    pub crossing_id: u64,
    pub body_id: u64,
    pub expected_state_revision: u64,
    pub interface_id: InterfaceId,
    pub expected_interface_revision: u64,
    pub expected_source_model_revision: u64,
    pub expected_target_model_revision: u64,
    pub direction: CrossingDirection,
    /// Exchange impulse expressed as target-region momentum coordinates.
    pub momentum_exchange: SVector<f64, D>,
    /// Declared kinetic-energy exchange in compatible region units.
    pub energy_exchange: f64,
    pub tolerance: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransitionReceipt<const D: usize> {
    pub request: TransitionRequest<D>,
    pub source_region: RegionId,
    pub target_region: RegionId,
    pub source_revision: u64,
    pub target_revision: u64,
    pub interface_revision: u64,
    pub source_model_revision: u64,
    pub target_model_revision: u64,
    pub before: RegionState<D>,
    pub after: RegionState<D>,
    pub momentum_before: SVector<f64, D>,
    pub momentum_after: SVector<f64, D>,
    pub kinetic_energy_before: f64,
    pub kinetic_energy_after: f64,
    pub transform_identity: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TransitionOutcome<const D: usize> {
    Committed(TransitionReceipt<D>),
    AlreadyCommitted(TransitionReceipt<D>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegionTransitionError {
    Graph(RegionGraphError),
    DuplicateBody {
        body_id: u64,
    },
    UnknownBody {
        body_id: u64,
    },
    StaleStateRevision {
        expected: u64,
        actual: u64,
    },
    StaleInterfaceRevision {
        expected: u64,
        actual: u64,
    },
    StaleRegionModelRevision {
        region: RegionId,
        expected: u64,
        actual: u64,
    },
    WrongSourceRegion,
    CrossingDirectionRejected,
    DirectionVelocityMismatch,
    OutsideSourceBoundary,
    OutsideTargetBoundary,
    OutsideTargetDomain,
    InvalidRequest {
        field: &'static str,
    },
    InvalidState {
        field: &'static str,
    },
    PolicyRejected {
        quantity: &'static str,
    },
    ConservationResidual {
        quantity: &'static str,
    },
    CrossingIdConflict {
        crossing_id: u64,
    },
    RevisionOverflow,
}

impl fmt::Display for RegionTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Graph(error) => write!(f, "region graph error: {error}"),
            Self::DuplicateBody { body_id } => write!(f, "duplicate body state {body_id}"),
            Self::UnknownBody { body_id } => write!(f, "unknown body state {body_id}"),
            Self::StaleStateRevision { expected, actual } => {
                write!(
                    f,
                    "stale body state revision: expected {expected}, actual {actual}"
                )
            }
            Self::StaleInterfaceRevision { expected, actual } => {
                write!(
                    f,
                    "stale interface revision: expected {expected}, actual {actual}"
                )
            }
            Self::StaleRegionModelRevision {
                region,
                expected,
                actual,
            } => write!(
                f,
                "stale model revision for region {}: expected {expected}, actual {actual}",
                region.0
            ),
            Self::WrongSourceRegion => write!(
                f,
                "body is not authoritative in the interface source region"
            ),
            Self::CrossingDirectionRejected => {
                write!(f, "crossing direction is not allowed by interface")
            }
            Self::DirectionVelocityMismatch => write!(
                f,
                "source velocity does not agree with the declared crossing direction"
            ),
            Self::OutsideSourceBoundary => {
                write!(f, "source state is outside interface boundary chart")
            }
            Self::OutsideTargetBoundary => {
                write!(f, "mapped state is outside target boundary chart")
            }
            Self::OutsideTargetDomain => write!(f, "mapped state is outside target region domain"),
            Self::InvalidRequest { field } => write!(f, "invalid transition request field {field}"),
            Self::InvalidState { field } => write!(f, "invalid authoritative state field {field}"),
            Self::PolicyRejected { quantity } => {
                write!(f, "transition policy rejects quantity {quantity}")
            }
            Self::ConservationResidual { quantity } => {
                write!(
                    f,
                    "transition conservation residual exceeds tolerance for {quantity}"
                )
            }
            Self::CrossingIdConflict { crossing_id } => {
                write!(
                    f,
                    "crossing ID {crossing_id} was reused for a different request"
                )
            }
            Self::RevisionOverflow => write!(f, "body state revision overflow"),
        }
    }
}

impl Error for RegionTransitionError {}

impl From<RegionGraphError> for RegionTransitionError {
    fn from(value: RegionGraphError) -> Self {
        Self::Graph(value)
    }
}

/// Owns one authoritative state per body plus idempotent transition receipts.
/// PhysicsWorld does not use this store yet; it is a reference for testing
/// transition atomicity and replay semantics before any engine integration.
#[derive(Clone, Debug, PartialEq)]
pub struct RegionStateStore<const D: usize> {
    states: BTreeMap<u64, RegionState<D>>,
    receipts: BTreeMap<u64, TransitionReceipt<D>>,
}

impl<const D: usize> Default for RegionStateStore<D> {
    fn default() -> Self {
        Self {
            states: BTreeMap::new(),
            receipts: BTreeMap::new(),
        }
    }
}

impl<const D: usize> RegionStateStore<D> {
    pub fn new() -> Result<Self, RegionTransitionError> {
        if D == 0 {
            return Err(RegionGraphError::InvalidDimension.into());
        }
        Ok(Self::default())
    }

    pub fn insert_state(
        &mut self,
        graph: &RegionGraph<D>,
        state: RegionState<D>,
    ) -> Result<(), RegionTransitionError> {
        state.validate()?;
        let region = graph
            .region(state.region)
            .ok_or(RegionGraphError::UnknownRegion { id: state.region })?;
        if !region.contains(&state.position, 0.0) {
            return Err(RegionTransitionError::OutsideTargetDomain);
        }
        if self.states.contains_key(&state.body_id) {
            return Err(RegionTransitionError::DuplicateBody {
                body_id: state.body_id,
            });
        }
        self.states.insert(state.body_id, state);
        Ok(())
    }

    pub fn state(&self, body_id: u64) -> Option<&RegionState<D>> {
        self.states.get(&body_id)
    }

    pub fn receipt(&self, crossing_id: u64) -> Option<&TransitionReceipt<D>> {
        self.receipts.get(&crossing_id)
    }

    pub fn receipts(&self) -> impl Iterator<Item = &TransitionReceipt<D>> {
        self.receipts.values()
    }

    /// Apply a region transition as an atomic authority transfer.
    ///
    /// The crossing ID is idempotent only for an exactly identical request.
    /// All source/target, mapping, policy, and conservation checks complete
    /// before either authoritative state or the receipt ledger is mutated.
    pub fn transition(
        &mut self,
        graph: &RegionGraph<D>,
        request: TransitionRequest<D>,
    ) -> Result<TransitionOutcome<D>, RegionTransitionError> {
        if let Some(receipt) = self.receipts.get(&request.crossing_id) {
            return if receipt.request == request {
                Ok(TransitionOutcome::AlreadyCommitted(receipt.clone()))
            } else {
                Err(RegionTransitionError::CrossingIdConflict {
                    crossing_id: request.crossing_id,
                })
            };
        }

        if !request
            .momentum_exchange
            .iter()
            .all(|value| value.is_finite())
        {
            return Err(RegionTransitionError::InvalidRequest {
                field: "momentum_exchange",
            });
        }
        if !request.energy_exchange.is_finite() {
            return Err(RegionTransitionError::InvalidRequest {
                field: "energy_exchange",
            });
        }
        if !request.tolerance.is_finite() || request.tolerance <= 0.0 {
            return Err(RegionTransitionError::InvalidRequest { field: "tolerance" });
        }

        let before = self
            .states
            .get(&request.body_id)
            .ok_or(RegionTransitionError::UnknownBody {
                body_id: request.body_id,
            })?
            .clone();
        before.validate()?;

        if before.revision != request.expected_state_revision {
            return Err(RegionTransitionError::StaleStateRevision {
                expected: request.expected_state_revision,
                actual: before.revision,
            });
        }

        let interface =
            graph
                .interface(request.interface_id)
                .ok_or(RegionGraphError::UnknownInterface {
                    id: request.interface_id,
                })?;
        if interface.revision != request.expected_interface_revision {
            return Err(RegionTransitionError::StaleInterfaceRevision {
                expected: request.expected_interface_revision,
                actual: interface.revision,
            });
        }
        if before.region != interface.source {
            return Err(RegionTransitionError::WrongSourceRegion);
        }
        if request.direction == CrossingDirection::Either {
            return Err(RegionTransitionError::InvalidRequest {
                field: "direction must identify an observed crossing direction",
            });
        }
        if interface.allowed_direction != CrossingDirection::Either
            && interface.allowed_direction != request.direction
        {
            return Err(RegionTransitionError::CrossingDirectionRejected);
        }
        if !interface.source_boundary.contains(&before.position) {
            return Err(RegionTransitionError::OutsideSourceBoundary);
        }
        // Crossing direction is not merely metadata: for this reference
        // contract, the source-chart normal velocity must point through the
        // interface in the declared direction. Zero/tangent velocity is
        // ambiguous and must be resolved by a caller with crossing history.
        let normal_speed = interface.source_boundary.normal.dot(&before.velocity);
        if !normal_speed.is_finite()
            || (request.direction == CrossingDirection::NegativeToPositive && normal_speed <= 0.0)
            || (request.direction == CrossingDirection::PositiveToNegative && normal_speed >= 0.0)
        {
            return Err(RegionTransitionError::DirectionVelocityMismatch);
        }

        let source_region =
            graph
                .region(interface.source)
                .ok_or(RegionGraphError::UnknownRegion {
                    id: interface.source,
                })?;
        let target_region =
            graph
                .region(interface.target)
                .ok_or(RegionGraphError::UnknownRegion {
                    id: interface.target,
                })?;
        for (region, expected) in [
            (source_region, request.expected_source_model_revision),
            (target_region, request.expected_target_model_revision),
        ] {
            if region.model_revision != expected {
                return Err(RegionTransitionError::StaleRegionModelRevision {
                    region: region.id,
                    expected,
                    actual: region.model_revision,
                });
            }
        }
        let mapped_position = interface.map.map_position(&before.position)?;
        let mut mapped_velocity = interface.map.map_velocity(&before.velocity)?;
        let mapped_time = interface.map.map_time(before.local_time)?;

        if !interface.target_boundary.contains(&mapped_position) {
            return Err(RegionTransitionError::OutsideTargetBoundary);
        }
        if !target_region.contains(&mapped_position, request.tolerance) {
            return Err(RegionTransitionError::OutsideTargetDomain);
        }

        match interface.momentum_policy {
            QuantityPolicy::Reject => {
                return Err(RegionTransitionError::PolicyRejected {
                    quantity: "momentum",
                });
            }
            QuantityPolicy::Preserve | QuantityPolicy::ExplicitTransform => {
                if request.momentum_exchange.iter().any(|value| *value != 0.0) {
                    return Err(RegionTransitionError::InvalidRequest {
                        field: "momentum_exchange requires Exchange policy",
                    });
                }
            }
            QuantityPolicy::Exchange => {
                mapped_velocity += request.momentum_exchange / before.mass;
                if !mapped_velocity.iter().all(|value| value.is_finite()) {
                    return Err(RegionTransitionError::InvalidState {
                        field: "velocity after momentum exchange",
                    });
                }
            }
        }

        if interface.energy_policy == QuantityPolicy::Reject {
            return Err(RegionTransitionError::PolicyRejected { quantity: "energy" });
        }
        if interface.energy_policy != QuantityPolicy::Exchange && request.energy_exchange != 0.0 {
            return Err(RegionTransitionError::InvalidRequest {
                field: "energy_exchange requires Exchange policy",
            });
        }

        let target_revision = before
            .revision
            .checked_add(1)
            .ok_or(RegionTransitionError::RevisionOverflow)?;
        let after = RegionState {
            body_id: before.body_id,
            region: interface.target,
            revision: target_revision,
            position: mapped_position,
            velocity: mapped_velocity,
            local_time: mapped_time,
            mass: before.mass,
        };
        after.validate()?;

        let momentum_before = before.momentum();
        let momentum_after = after.momentum();
        let kinetic_energy_before = before.kinetic_energy();
        let kinetic_energy_after = after.kinetic_energy();
        if !momentum_before.iter().all(|value| value.is_finite())
            || !momentum_after.iter().all(|value| value.is_finite())
            || !kinetic_energy_before.is_finite()
            || !kinetic_energy_after.is_finite()
        {
            return Err(RegionTransitionError::InvalidState {
                field: "transition diagnostics",
            });
        }

        match interface.momentum_policy {
            QuantityPolicy::Preserve => {
                let mapped_momentum =
                    interface.map.linear * momentum_before / interface.map.time_scale;
                if !vector_close(&momentum_after, &mapped_momentum, request.tolerance)
                    || !scalar_close(
                        momentum_after.norm(),
                        momentum_before.norm(),
                        request.tolerance,
                    )
                {
                    return Err(RegionTransitionError::ConservationResidual {
                        quantity: "linear momentum",
                    });
                }
            }
            QuantityPolicy::Exchange => {
                let mapped_momentum =
                    interface.map.linear * momentum_before / interface.map.time_scale;
                let expected = mapped_momentum + request.momentum_exchange;
                if !vector_close(&momentum_after, &expected, request.tolerance) {
                    return Err(RegionTransitionError::ConservationResidual {
                        quantity: "linear momentum exchange",
                    });
                }
            }
            QuantityPolicy::ExplicitTransform => {}
            QuantityPolicy::Reject => unreachable!("rejected above"),
        }

        match interface.energy_policy {
            QuantityPolicy::Preserve => {
                if !scalar_close(
                    kinetic_energy_after,
                    kinetic_energy_before,
                    request.tolerance,
                ) {
                    return Err(RegionTransitionError::ConservationResidual {
                        quantity: "kinetic energy",
                    });
                }
            }
            QuantityPolicy::Exchange => {
                let expected = kinetic_energy_before + request.energy_exchange;
                if !expected.is_finite()
                    || !scalar_close(kinetic_energy_after, expected, request.tolerance)
                {
                    return Err(RegionTransitionError::ConservationResidual {
                        quantity: "kinetic energy exchange",
                    });
                }
            }
            QuantityPolicy::ExplicitTransform => {}
            QuantityPolicy::Reject => unreachable!("rejected above"),
        }

        let receipt = TransitionReceipt {
            request: request.clone(),
            source_region: before.region,
            target_region: after.region,
            source_revision: before.revision,
            target_revision,
            interface_revision: interface.revision,
            source_model_revision: source_region.model_revision,
            target_model_revision: target_region.model_revision,
            before,
            after: after.clone(),
            momentum_before,
            momentum_after,
            kinetic_energy_before,
            kinetic_energy_after,
            transform_identity: interface.transform_identity.clone(),
        };

        // Commit operations below are infallible map replacements/inserts.
        self.states.insert(request.body_id, after);
        self.receipts.insert(request.crossing_id, receipt.clone());

        Ok(TransitionOutcome::Committed(receipt))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AffineRegionMap, CrossingDirection, InterfaceId, PlaneBoundary, QuantityPolicy,
        RegionGraph, RegionGraphError, RegionId, RegionInterface, RegionSpec, RegionState,
        RegionStateStore, RegionTransitionError, TransitionOutcome, TransitionRequest,
    };
    use nalgebra::{SMatrix, SVector};

    fn region(id: u64, lower: [f64; 2], upper: [f64; 2]) -> RegionSpec<2> {
        RegionSpec {
            id: RegionId(id),
            lower: SVector::from(lower),
            upper: SVector::from(upper),
            metric_profile: "euclidean".to_string(),
            coordinate_unit: "m".to_string(),
            time_unit: "s".to_string(),
            clock_id: "sim-clock".to_string(),
            model_revision: 1,
        }
    }

    fn boundary_x0() -> PlaneBoundary<2> {
        PlaneBoundary::new(SVector::from([1.0, 0.0]), 0.0, 1e-8).unwrap()
    }

    fn graph_identity() -> RegionGraph<2> {
        let mut graph = RegionGraph::<2>::new().unwrap();
        graph
            .add_region(region(1, [-1.0, -1.0], [0.0, 1.0]))
            .unwrap();
        graph
            .add_region(region(2, [0.0, -1.0], [1.0, 1.0]))
            .unwrap();
        let map = AffineRegionMap::<2>::identity();
        let inverse = map.inverse().unwrap();
        let interface = RegionInterface::new(
            InterfaceId(10),
            RegionId(1),
            RegionId(2),
            1,
            boundary_x0(),
            boundary_x0(),
            map,
            Some(inverse),
            true,
            CrossingDirection::NegativeToPositive,
            QuantityPolicy::Preserve,
            QuantityPolicy::Preserve,
            None,
        )
        .unwrap();
        graph.add_interface(interface).unwrap();
        graph
    }

    fn state() -> RegionState<2> {
        RegionState {
            body_id: 99,
            region: RegionId(1),
            revision: 7,
            position: SVector::from([0.0, 0.25]),
            velocity: SVector::from([1.0, 0.0]),
            local_time: 12.0,
            mass: 2.0,
        }
    }

    fn request(crossing_id: u64) -> TransitionRequest<2> {
        TransitionRequest {
            crossing_id,
            body_id: 99,
            expected_state_revision: 7,
            interface_id: InterfaceId(10),
            expected_interface_revision: 1,
            expected_source_model_revision: 1,
            expected_target_model_revision: 1,
            direction: CrossingDirection::NegativeToPositive,
            momentum_exchange: SVector::zeros(),
            energy_exchange: 0.0,
            tolerance: 1e-9,
        }
    }

    #[test]
    fn identity_transition_is_atomic_and_replay_is_idempotent() {
        let graph = graph_identity();
        let mut store = RegionStateStore::new().unwrap();
        store.insert_state(&graph, state()).unwrap();

        let result = store.transition(&graph, request(100)).unwrap();
        let receipt = match result {
            TransitionOutcome::Committed(receipt) => receipt,
            _ => panic!("first transition must commit"),
        };
        assert_eq!(receipt.source_region, RegionId(1));
        assert_eq!(receipt.target_region, RegionId(2));
        assert_eq!(receipt.source_revision, 7);
        assert_eq!(receipt.target_revision, 8);
        assert_eq!(store.state(99).unwrap(), &receipt.after);
        assert_eq!(receipt.after.position, SVector::from([0.0, 0.25]));
        assert_eq!(receipt.after.velocity, SVector::from([1.0, 0.0]));
        assert_eq!(receipt.after.local_time, 12.0);
        assert_eq!(receipt.source_model_revision, 1);
        assert_eq!(receipt.target_model_revision, 1);

        assert_eq!(
            store.transition(&graph, request(100)).unwrap(),
            TransitionOutcome::AlreadyCommitted(receipt.clone())
        );
        assert_eq!(store.receipts().count(), 1);
        assert_eq!(store.state(99).unwrap().revision, 8);
    }

    #[test]
    fn reusing_crossing_id_for_different_request_is_rejected() {
        let graph = graph_identity();
        let mut store = RegionStateStore::new().unwrap();
        store.insert_state(&graph, state()).unwrap();
        store.transition(&graph, request(101)).unwrap();

        let mut conflict = request(101);
        conflict.energy_exchange = 1.0;
        assert_eq!(
            store.transition(&graph, conflict),
            Err(RegionTransitionError::CrossingIdConflict { crossing_id: 101 })
        );
        assert_eq!(store.state(99).unwrap().revision, 8);
    }

    #[test]
    fn rejected_direction_and_stale_revisions_do_not_mutate_state() {
        let graph = graph_identity();
        let mut store = RegionStateStore::new().unwrap();
        store.insert_state(&graph, state()).unwrap();

        let mut wrong_direction = request(102);
        wrong_direction.direction = CrossingDirection::PositiveToNegative;
        assert_eq!(
            store.transition(&graph, wrong_direction),
            Err(RegionTransitionError::CrossingDirectionRejected)
        );

        let mut stale = request(103);
        stale.expected_state_revision = 6;
        assert_eq!(
            store.transition(&graph, stale),
            Err(RegionTransitionError::StaleStateRevision {
                expected: 6,
                actual: 7
            })
        );
        assert_eq!(store.state(99).unwrap().revision, 7);
        assert_eq!(store.receipts().count(), 0);
    }

    #[test]
    fn stale_target_model_revision_fails_before_state_mutation() {
        let original = graph_identity();
        let mut changed = RegionGraph::<2>::new().unwrap();
        changed
            .add_region(region(1, [-1.0, -1.0], [0.0, 1.0]))
            .unwrap();
        let mut target = region(2, [0.0, -1.0], [1.0, 1.0]);
        target.model_revision = 2;
        changed.add_region(target).unwrap();

        let map = AffineRegionMap::<2>::identity();
        let inverse = map.inverse().unwrap();
        changed
            .add_interface(
                RegionInterface::new(
                    InterfaceId(10),
                    RegionId(1),
                    RegionId(2),
                    1,
                    boundary_x0(),
                    boundary_x0(),
                    map,
                    Some(inverse),
                    true,
                    CrossingDirection::NegativeToPositive,
                    QuantityPolicy::Preserve,
                    QuantityPolicy::Preserve,
                    None,
                )
                .unwrap(),
            )
            .unwrap();

        let mut store = RegionStateStore::new().unwrap();
        store.insert_state(&original, state()).unwrap();
        assert_eq!(
            store.transition(&changed, request(110)),
            Err(RegionTransitionError::StaleRegionModelRevision {
                region: RegionId(2),
                expected: 1,
                actual: 2,
            })
        );
        assert_eq!(store.state(99).unwrap(), &state());
        assert_eq!(store.receipts().count(), 0);
    }

    #[test]
    fn direction_must_match_velocity_through_interface() {
        let graph = graph_identity();
        let mut store = RegionStateStore::new().unwrap();
        let mut moving_away = state();
        moving_away.velocity = SVector::from([-1.0, 0.0]);
        store.insert_state(&graph, moving_away.clone()).unwrap();

        assert_eq!(
            store.transition(&graph, request(108)),
            Err(RegionTransitionError::DirectionVelocityMismatch)
        );
        assert_eq!(store.state(99).unwrap(), &moving_away);
        assert_eq!(store.receipts().count(), 0);
    }

    #[test]
    fn preserve_policy_allows_time_origin_shift_with_same_clock_rate() {
        let mut graph = RegionGraph::<2>::new().unwrap();
        graph
            .add_region(region(1, [-1.0, -1.0], [0.0, 1.0]))
            .unwrap();
        graph
            .add_region(region(2, [0.0, -1.0], [1.0, 1.0]))
            .unwrap();

        let map =
            AffineRegionMap::new(SMatrix::<f64, 2, 2>::identity(), SVector::zeros(), 1.0, 5.0)
                .unwrap();
        let inverse = map.inverse().unwrap();
        let interface = RegionInterface::new(
            InterfaceId(16),
            RegionId(1),
            RegionId(2),
            1,
            boundary_x0(),
            boundary_x0(),
            map,
            Some(inverse),
            true,
            CrossingDirection::NegativeToPositive,
            QuantityPolicy::Preserve,
            QuantityPolicy::Preserve,
            None,
        )
        .unwrap();
        graph.add_interface(interface).unwrap();

        let mut store = RegionStateStore::new().unwrap();
        store.insert_state(&graph, state()).unwrap();
        let mut transition_request = request(109);
        transition_request.interface_id = InterfaceId(16);
        let receipt = match store.transition(&graph, transition_request).unwrap() {
            TransitionOutcome::Committed(receipt) => receipt,
            _ => panic!("first transition must commit"),
        };
        assert_eq!(receipt.after.local_time, 17.0);
        assert_eq!(receipt.after.velocity, SVector::from([1.0, 0.0]));
        assert!((receipt.kinetic_energy_after - receipt.kinetic_energy_before).abs() < 1e-12);
    }

    #[test]
    fn preserve_policy_rejects_non_isometric_map() {
        let mut graph = RegionGraph::<2>::new().unwrap();
        graph
            .add_region(region(1, [-1.0, -1.0], [0.0, 1.0]))
            .unwrap();
        graph
            .add_region(region(2, [0.0, -1.0], [2.0, 1.0]))
            .unwrap();

        let scale = SMatrix::<f64, 2, 2>::from_diagonal(&SVector::from([2.0, 1.0]));
        let map = AffineRegionMap::new(scale, SVector::zeros(), 1.0, 0.0).unwrap();
        let interface = RegionInterface::new(
            InterfaceId(11),
            RegionId(1),
            RegionId(2),
            1,
            boundary_x0(),
            boundary_x0(),
            map,
            None,
            false,
            CrossingDirection::Either,
            QuantityPolicy::Preserve,
            QuantityPolicy::Preserve,
            None,
        )
        .unwrap();

        assert_eq!(
            graph.add_interface(interface),
            Err(RegionGraphError::NonIsometricMapForPreservation)
        );
    }

    #[test]
    fn exchange_policy_accepts_non_isometric_map_when_ledgers_close() {
        let mut graph = RegionGraph::<2>::new().unwrap();
        graph
            .add_region(region(1, [-1.0, -1.0], [0.0, 1.0]))
            .unwrap();
        graph
            .add_region(region(2, [0.0, -1.0], [2.0, 1.0]))
            .unwrap();

        let scale = SMatrix::<f64, 2, 2>::from_diagonal(&SVector::from([2.0, 1.0]));
        let map = AffineRegionMap::new(scale, SVector::zeros(), 1.0, 0.0).unwrap();
        let interface = RegionInterface::new(
            InterfaceId(17),
            RegionId(1),
            RegionId(2),
            1,
            boundary_x0(),
            boundary_x0(),
            map,
            None,
            false,
            CrossingDirection::Either,
            QuantityPolicy::Exchange,
            QuantityPolicy::Exchange,
            None,
        )
        .unwrap();

        graph.add_interface(interface).unwrap();
        let mut store = RegionStateStore::new().unwrap();
        store.insert_state(&graph, state()).unwrap();

        let mut transition_request = request(111);
        transition_request.interface_id = InterfaceId(17);
        transition_request.direction = CrossingDirection::NegativeToPositive;
        transition_request.energy_exchange = 3.0;
        let receipt = match store.transition(&graph, transition_request).unwrap() {
            TransitionOutcome::Committed(receipt) => receipt,
            _ => panic!("first transition must commit"),
        };

        assert_eq!(receipt.after.velocity, SVector::from([2.0, 0.0]));
        assert_eq!(receipt.momentum_before, SVector::from([2.0, 0.0]));
        assert_eq!(receipt.momentum_after, SVector::from([4.0, 0.0]));
        assert!((receipt.kinetic_energy_before - 1.0).abs() < 1e-12);
        assert!((receipt.kinetic_energy_after - 4.0).abs() < 1e-12);
        assert!((receipt.kinetic_energy_after - receipt.kinetic_energy_before - 3.0).abs() < 1e-12);
    }

    #[test]
    fn conservation_ledger_rejects_metric_profile_mismatch() {
        let mut graph = RegionGraph::<2>::new().unwrap();
        graph
            .add_region(region(1, [-1.0, -1.0], [0.0, 1.0]))
            .unwrap();
        let mut target = region(2, [0.0, -1.0], [1.0, 1.0]);
        target.metric_profile = "different-metric".to_string();
        graph.add_region(target).unwrap();

        let map = AffineRegionMap::<2>::identity();
        let interface = RegionInterface::new(
            InterfaceId(18),
            RegionId(1),
            RegionId(2),
            1,
            boundary_x0(),
            boundary_x0(),
            map,
            None,
            false,
            CrossingDirection::Either,
            QuantityPolicy::Preserve,
            QuantityPolicy::Preserve,
            None,
        )
        .unwrap();
        assert_eq!(
            graph.add_interface(interface),
            Err(RegionGraphError::MetricMismatchForConservation)
        );
    }

    #[test]
    fn invalid_target_boundary_fails_without_partial_authority_transfer() {
        let mut graph = RegionGraph::<2>::new().unwrap();
        graph
            .add_region(region(1, [-1.0, -1.0], [0.0, 1.0]))
            .unwrap();
        graph
            .add_region(region(2, [0.0, -1.0], [1.0, 1.0]))
            .unwrap();
        let mut translated = SVector::zeros();
        translated[0] = 0.25;
        let map = AffineRegionMap::new(SMatrix::identity(), translated, 1.0, 0.0).unwrap();
        let interface = RegionInterface::new(
            InterfaceId(12),
            RegionId(1),
            RegionId(2),
            1,
            boundary_x0(),
            boundary_x0(),
            map,
            None,
            false,
            CrossingDirection::Either,
            QuantityPolicy::ExplicitTransform,
            QuantityPolicy::ExplicitTransform,
            Some("portal-v1".to_string()),
        )
        .unwrap();
        graph.add_interface(interface).unwrap();

        let mut store = RegionStateStore::new().unwrap();
        store.insert_state(&graph, state()).unwrap();
        let result = store.transition(&graph, {
            let mut req = request(104);
            req.interface_id = InterfaceId(12);
            req.direction = CrossingDirection::NegativeToPositive;
            req.momentum_exchange = SVector::zeros();
            req
        });
        assert_eq!(result, Err(RegionTransitionError::OutsideTargetBoundary));
        assert_eq!(store.state(99).unwrap(), &state());
        assert_eq!(store.receipts().count(), 0);
    }

    #[test]
    fn momentum_and_energy_exchange_must_close_the_same_transition() {
        let mut graph = RegionGraph::<2>::new().unwrap();
        graph
            .add_region(region(1, [-1.0, -1.0], [0.0, 1.0]))
            .unwrap();
        graph
            .add_region(region(2, [0.0, -1.0], [1.0, 1.0]))
            .unwrap();
        let map = AffineRegionMap::<2>::identity();
        let interface = RegionInterface::new(
            InterfaceId(13),
            RegionId(1),
            RegionId(2),
            1,
            boundary_x0(),
            boundary_x0(),
            map,
            None,
            false,
            CrossingDirection::Either,
            QuantityPolicy::Exchange,
            QuantityPolicy::Exchange,
            None,
        )
        .unwrap();
        graph.add_interface(interface).unwrap();

        let mut store = RegionStateStore::new().unwrap();
        store.insert_state(&graph, state()).unwrap();

        let mut request_bad = request(105);
        request_bad.interface_id = InterfaceId(13);
        request_bad.direction = CrossingDirection::NegativeToPositive;
        request_bad.momentum_exchange = SVector::from([2.0, 0.0]);
        request_bad.energy_exchange = 1.0; // target v=2, KE rises from 1 to 4: delta=3
        assert_eq!(
            store.transition(&graph, request_bad),
            Err(RegionTransitionError::ConservationResidual {
                quantity: "kinetic energy exchange"
            })
        );
        assert_eq!(store.state(99).unwrap(), &state());

        let mut request_good = request(106);
        request_good.interface_id = InterfaceId(13);
        request_good.direction = CrossingDirection::NegativeToPositive;
        request_good.momentum_exchange = SVector::from([2.0, 0.0]);
        request_good.energy_exchange = 3.0;
        assert!(matches!(
            store.transition(&graph, request_good),
            Ok(TransitionOutcome::Committed(_))
        ));
        assert_eq!(store.state(99).unwrap().velocity, SVector::from([2.0, 0.0]));
    }

    #[test]
    fn affine_inverse_roundtrips_position_and_velocity() {
        let linear = SMatrix::<f64, 2, 2>::new(0.0, -1.0, 1.0, 0.0);
        let map = AffineRegionMap::new(linear, SVector::from([3.0, -2.0]), 2.0, 5.0).unwrap();
        let inverse = map.inverse().unwrap();
        map.validate_inverse(&inverse, 1e-12).unwrap();

        let position = SVector::from([4.0, 5.0]);
        let velocity = SVector::from([2.0, -1.0]);
        let local_time = 12.0;
        let mapped_position = map.map_position(&position).unwrap();
        let mapped_velocity = map.map_velocity(&velocity).unwrap();
        let mapped_time = map.map_time(local_time).unwrap();
        let restored_position = inverse.map_position(&mapped_position).unwrap();
        let restored_velocity = inverse.map_velocity(&mapped_velocity).unwrap();
        let restored_time = inverse.map_time(mapped_time).unwrap();
        assert!((restored_position - position).norm() < 1e-12);
        assert!((restored_velocity - velocity).norm() < 1e-12);
        assert!((restored_time - local_time).abs() < 1e-12);
    }

    #[test]
    fn explicit_time_map_transforms_local_clock_and_velocity() {
        let mut graph = RegionGraph::<2>::new().unwrap();
        let mut source = region(1, [-1.0, -1.0], [0.0, 1.0]);
        source.clock_id = "source-clock".to_string();
        let mut target = region(2, [0.0, -1.0], [1.0, 1.0]);
        target.clock_id = "target-clock".to_string();
        graph.add_region(source).unwrap();
        graph.add_region(target).unwrap();

        let map =
            AffineRegionMap::new(SMatrix::<f64, 2, 2>::identity(), SVector::zeros(), 2.0, 5.0)
                .unwrap();
        let interface = RegionInterface::new(
            InterfaceId(15),
            RegionId(1),
            RegionId(2),
            3,
            boundary_x0(),
            boundary_x0(),
            map,
            None,
            false,
            CrossingDirection::Either,
            QuantityPolicy::ExplicitTransform,
            QuantityPolicy::ExplicitTransform,
            Some("clock-transform-v1".to_string()),
        )
        .unwrap();
        graph.add_interface(interface).unwrap();

        let mut store = RegionStateStore::new().unwrap();
        store.insert_state(&graph, state()).unwrap();
        let mut request = request(107);
        request.interface_id = InterfaceId(15);
        request.expected_interface_revision = 3;
        request.direction = CrossingDirection::NegativeToPositive;

        let receipt = match store.transition(&graph, request).unwrap() {
            TransitionOutcome::Committed(receipt) => receipt,
            _ => panic!("first transition must commit"),
        };
        assert_eq!(receipt.after.local_time, 29.0);
        assert_eq!(receipt.after.velocity, SVector::from([0.5, 0.0]));
        assert_eq!(
            receipt.transform_identity.as_deref(),
            Some("clock-transform-v1")
        );
        assert!((receipt.kinetic_energy_before - 1.0).abs() < 1e-12);
        assert!((receipt.kinetic_energy_after - 0.25).abs() < 1e-12);
    }

    #[test]
    fn declared_interface_path_composes_only_contiguous_region_edges() {
        let mut graph = RegionGraph::<2>::new().unwrap();
        graph
            .add_region(region(1, [-1.0, -1.0], [0.0, 1.0]))
            .unwrap();
        graph
            .add_region(region(2, [0.0, -1.0], [1.0, 1.0]))
            .unwrap();
        graph
            .add_region(region(3, [0.0, -2.0], [2.0, 2.0]))
            .unwrap();

        let first_map = AffineRegionMap::<2>::identity();
        let first = RegionInterface::new(
            InterfaceId(20),
            RegionId(1),
            RegionId(2),
            1,
            boundary_x0(),
            boundary_x0(),
            first_map.clone(),
            Some(first_map.inverse().unwrap()),
            true,
            CrossingDirection::Either,
            QuantityPolicy::Preserve,
            QuantityPolicy::Preserve,
            None,
        )
        .unwrap();
        graph.add_interface(first).unwrap();

        let second_linear = SMatrix::<f64, 2, 2>::from_diagonal(&SVector::from([2.0, 1.0]));
        let second_map = AffineRegionMap::new(second_linear, SVector::zeros(), 1.5, 2.0).unwrap();
        let second = RegionInterface::new(
            InterfaceId(21),
            RegionId(2),
            RegionId(3),
            1,
            boundary_x0(),
            boundary_x0(),
            second_map.clone(),
            None,
            false,
            CrossingDirection::Either,
            QuantityPolicy::ExplicitTransform,
            QuantityPolicy::ExplicitTransform,
            Some("scale-clock-v1".to_string()),
        )
        .unwrap();
        graph.add_interface(second).unwrap();

        let path = graph
            .compose_interface_path(&[InterfaceId(20), InterfaceId(21)])
            .unwrap();
        assert_eq!(path.source_region, RegionId(1));
        assert_eq!(path.target_region, RegionId(3));
        assert_eq!(path.interface_ids, vec![InterfaceId(20), InterfaceId(21)]);

        let position = SVector::from([0.0, 0.25]);
        let velocity = SVector::from([1.0, -2.0]);
        assert!(
            (path.map.map_position(&position).unwrap()
                - second_map.map_position(&position).unwrap())
            .norm()
                < 1e-12
        );
        assert!(
            (path.map.map_velocity(&velocity).unwrap()
                - second_map.map_velocity(&velocity).unwrap())
            .norm()
                < 1e-12
        );
        assert!(
            (path.map.map_time(7.0).unwrap() - second_map.map_time(7.0).unwrap()).abs() < 1e-12
        );

        assert_eq!(
            graph.compose_interface_path(&[]),
            Err(RegionGraphError::EmptyInterfacePath)
        );
        assert_eq!(
            graph.compose_interface_path(&[InterfaceId(20), InterfaceId(20)]),
            Err(RegionGraphError::DisconnectedInterfacePath {
                previous_target: RegionId(2),
                next_source: RegionId(1),
            })
        );
    }

    #[test]
    fn composed_interface_path_binds_all_model_and_interface_revisions() {
        let mut graph = RegionGraph::<2>::new().unwrap();
        graph.add_region(region(1, [-1.0, -1.0], [0.0, 1.0])).unwrap();
        graph.add_region(region(2, [0.0, -1.0], [1.0, 1.0])).unwrap();
        graph.add_region(region(3, [1.0, -1.0], [2.0, 1.0])).unwrap();

        let first_map = AffineRegionMap::<2>::identity();
        let second_map = AffineRegionMap::<2>::identity();
        let first = RegionInterface::new(
            InterfaceId(30),
            RegionId(1),
            RegionId(2),
            4,
            boundary_x0(),
            boundary_x0(),
            first_map,
            None,
            false,
            CrossingDirection::Either,
            QuantityPolicy::ExplicitTransform,
            QuantityPolicy::ExplicitTransform,
            Some("region-map-a".to_string()),
        )
        .unwrap();
        let second = RegionInterface::new(
            InterfaceId(31),
            RegionId(2),
            RegionId(3),
            7,
            PlaneBoundary::new(SVector::from([1.0, 0.0]), 1.0, 1e-8).unwrap(),
            PlaneBoundary::new(SVector::from([1.0, 0.0]), 1.0, 1e-8).unwrap(),
            second_map,
            None,
            false,
            CrossingDirection::Either,
            QuantityPolicy::ExplicitTransform,
            QuantityPolicy::ExplicitTransform,
            Some("region-map-b".to_string()),
        )
        .unwrap();
        graph.add_interface(first).unwrap();
        graph.add_interface(second).unwrap();

        let path = graph
            .compose_interface_path(&[InterfaceId(30), InterfaceId(31)])
            .unwrap();
        assert_eq!(
            path.interface_revisions,
            vec![(InterfaceId(30), 4), (InterfaceId(31), 7)]
        );
        assert_eq!(
            path.region_model_revisions,
            vec![(RegionId(1), 1), (RegionId(2), 1), (RegionId(3), 1)]
        );
        path.validate_current(&graph).unwrap();

        let mut changed_target = region(3, [1.0, -1.0], [2.0, 1.0]);
        changed_target.model_revision = 2;
        let mut rebuilt = RegionGraph::<2>::new().unwrap();
        rebuilt
            .add_region(region(1, [-1.0, -1.0], [0.0, 1.0]))
            .unwrap();
        rebuilt
            .add_region(region(2, [0.0, -1.0], [1.0, 1.0]))
            .unwrap();
        rebuilt.add_region(changed_target).unwrap();
        rebuilt
            .add_interface(
                graph
                    .interface(InterfaceId(30))
                    .unwrap()
                    .clone(),
            )
            .unwrap();
        rebuilt
            .add_interface(
                graph
                    .interface(InterfaceId(31))
                    .unwrap()
                    .clone(),
            )
            .unwrap();

        assert_eq!(
            path.validate_current(&rebuilt),
            Err(RegionGraphError::StalePathRegionModelRevision {
                region: RegionId(3),
                expected: 1,
                actual: 2,
            })
        );
    }

    #[test]
    fn affine_map_composition_matches_sequential_position_velocity_and_clock() {
        let first_linear = SMatrix::<f64, 2, 2>::new(2.0, 0.0, 0.0, 1.0);
        let first =
            AffineRegionMap::new(first_linear, SVector::from([1.0, -2.0]), 2.0, 3.0).unwrap();
        let second_linear = SMatrix::<f64, 2, 2>::new(0.0, -1.0, 1.0, 0.0);
        let second =
            AffineRegionMap::new(second_linear, SVector::from([4.0, 5.0]), 0.5, -1.0).unwrap();

        let composed = first.then(&second).unwrap();
        let position = SVector::from([3.0, 4.0]);
        let velocity = SVector::from([2.0, -3.0]);
        let time = 11.0;

        let sequential_position = second
            .map_position(&first.map_position(&position).unwrap())
            .unwrap();
        let sequential_velocity = second
            .map_velocity(&first.map_velocity(&velocity).unwrap())
            .unwrap();
        let sequential_time = second.map_time(first.map_time(time).unwrap()).unwrap();

        assert!((composed.map_position(&position).unwrap() - sequential_position).norm() < 1e-12);
        assert!((composed.map_velocity(&velocity).unwrap() - sequential_velocity).norm() < 1e-12);
        assert!((composed.map_time(time).unwrap() - sequential_time).abs() < 1e-12);
    }

    #[test]
    fn affine_map_composition_is_associative_with_clock_offsets() {
        let a = AffineRegionMap::new(
            SMatrix::<f64, 2, 2>::new(1.0, 0.25, 0.0, 2.0),
            SVector::from([1.0, -1.0]),
            2.0,
            3.0,
        )
        .unwrap();
        let b = AffineRegionMap::new(
            SMatrix::<f64, 2, 2>::new(0.0, -1.0, 1.0, 0.0),
            SVector::from([2.0, 4.0]),
            0.5,
            -2.0,
        )
        .unwrap();
        let c = AffineRegionMap::new(
            SMatrix::<f64, 2, 2>::new(1.5, 0.0, 0.0, 0.75),
            SVector::from([-3.0, 1.0]),
            1.25,
            0.5,
        )
        .unwrap();

        let left = a.then(&b).unwrap().then(&c).unwrap();
        let right = a.then(&b.then(&c).unwrap()).unwrap();
        let position = SVector::from([0.25, -7.0]);
        let velocity = SVector::from([-1.0, 2.0]);
        assert!(
            (left.map_position(&position).unwrap() - right.map_position(&position).unwrap()).norm()
                < 1e-12
        );
        assert!(
            (left.map_velocity(&velocity).unwrap() - right.map_velocity(&velocity).unwrap()).norm()
                < 1e-12
        );
        assert!((left.map_time(19.0).unwrap() - right.map_time(19.0).unwrap()).abs() < 1e-12);
    }

    #[test]
    fn conservation_ledger_rejects_unsupported_same_named_metric_profile() {
        let mut graph = RegionGraph::<2>::new().unwrap();
        let mut source = region(1, [-1.0, -1.0], [0.0, 1.0]);
        let mut target = region(2, [0.0, -1.0], [1.0, 1.0]);
        source.metric_profile = "hyperbolic-v1".to_string();
        target.metric_profile = "hyperbolic-v1".to_string();
        graph.add_region(source).unwrap();
        graph.add_region(target).unwrap();
        let map = AffineRegionMap::<2>::identity();
        let interface = RegionInterface::new(
            InterfaceId(19),
            RegionId(1),
            RegionId(2),
            1,
            boundary_x0(),
            boundary_x0(),
            map,
            None,
            false,
            CrossingDirection::Either,
            QuantityPolicy::Exchange,
            QuantityPolicy::Exchange,
            None,
        )
        .unwrap();
        assert_eq!(
            graph.add_interface(interface),
            Err(RegionGraphError::UnsupportedMetricForConservation)
        );
    }

    #[test]
    fn missing_transform_identity_is_rejected() {
        assert_eq!(
            RegionInterface::new(
                InterfaceId(14),
                RegionId(1),
                RegionId(2),
                1,
                boundary_x0(),
                boundary_x0(),
                AffineRegionMap::identity(),
                None,
                false,
                CrossingDirection::Either,
                QuantityPolicy::ExplicitTransform,
                QuantityPolicy::ExplicitTransform,
                None,
            ),
            Err(RegionGraphError::MissingTransformIdentity)
        );
    }
}
