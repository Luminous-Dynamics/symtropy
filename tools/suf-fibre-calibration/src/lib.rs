// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fail-closed reference kernel for mapping DAS optical range to a
//! calibrated fibre-relative position and then to a projected route position.
//!
//! This crate deliberately does not perform asset attribution, datum conversion,
//! geodetic projection, DAS event detection, or physical qualification.

pub mod coverage;
pub mod decision;
pub mod source;

use core::fmt;

/// Result type used throughout the calibration kernel.
pub type CalibrationResult<T> = Result<T, CalibrationError>;

/// Direction in which the interrogator traverses the named fibre span.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FibreDirection {
    /// From the declared endpoint A toward endpoint B.
    AToB,
    /// From the declared endpoint B toward endpoint A.
    BToA,
}

/// Declared coordinate-frame kind. The kernel only interpolates in projected
/// planar metric coordinates; it does not implement geodetic projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordinateKind {
    /// Projected planar coordinates whose axes are measured in metres.
    ProjectedMetric,
    /// Angular coordinates such as longitude/latitude.
    GeographicAngular,
}

/// Axis order declared by the spatial source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisOrder {
    /// First coordinate is easting; second coordinate is northing.
    EastingNorthing,
    /// First coordinate is northing; second coordinate is easting.
    NorthingEasting,
}

/// Coordinate unit declared by the spatial source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordinateUnit {
    /// Metres.
    Metre,
    /// Degrees.
    Degree,
}

/// Errors are explicit because an invalid transformation must not degrade into
/// a plausible-looking coordinate.
#[derive(Clone, Debug, PartialEq)]
pub enum CalibrationError {
    /// A required identity, revision, or CRS identifier was empty.
    EmptyIdentifier(&'static str),
    /// A required numeric value was not finite.
    NonFinite(&'static str),
    /// An uncertainty/error bound was negative.
    NegativeUncertainty(&'static str),
    /// Fewer than two anchors were supplied.
    TooFewAnchors,
    /// Input anchors were not strictly increasing.
    NonMonotoneInputAnchors,
    /// Output anchors were not strictly increasing.
    NonMonotoneOutputAnchors,
    /// A query fell outside the calibration domain.
    OutOfRange {
        /// Axis name.
        axis: &'static str,
        /// Requested value.
        value: f64,
        /// Minimum supported value.
        min: f64,
        /// Maximum supported value.
        max: f64,
    },
    /// An observation does not match the frozen calibration binding.
    BindingMismatch(&'static str),
    /// A source digest is not exactly 64 hexadecimal characters.
    InvalidSha256,
    /// The route polyline does not contain valid, monotone vertices.
    InvalidRouteGeometry(&'static str),
    /// Only projected metres with easting/northing order are supported.
    UnsupportedCoordinateFrame,
    /// The route topology is not a single disambiguated fibre span.
    UnsupportedTopology,
    /// Observation phenomenon time is outside the calibration validity interval.
    OutsideValidityInterval {
        /// Observation phenomenon time in Unix milliseconds.
        observed_at_unix_ms: i64,
        /// Inclusive calibration validity start in Unix milliseconds.
        valid_from_unix_ms: i64,
        /// Inclusive calibration validity end in Unix milliseconds.
        valid_until_unix_ms: i64,
    },
    /// A mapped route station lies outside the supplied route geometry.
    RouteStationOutsideGeometry,
}

impl fmt::Display for CalibrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier(name) => write!(f, "empty identifier: {name}"),
            Self::NonFinite(name) => write!(f, "non-finite value: {name}"),
            Self::NegativeUncertainty(name) => write!(f, "negative uncertainty: {name}"),
            Self::TooFewAnchors => write!(f, "piecewise map requires at least two anchors"),
            Self::NonMonotoneInputAnchors => write!(f, "input anchors must be strictly increasing"),
            Self::NonMonotoneOutputAnchors => {
                write!(f, "output anchors must be strictly increasing")
            }
            Self::OutOfRange {
                axis,
                value,
                min,
                max,
            } => {
                write!(f, "{axis} value {value} is outside [{min}, {max}]")
            }
            Self::BindingMismatch(name) => write!(f, "observation binding mismatch: {name}"),
            Self::InvalidSha256 => write!(f, "source digest must be 64 hexadecimal characters"),
            Self::InvalidRouteGeometry(reason) => write!(f, "invalid route geometry: {reason}"),
            Self::UnsupportedCoordinateFrame => {
                write!(
                    f,
                    "only projected metre coordinates with easting/northing order are supported"
                )
            }
            Self::UnsupportedTopology => {
                write!(
                    f,
                    "branched or ambiguous fibre topology is unsupported by this single-span kernel"
                )
            }
            Self::OutsideValidityInterval {
                observed_at_unix_ms,
                valid_from_unix_ms,
                valid_until_unix_ms,
            } => {
                write!(
                    f,
                    "observation time {observed_at_unix_ms} is outside calibration validity [{valid_from_unix_ms}, {valid_until_unix_ms}]"
                )
            }
            Self::RouteStationOutsideGeometry => {
                write!(f, "mapped route station is outside the route geometry")
            }
        }
    }
}

impl std::error::Error for CalibrationError {}

/// A point correspondence in a one-dimensional calibration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalibrationAnchor {
    /// Coordinate on the input axis, in metres.
    pub input_m: f64,
    /// Corresponding coordinate on the output axis, in metres.
    pub output_m: f64,
}

/// Monotone, piecewise-linear map between two metre-valued axes.
///
/// Extrapolation is prohibited. Both input and output anchors must be strictly
/// increasing; reversed direction is represented explicitly by the binding.
#[derive(Clone, Debug, PartialEq)]
pub struct PiecewiseLinearMap {
    anchors: Vec<CalibrationAnchor>,
}

impl PiecewiseLinearMap {
    /// Construct and validate a non-extrapolating calibration map.
    pub fn new(anchors: Vec<CalibrationAnchor>) -> CalibrationResult<Self> {
        if anchors.len() < 2 {
            return Err(CalibrationError::TooFewAnchors);
        }
        for anchor in &anchors {
            if !anchor.input_m.is_finite() {
                return Err(CalibrationError::NonFinite("input anchor"));
            }
            if !anchor.output_m.is_finite() {
                return Err(CalibrationError::NonFinite("output anchor"));
            }
        }
        for pair in anchors.windows(2) {
            if pair[1].input_m <= pair[0].input_m {
                return Err(CalibrationError::NonMonotoneInputAnchors);
            }
            if pair[1].output_m <= pair[0].output_m {
                return Err(CalibrationError::NonMonotoneOutputAnchors);
            }
        }
        Ok(Self { anchors })
    }

    /// Lowest supported input coordinate.
    pub fn min_input_m(&self) -> f64 {
        self.anchors[0].input_m
    }

    /// Highest supported input coordinate.
    pub fn max_input_m(&self) -> f64 {
        self.anchors[self.anchors.len() - 1].input_m
    }

    /// Lowest mapped output coordinate.
    pub fn min_output_m(&self) -> f64 {
        self.anchors[0].output_m
    }

    /// Highest mapped output coordinate.
    pub fn max_output_m(&self) -> f64 {
        self.anchors[self.anchors.len() - 1].output_m
    }

    /// Map a value using linear interpolation; never extrapolate.
    pub fn map(&self, value_m: f64, axis: &'static str) -> CalibrationResult<f64> {
        if !value_m.is_finite() {
            return Err(CalibrationError::NonFinite(axis));
        }
        if value_m < self.min_input_m() || value_m > self.max_input_m() {
            return Err(CalibrationError::OutOfRange {
                axis,
                value: value_m,
                min: self.min_input_m(),
                max: self.max_input_m(),
            });
        }

        for pair in self.anchors.windows(2) {
            let left = pair[0];
            let right = pair[1];
            if value_m <= right.input_m {
                let fraction = (value_m - left.input_m) / (right.input_m - left.input_m);
                let mapped = left.output_m + fraction * (right.output_m - left.output_m);
                if !mapped.is_finite() {
                    return Err(CalibrationError::NonFinite("mapped coordinate"));
                }
                return Ok(mapped);
            }
        }

        // The upper endpoint is handled exactly to avoid a rounding-dependent
        // search fall-through.
        Ok(self.anchors[self.anchors.len() - 1].output_m)
    }
}

/// Immutable identities to which a calibration and every derived claim bind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalibrationBinding {
    /// Owner/provider-scoped fibre identity.
    pub fibre_id: String,
    /// Exact fibre/as-built revision.
    pub fibre_revision: String,
    /// Exact route-geometry revision.
    pub route_revision: String,
    /// Exact interrogator optical-path/configuration revision.
    pub interrogator_config_revision: String,
    /// Unique calibration revision.
    pub calibration_revision: String,
    /// Exact algorithm/method revision used to create the calibration.
    pub method_revision: String,
    /// SHA-256 of the calibration receipt/artifact bytes supplied by the caller.
    pub calibration_artifact_sha256: String,
    /// SHA-256 of the exact route geometry artifact bytes supplied by the caller.
    pub route_geometry_sha256: String,
    /// Declared endpoint A identity.
    pub endpoint_a_id: String,
    /// Declared endpoint B identity.
    pub endpoint_b_id: String,
    /// Direction expected by this single-span calibration.
    pub direction: FibreDirection,
    /// Inclusive start of calibration validity, Unix milliseconds.
    pub valid_from_unix_ms: i64,
    /// Inclusive end of calibration validity, Unix milliseconds.
    pub valid_until_unix_ms: i64,
}

impl CalibrationBinding {
    fn validate(&self) -> CalibrationResult<()> {
        for (name, value) in [
            ("fibre_id", self.fibre_id.as_str()),
            ("fibre_revision", self.fibre_revision.as_str()),
            ("route_revision", self.route_revision.as_str()),
            (
                "interrogator_config_revision",
                self.interrogator_config_revision.as_str(),
            ),
            ("calibration_revision", self.calibration_revision.as_str()),
            ("method_revision", self.method_revision.as_str()),
            ("endpoint_a_id", self.endpoint_a_id.as_str()),
            ("endpoint_b_id", self.endpoint_b_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(CalibrationError::EmptyIdentifier(name));
            }
        }
        validate_sha256(&self.calibration_artifact_sha256)?;
        validate_sha256(&self.route_geometry_sha256)?;
        if self.endpoint_a_id == self.endpoint_b_id {
            return Err(CalibrationError::BindingMismatch(
                "endpoints must be distinct",
            ));
        }
        if self.valid_from_unix_ms > self.valid_until_unix_ms {
            return Err(CalibrationError::BindingMismatch(
                "calibration validity interval",
            ));
        }
        Ok(())
    }
}

/// Declared coordinate frame for the supplied route geometry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoordinateFrame {
    /// CRS identifier, preferably a resolvable authority URI.
    pub crs_id: String,
    /// Coordinate kind.
    pub kind: CoordinateKind,
    /// Axis order.
    pub axis_order: AxisOrder,
    /// Coordinate unit.
    pub unit: CoordinateUnit,
}

impl CoordinateFrame {
    fn validate(&self) -> CalibrationResult<()> {
        if self.crs_id.trim().is_empty() {
            return Err(CalibrationError::EmptyIdentifier("crs_id"));
        }
        if self.kind != CoordinateKind::ProjectedMetric
            || self.axis_order != AxisOrder::EastingNorthing
            || self.unit != CoordinateUnit::Metre
        {
            return Err(CalibrationError::UnsupportedCoordinateFrame);
        }
        Ok(())
    }
}

/// One surveyed or otherwise independently evidenced route-geometry vertex.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RouteVertex {
    /// Curvilinear route station, in metres.
    pub station_m: f64,
    /// Projected easting, in metres.
    pub easting_m: f64,
    /// Projected northing, in metres.
    pub northing_m: f64,
}

/// Versioned, piecewise-linear route geometry in a projected metre CRS.
#[derive(Clone, Debug, PartialEq)]
pub struct RouteGeometry {
    /// Exact route revision.
    pub route_revision: String,
    /// Physical endpoint at route station zero.
    pub station_origin_endpoint_id: String,
    /// Physical endpoint toward which route station increases.
    pub station_increasing_to_endpoint_id: String,
    /// Declared coordinate frame.
    pub frame: CoordinateFrame,
    vertices: Vec<RouteVertex>,
}

impl RouteGeometry {
    /// Construct and validate route geometry.
    pub fn new(
        route_revision: String,
        station_origin_endpoint_id: String,
        station_increasing_to_endpoint_id: String,
        frame: CoordinateFrame,
        vertices: Vec<RouteVertex>,
    ) -> CalibrationResult<Self> {
        if route_revision.trim().is_empty() {
            return Err(CalibrationError::EmptyIdentifier("route_revision"));
        }
        if station_origin_endpoint_id.trim().is_empty() {
            return Err(CalibrationError::EmptyIdentifier(
                "station_origin_endpoint_id",
            ));
        }
        if station_increasing_to_endpoint_id.trim().is_empty() {
            return Err(CalibrationError::EmptyIdentifier(
                "station_increasing_to_endpoint_id",
            ));
        }
        if station_origin_endpoint_id == station_increasing_to_endpoint_id {
            return Err(CalibrationError::BindingMismatch(
                "route station endpoints must be distinct",
            ));
        }
        frame.validate()?;
        if vertices.len() < 2 {
            return Err(CalibrationError::InvalidRouteGeometry(
                "at least two vertices are required",
            ));
        }
        for vertex in &vertices {
            if !vertex.station_m.is_finite()
                || !vertex.easting_m.is_finite()
                || !vertex.northing_m.is_finite()
            {
                return Err(CalibrationError::InvalidRouteGeometry(
                    "all coordinates must be finite",
                ));
            }
        }
        for pair in vertices.windows(2) {
            if pair[1].station_m <= pair[0].station_m {
                return Err(CalibrationError::InvalidRouteGeometry(
                    "route stations must be strictly increasing",
                ));
            }
            let dx = pair[1].easting_m - pair[0].easting_m;
            let dy = pair[1].northing_m - pair[0].northing_m;
            if dx.hypot(dy) <= f64::EPSILON {
                return Err(CalibrationError::InvalidRouteGeometry(
                    "consecutive vertices must not have identical horizontal coordinates",
                ));
            }
        }
        Ok(Self {
            route_revision,
            station_origin_endpoint_id,
            station_increasing_to_endpoint_id,
            frame,
            vertices,
        })
    }

    /// Lowest supported route station.
    pub fn min_station_m(&self) -> f64 {
        self.vertices[0].station_m
    }

    /// Highest supported route station.
    pub fn max_station_m(&self) -> f64 {
        self.vertices[self.vertices.len() - 1].station_m
    }

    /// Interpolate a point on the declared route station axis.
    pub fn position_at(&self, station_m: f64) -> CalibrationResult<(f64, f64)> {
        if !station_m.is_finite() {
            return Err(CalibrationError::NonFinite("route_station_m"));
        }
        if station_m < self.min_station_m() || station_m > self.max_station_m() {
            return Err(CalibrationError::RouteStationOutsideGeometry);
        }
        for pair in self.vertices.windows(2) {
            let left = pair[0];
            let right = pair[1];
            if station_m <= right.station_m {
                let fraction = (station_m - left.station_m) / (right.station_m - left.station_m);
                let easting = left.easting_m + fraction * (right.easting_m - left.easting_m);
                let northing = left.northing_m + fraction * (right.northing_m - left.northing_m);
                if !easting.is_finite() || !northing.is_finite() {
                    return Err(CalibrationError::NonFinite(
                        "interpolated projected position",
                    ));
                }
                return Ok((easting, northing));
            }
        }
        let last = self.vertices[self.vertices.len() - 1];
        Ok((last.easting_m, last.northing_m))
    }
}

/// Conservative horizontal-equivalent absolute error bounds, all expressed
/// in projected horizontal metres.
///
/// Each upstream component must already be propagated into a horizontal
/// coordinate bound for this exact calibration/profile. In particular, raw
/// optical-range/fibre-chainage/route-station errors must not be inserted
/// without accounting for the corresponding transform sensitivity.
///
/// The total is a triangle-inequality upper bound, not a Gaussian standard
/// deviation or a statistically calibrated confidence interval. Correlations
/// are therefore not incorrectly assumed to be independent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UncertaintyBudget {
    /// Bound on optical-range measurement error.
    pub optical_measurement_bound_m: f64,
    /// Bound contributed by optical-range to physical-fibre calibration.
    pub optical_to_fibre_bound_m: f64,
    /// Bound contributed by physical-fibre to route-station calibration.
    pub fibre_to_route_bound_m: f64,
    /// Bound contributed by route geometry/survey.
    pub route_geometry_bound_m: f64,
}

impl UncertaintyBudget {
    /// Validate and sum the conservative component bounds.
    pub fn total_bound_m(self) -> CalibrationResult<f64> {
        for (name, value) in [
            (
                "optical_measurement_bound_m",
                self.optical_measurement_bound_m,
            ),
            ("optical_to_fibre_bound_m", self.optical_to_fibre_bound_m),
            ("fibre_to_route_bound_m", self.fibre_to_route_bound_m),
            ("route_geometry_bound_m", self.route_geometry_bound_m),
        ] {
            if !value.is_finite() {
                return Err(CalibrationError::NonFinite(name));
            }
            if value < 0.0 {
                return Err(CalibrationError::NegativeUncertainty(name));
            }
        }
        let total = self.optical_measurement_bound_m
            + self.optical_to_fibre_bound_m
            + self.fibre_to_route_bound_m
            + self.route_geometry_bound_m;
        if !total.is_finite() {
            return Err(CalibrationError::NonFinite("total uncertainty bound"));
        }
        Ok(total)
    }
}

/// Fibre topology supported by this reference kernel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FibreTopology {
    /// One identified, directionally resolved span.
    SingleSpan,
    /// A branched, multiplexed or otherwise ambiguous path.
    BranchedOrAmbiguous,
}

/// A source observation containing a fibre-relative optical range.
#[derive(Clone, Debug, PartialEq)]
pub struct FibreObservation {
    /// Unique observation identifier.
    pub observation_id: String,
    /// Fibre identity.
    pub fibre_id: String,
    /// Exact fibre/as-built revision.
    pub fibre_revision: String,
    /// Exact route revision used by the producer.
    pub route_revision: String,
    /// Exact interrogator configuration revision.
    pub interrogator_config_revision: String,
    /// Direction of the optical path.
    pub direction: FibreDirection,
    /// Physical endpoint at which this observation's optical path begins.
    pub endpoint_from_id: String,
    /// Physical endpoint at which this observation's optical path ends.
    pub endpoint_to_id: String,
    /// Phenomenon time represented by the observation, Unix milliseconds.
    pub phenomenon_time_unix_ms: i64,
    /// Optical range emitted by the interrogator, in metres.
    pub optical_range_m: f64,
    /// SHA-256 digest of the exact acquired source artifact.
    pub source_sha256: String,
}

/// Qualified position on a route.
///
/// There is intentionally no asset ID or asset-owner field: this result is not
/// an asset-attribution claim.
#[derive(Clone, Debug, PartialEq)]
pub struct QualifiedRoutePosition {
    /// Source observation identity.
    pub observation_id: String,
    /// Fibre identity and revision.
    pub fibre_id: String,
    /// Exact fibre revision.
    pub fibre_revision: String,
    /// Exact route revision.
    pub route_revision: String,
    /// Exact calibration revision.
    pub calibration_revision: String,
    /// Calibration method revision.
    pub method_revision: String,
    /// Digest of the exact calibration receipt/artifact.
    pub calibration_artifact_sha256: String,
    /// Digest of the exact route geometry artifact.
    pub route_geometry_sha256: String,
    /// Physical fibre chainage, in metres.
    pub physical_fibre_chainage_m: f64,
    /// Route station, in metres.
    pub route_station_m: f64,
    /// Projected easting, in metres.
    pub easting_m: f64,
    /// Projected northing, in metres.
    pub northing_m: f64,
    /// Conservative horizontal error bound, in metres.
    pub horizontal_error_bound_m: f64,
    /// CRS identifier for the projected coordinates.
    pub crs_id: String,
    /// Digest of the source observation artifact.
    pub source_sha256: String,
}

/// Single-span calibration model with explicit axis transforms and bindings.
#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationModel {
    binding: CalibrationBinding,
    optical_to_fibre: PiecewiseLinearMap,
    fibre_to_route: PiecewiseLinearMap,
    route_geometry: RouteGeometry,
    uncertainty: UncertaintyBudget,
}

impl CalibrationModel {
    /// Construct a model, checking exact route revision and complete axis
    /// coverage before it can produce a position.
    pub fn new(
        binding: CalibrationBinding,
        topology: FibreTopology,
        optical_to_fibre: PiecewiseLinearMap,
        fibre_to_route: PiecewiseLinearMap,
        route_geometry: RouteGeometry,
        uncertainty: UncertaintyBudget,
    ) -> CalibrationResult<Self> {
        binding.validate()?;
        if topology != FibreTopology::SingleSpan {
            return Err(CalibrationError::UnsupportedTopology);
        }
        if route_geometry.route_revision != binding.route_revision {
            return Err(CalibrationError::BindingMismatch("route geometry revision"));
        }
        let (expected_origin, expected_increasing_to) = match binding.direction {
            FibreDirection::AToB => (&binding.endpoint_a_id, &binding.endpoint_b_id),
            FibreDirection::BToA => (&binding.endpoint_b_id, &binding.endpoint_a_id),
        };
        if &route_geometry.station_origin_endpoint_id != expected_origin
            || &route_geometry.station_increasing_to_endpoint_id != expected_increasing_to
        {
            return Err(CalibrationError::BindingMismatch(
                "route geometry station orientation",
            ));
        }
        let uncertainty_total = uncertainty.total_bound_m()?;
        if !uncertainty_total.is_finite() {
            return Err(CalibrationError::NonFinite("total uncertainty bound"));
        }

        // The complete optical calibration output domain must be supported by
        // the fibre-to-route calibration input domain.
        if optical_to_fibre.min_output_m() < fibre_to_route.min_input_m()
            || optical_to_fibre.max_output_m() > fibre_to_route.max_input_m()
        {
            return Err(CalibrationError::OutOfRange {
                axis: "fibre_to_route_calibration_domain",
                value: optical_to_fibre.max_output_m(),
                min: fibre_to_route.min_input_m(),
                max: fibre_to_route.max_input_m(),
            });
        }

        if fibre_to_route.min_output_m() < route_geometry.min_station_m()
            || fibre_to_route.max_output_m() > route_geometry.max_station_m()
        {
            return Err(CalibrationError::RouteStationOutsideGeometry);
        }

        Ok(Self {
            binding,
            optical_to_fibre,
            fibre_to_route,
            route_geometry,
            uncertainty,
        })
    }

    /// Transform an observation only when every identity, revision, direction
    /// and source-integrity binding matches the model.
    pub fn locate(
        &self,
        observation: &FibreObservation,
    ) -> CalibrationResult<QualifiedRoutePosition> {
        if observation.observation_id.trim().is_empty() {
            return Err(CalibrationError::EmptyIdentifier("observation_id"));
        }
        if observation.fibre_id != self.binding.fibre_id {
            return Err(CalibrationError::BindingMismatch("fibre_id"));
        }
        if observation.fibre_revision != self.binding.fibre_revision {
            return Err(CalibrationError::BindingMismatch("fibre_revision"));
        }
        if observation.route_revision != self.binding.route_revision {
            return Err(CalibrationError::BindingMismatch("route_revision"));
        }
        if observation.interrogator_config_revision != self.binding.interrogator_config_revision {
            return Err(CalibrationError::BindingMismatch(
                "interrogator_config_revision",
            ));
        }
        if observation.direction != self.binding.direction {
            return Err(CalibrationError::BindingMismatch("direction"));
        }
        let (expected_from, expected_to) = match self.binding.direction {
            FibreDirection::AToB => (&self.binding.endpoint_a_id, &self.binding.endpoint_b_id),
            FibreDirection::BToA => (&self.binding.endpoint_b_id, &self.binding.endpoint_a_id),
        };
        if &observation.endpoint_from_id != expected_from {
            return Err(CalibrationError::BindingMismatch("endpoint_from_id"));
        }
        if &observation.endpoint_to_id != expected_to {
            return Err(CalibrationError::BindingMismatch("endpoint_to_id"));
        }
        if observation.phenomenon_time_unix_ms < self.binding.valid_from_unix_ms
            || observation.phenomenon_time_unix_ms > self.binding.valid_until_unix_ms
        {
            return Err(CalibrationError::OutsideValidityInterval {
                observed_at_unix_ms: observation.phenomenon_time_unix_ms,
                valid_from_unix_ms: self.binding.valid_from_unix_ms,
                valid_until_unix_ms: self.binding.valid_until_unix_ms,
            });
        }
        validate_sha256(&observation.source_sha256)?;

        let physical_fibre_chainage_m = self
            .optical_to_fibre
            .map(observation.optical_range_m, "optical_range_m")?;
        let route_station_m = self
            .fibre_to_route
            .map(physical_fibre_chainage_m, "physical_fibre_chainage_m")?;
        let (easting_m, northing_m) = self.route_geometry.position_at(route_station_m)?;
        let horizontal_error_bound_m = self.uncertainty.total_bound_m()?;

        Ok(QualifiedRoutePosition {
            observation_id: observation.observation_id.clone(),
            fibre_id: self.binding.fibre_id.clone(),
            fibre_revision: self.binding.fibre_revision.clone(),
            route_revision: self.binding.route_revision.clone(),
            calibration_revision: self.binding.calibration_revision.clone(),
            method_revision: self.binding.method_revision.clone(),
            calibration_artifact_sha256: self.binding.calibration_artifact_sha256.clone(),
            route_geometry_sha256: self.binding.route_geometry_sha256.clone(),
            physical_fibre_chainage_m,
            route_station_m,
            easting_m,
            northing_m,
            horizontal_error_bound_m,
            crs_id: self.route_geometry.frame.crs_id.clone(),
            source_sha256: observation.source_sha256.clone(),
        })
    }
}

fn validate_sha256(value: &str) -> CalibrationResult<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(CalibrationError::InvalidSha256);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn map(anchors: &[(f64, f64)]) -> PiecewiseLinearMap {
        PiecewiseLinearMap::new(
            anchors
                .iter()
                .map(|(input_m, output_m)| CalibrationAnchor {
                    input_m: *input_m,
                    output_m: *output_m,
                })
                .collect(),
        )
        .expect("valid test map")
    }

    fn frame() -> CoordinateFrame {
        CoordinateFrame {
            crs_id: "urn:ogc:def:crs:EPSG::32735".to_owned(),
            kind: CoordinateKind::ProjectedMetric,
            axis_order: AxisOrder::EastingNorthing,
            unit: CoordinateUnit::Metre,
        }
    }

    fn route() -> RouteGeometry {
        RouteGeometry::new(
            "route-r7".to_owned(),
            "chamber-A".to_owned(),
            "chamber-B".to_owned(),
            frame(),
            vec![
                RouteVertex {
                    station_m: 0.0,
                    easting_m: 100.0,
                    northing_m: 200.0,
                },
                RouteVertex {
                    station_m: 50.0,
                    easting_m: 150.0,
                    northing_m: 200.0,
                },
                RouteVertex {
                    station_m: 100.0,
                    easting_m: 150.0,
                    northing_m: 250.0,
                },
            ],
        )
        .expect("valid route")
    }

    fn model() -> CalibrationModel {
        let binding = CalibrationBinding {
            fibre_id: "utility-owner:fibre:17".to_owned(),
            fibre_revision: "fibre-r12".to_owned(),
            route_revision: "route-r7".to_owned(),
            interrogator_config_revision: "interrogator-config-3".to_owned(),
            calibration_revision: "calibration-r4".to_owned(),
            method_revision: "piecewise-linear-v1".to_owned(),
            calibration_artifact_sha256: DIGEST.to_owned(),
            route_geometry_sha256: DIGEST.to_owned(),
            endpoint_a_id: "chamber-A".to_owned(),
            endpoint_b_id: "chamber-B".to_owned(),
            direction: FibreDirection::AToB,
            valid_from_unix_ms: 1_000,
            valid_until_unix_ms: 2_000,
        };
        CalibrationModel::new(
            binding,
            FibreTopology::SingleSpan,
            map(&[(0.0, 10.0), (100.0, 110.0)]),
            map(&[(10.0, 0.0), (110.0, 100.0)]),
            route(),
            UncertaintyBudget {
                optical_measurement_bound_m: 0.2,
                optical_to_fibre_bound_m: 0.3,
                fibre_to_route_bound_m: 0.4,
                route_geometry_bound_m: 0.5,
            },
        )
        .expect("valid calibration model")
    }

    fn observation() -> FibreObservation {
        FibreObservation {
            observation_id: "obs-1".to_owned(),
            fibre_id: "utility-owner:fibre:17".to_owned(),
            fibre_revision: "fibre-r12".to_owned(),
            route_revision: "route-r7".to_owned(),
            interrogator_config_revision: "interrogator-config-3".to_owned(),
            direction: FibreDirection::AToB,
            endpoint_from_id: "chamber-A".to_owned(),
            endpoint_to_id: "chamber-B".to_owned(),
            phenomenon_time_unix_ms: 1_500,
            optical_range_m: 60.0,
            source_sha256: DIGEST.to_owned(),
        }
    }

    #[test]
    fn composes_distinct_coordinate_axes_without_asset_attribution() {
        let position = model()
            .locate(&observation())
            .expect("position should qualify");
        assert!((position.physical_fibre_chainage_m - 70.0).abs() < 1e-9);
        assert!((position.route_station_m - 60.0).abs() < 1e-9);
        assert!((position.easting_m - 150.0).abs() < 1e-9);
        assert!((position.northing_m - 210.0).abs() < 1e-9);
        assert!((position.horizontal_error_bound_m - 1.4).abs() < 1e-9);
        assert_eq!(position.crs_id, "urn:ogc:def:crs:EPSG::32735");
        assert_eq!(position.method_revision, "piecewise-linear-v1");
        assert_eq!(position.calibration_artifact_sha256, DIGEST);
        assert_eq!(position.route_geometry_sha256, DIGEST);
        // QualifiedRoutePosition intentionally has no asset_id field.
    }

    #[test]
    fn piecewise_map_interpolates_between_control_points() {
        let mapping = map(&[(0.0, 0.0), (10.0, 20.0), (30.0, 40.0)]);
        assert!((mapping.map(5.0, "test").expect("map") - 10.0).abs() < 1e-9);
        assert!((mapping.map(20.0, "test").expect("map") - 30.0).abs() < 1e-9);
    }

    #[test]
    fn exact_upper_endpoint_is_supported() {
        assert_eq!(map(&[(0.0, 5.0), (10.0, 15.0)]).map(10.0, "test"), Ok(15.0));
    }

    #[test]
    fn does_not_extrapolate_optical_range() {
        assert!(matches!(
            model().locate(&FibreObservation {
                optical_range_m: 101.0,
                ..observation()
            }),
            Err(CalibrationError::OutOfRange {
                axis: "optical_range_m",
                ..
            })
        ));
    }

    #[test]
    fn rejects_stale_fibre_revision() {
        assert_eq!(
            model().locate(&FibreObservation {
                fibre_revision: "fibre-r11".to_owned(),
                ..observation()
            }),
            Err(CalibrationError::BindingMismatch("fibre_revision"))
        );
    }

    #[test]
    fn rejects_stale_route_revision_after_repair() {
        assert_eq!(
            model().locate(&FibreObservation {
                route_revision: "route-r8-after-repair".to_owned(),
                ..observation()
            }),
            Err(CalibrationError::BindingMismatch("route_revision"))
        );
    }

    #[test]
    fn rejects_interrogator_configuration_change() {
        assert_eq!(
            model().locate(&FibreObservation {
                interrogator_config_revision: "config-r4".to_owned(),
                ..observation()
            }),
            Err(CalibrationError::BindingMismatch(
                "interrogator_config_revision"
            ))
        );
    }

    #[test]
    fn rejects_endpoint_identity_mismatch_even_when_direction_label_matches() {
        assert_eq!(
            model().locate(&FibreObservation {
                endpoint_from_id: "unknown-chamber".to_owned(),
                ..observation()
            }),
            Err(CalibrationError::BindingMismatch("endpoint_from_id"))
        );
    }

    #[test]
    fn rejects_endpoint_direction_mismatch() {
        assert_eq!(
            model().locate(&FibreObservation {
                direction: FibreDirection::BToA,
                ..observation()
            }),
            Err(CalibrationError::BindingMismatch("direction"))
        );
    }

    #[test]
    fn rejects_observation_outside_calibration_validity_window() {
        assert!(matches!(
            model().locate(&FibreObservation {
                phenomenon_time_unix_ms: 2_001,
                ..observation()
            }),
            Err(CalibrationError::OutsideValidityInterval { .. })
        ));
    }

    #[test]
    fn rejects_invalid_source_digest() {
        assert_eq!(
            model().locate(&FibreObservation {
                source_sha256: "not-a-digest".to_owned(),
                ..observation()
            }),
            Err(CalibrationError::InvalidSha256)
        );
    }

    #[test]
    fn rejects_non_finite_optical_range() {
        assert_eq!(
            model().locate(&FibreObservation {
                optical_range_m: f64::NAN,
                ..observation()
            }),
            Err(CalibrationError::NonFinite("optical_range_m"))
        );
    }

    #[test]
    fn rejects_non_monotone_input_anchors() {
        assert_eq!(
            PiecewiseLinearMap::new(vec![
                CalibrationAnchor {
                    input_m: 0.0,
                    output_m: 0.0
                },
                CalibrationAnchor {
                    input_m: 0.0,
                    output_m: 1.0
                },
            ]),
            Err(CalibrationError::NonMonotoneInputAnchors)
        );
    }

    #[test]
    fn rejects_non_monotone_output_anchors() {
        assert_eq!(
            PiecewiseLinearMap::new(vec![
                CalibrationAnchor {
                    input_m: 0.0,
                    output_m: 10.0
                },
                CalibrationAnchor {
                    input_m: 1.0,
                    output_m: 9.0
                },
            ]),
            Err(CalibrationError::NonMonotoneOutputAnchors)
        );
    }

    #[test]
    fn rejects_geographic_degrees_instead_of_treating_them_as_metres() {
        let mut geographic = frame();
        geographic.kind = CoordinateKind::GeographicAngular;
        geographic.unit = CoordinateUnit::Degree;
        assert_eq!(
            RouteGeometry::new(
                "route-r7".to_owned(),
                "chamber-A".to_owned(),
                "chamber-B".to_owned(),
                geographic,
                route().vertices,
            ),
            Err(CalibrationError::UnsupportedCoordinateFrame)
        );
    }

    #[test]
    fn rejects_axis_order_mismatch() {
        let mut wrong_axis = frame();
        wrong_axis.axis_order = AxisOrder::NorthingEasting;
        assert_eq!(
            RouteGeometry::new(
                "route-r7".to_owned(),
                "chamber-A".to_owned(),
                "chamber-B".to_owned(),
                wrong_axis,
                route().vertices,
            ),
            Err(CalibrationError::UnsupportedCoordinateFrame)
        );
    }

    #[test]
    fn rejects_route_geometry_with_non_monotone_station() {
        assert!(matches!(
            RouteGeometry::new(
                "route-r7".to_owned(),
                "chamber-A".to_owned(),
                "chamber-B".to_owned(),
                frame(),
                vec![
                    RouteVertex {
                        station_m: 0.0,
                        easting_m: 0.0,
                        northing_m: 0.0
                    },
                    RouteVertex {
                        station_m: 0.0,
                        easting_m: 1.0,
                        northing_m: 1.0
                    },
                ],
            ),
            Err(CalibrationError::InvalidRouteGeometry(_))
        ));
    }

    #[test]
    fn rejects_ambiguous_or_branched_topology() {
        let base = model();
        assert_eq!(
            CalibrationModel::new(
                base.binding,
                FibreTopology::BranchedOrAmbiguous,
                base.optical_to_fibre,
                base.fibre_to_route,
                base.route_geometry,
                base.uncertainty,
            ),
            Err(CalibrationError::UnsupportedTopology)
        );
    }

    #[test]
    fn rejects_invalid_calibration_receipt_digest() {
        let mut binding = model().binding;
        binding.calibration_artifact_sha256 = "not-a-digest".to_owned();
        assert_eq!(
            CalibrationModel::new(
                binding,
                FibreTopology::SingleSpan,
                map(&[(0.0, 10.0), (100.0, 110.0)]),
                map(&[(10.0, 0.0), (110.0, 100.0)]),
                route(),
                UncertaintyBudget {
                    optical_measurement_bound_m: 0.0,
                    optical_to_fibre_bound_m: 0.0,
                    fibre_to_route_bound_m: 0.0,
                    route_geometry_bound_m: 0.0,
                },
            ),
            Err(CalibrationError::InvalidSha256)
        );
    }

    #[test]
    fn rejects_route_revision_mismatch_in_model_construction() {
        let mut base = model();
        base.binding.route_revision = "route-r8".to_owned();
        assert_eq!(
            CalibrationModel::new(
                base.binding,
                FibreTopology::SingleSpan,
                base.optical_to_fibre,
                base.fibre_to_route,
                base.route_geometry,
                base.uncertainty,
            ),
            Err(CalibrationError::BindingMismatch("route geometry revision"))
        );
    }

    #[test]
    fn rejects_route_station_orientation_opposite_to_interrogator_direction() {
        let base = model();
        let reversed_route = RouteGeometry::new(
            "route-r7".to_owned(),
            "chamber-B".to_owned(),
            "chamber-A".to_owned(),
            frame(),
            vec![
                RouteVertex {
                    station_m: 0.0,
                    easting_m: 150.0,
                    northing_m: 250.0,
                },
                RouteVertex {
                    station_m: 50.0,
                    easting_m: 150.0,
                    northing_m: 200.0,
                },
                RouteVertex {
                    station_m: 100.0,
                    easting_m: 100.0,
                    northing_m: 200.0,
                },
            ],
        )
        .expect("reversed route geometry is valid");

        assert_eq!(
            CalibrationModel::new(
                base.binding,
                FibreTopology::SingleSpan,
                base.optical_to_fibre,
                base.fibre_to_route,
                reversed_route,
                base.uncertainty,
            ),
            Err(CalibrationError::BindingMismatch(
                "route geometry station orientation"
            ))
        );
    }

    #[test]
    fn reverse_interrogation_uses_a_reverse_oriented_route_station_axis() {
        let base = model();
        let mut binding = base.binding;
        binding.direction = FibreDirection::BToA;
        let reversed_route = RouteGeometry::new(
            "route-r7".to_owned(),
            "chamber-B".to_owned(),
            "chamber-A".to_owned(),
            frame(),
            vec![
                RouteVertex {
                    station_m: 0.0,
                    easting_m: 150.0,
                    northing_m: 250.0,
                },
                RouteVertex {
                    station_m: 50.0,
                    easting_m: 150.0,
                    northing_m: 200.0,
                },
                RouteVertex {
                    station_m: 100.0,
                    easting_m: 100.0,
                    northing_m: 200.0,
                },
            ],
        )
        .expect("reverse-oriented route geometry is valid");
        let reverse_model = CalibrationModel::new(
            binding,
            FibreTopology::SingleSpan,
            base.optical_to_fibre,
            base.fibre_to_route,
            reversed_route,
            base.uncertainty,
        )
        .expect("reverse-oriented calibration should qualify");

        let reverse_observation = FibreObservation {
            direction: FibreDirection::BToA,
            endpoint_from_id: "chamber-B".to_owned(),
            endpoint_to_id: "chamber-A".to_owned(),
            ..observation()
        };
        let position = reverse_model
            .locate(&reverse_observation)
            .expect("reverse observation");
        assert!((position.route_station_m - 60.0).abs() < 1e-9);
        assert!((position.easting_m - 140.0).abs() < 1e-9);
        assert!((position.northing_m - 200.0).abs() < 1e-9);
    }

    #[test]
    fn uncertainty_is_conservatively_summed_not_rss_combined() {
        let budget = UncertaintyBudget {
            optical_measurement_bound_m: 1.0,
            optical_to_fibre_bound_m: 2.0,
            fibre_to_route_bound_m: 3.0,
            route_geometry_bound_m: 4.0,
        };
        assert_eq!(budget.total_bound_m(), Ok(10.0));
    }

    #[test]
    fn rejects_negative_uncertainty() {
        let budget = UncertaintyBudget {
            optical_measurement_bound_m: 0.0,
            optical_to_fibre_bound_m: -0.1,
            fibre_to_route_bound_m: 0.0,
            route_geometry_bound_m: 0.0,
        };
        assert_eq!(
            budget.total_bound_m(),
            Err(CalibrationError::NegativeUncertainty(
                "optical_to_fibre_bound_m"
            ))
        );
    }

    #[test]
    fn rejects_incomplete_fibre_to_route_calibration_domain() {
        let binding = model().binding;
        let result = CalibrationModel::new(
            binding,
            FibreTopology::SingleSpan,
            map(&[(0.0, 10.0), (100.0, 110.0)]),
            map(&[(20.0, 0.0), (110.0, 100.0)]),
            route(),
            UncertaintyBudget {
                optical_measurement_bound_m: 0.0,
                optical_to_fibre_bound_m: 0.0,
                fibre_to_route_bound_m: 0.0,
                route_geometry_bound_m: 0.0,
            },
        );
        assert!(matches!(
            result,
            Err(CalibrationError::OutOfRange {
                axis: "fibre_to_route_calibration_domain",
                ..
            })
        ));
    }
}
