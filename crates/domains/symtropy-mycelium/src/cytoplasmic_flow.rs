// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed observations of cytoplasmic flow on hyphal segments.
//!
//! Each stream remains an independent signed observation. Opposing streams in
//! the same segment/time window must not be collapsed into an unsigned speed.
//! Velocity alone is not volumetric flow; area and concentration are optional
//! and missing values never receive inferred defaults.

use std::fmt;

use crate::hyphal_graph::{HyphalGraph, SegmentId};
use crate::transport::SoluteId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FlowObservationId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FlowStreamId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FlowRoiId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowDirection {
    AlongSegment,
    AgainstSegment,
    Stationary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowVelocitySupport {
    /// Velocity of a tracked particle/trajectory; not a cross-sectional mean.
    TrajectoryParticle,
    /// Velocity explicitly qualified as representative across the stream area.
    CrossSectionMean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowTracerKind {
    /// Bulk cytoplasmic movement measurement, not cargo-specific movement.
    BulkCytoplasm,
    /// A fluorescently labelled cargo such as Nile-red-labelled lipid.
    FluorescentLipidCargo,
    /// Particle/tracer identity has not been chemically or biologically typed.
    UnspecifiedParticle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlowSoluteConcentration {
    pub solute: SoluteId,
    /// Milligrams per cubic millimetre.
    pub concentration_mg_per_mm3: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CytoplasmicFlowObservation {
    pub id: FlowObservationId,
    pub stream: FlowStreamId,
    pub roi: FlowRoiId,
    pub segment: SegmentId,
    pub time_hours: f64,
    /// Identity of the measured moving material; distinct tracers are never pooled.
    pub tracer: FlowTracerKind,
    /// Signed relative to the graph segment's declared from-to orientation.
    pub signed_velocity_mm_per_s: f64,
    /// Prevents treating a tracked particle's local speed as bulk mean velocity.
    pub velocity_support: FlowVelocitySupport,
    pub velocity_std_dev_mm_per_s: Option<f64>,
    pub observation_count: u32,
    /// Optional measured stream cross-section, in square millimetres.
    pub cross_section_area_mm2: Option<f64>,
    pub solute: Option<FlowSoluteConcentration>,
    pub sample_duration_s: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FlowObservationError {
    InvalidTimestamp,
    InvalidVelocity,
    InvalidVelocityUncertainty,
    EmptyObservationCount,
    InvalidCrossSectionArea,
    InvalidSoluteConcentration,
    InvalidSampleDuration,
    MissingSegment(SegmentId),
}

impl fmt::Display for FlowObservationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTimestamp => write!(f, "flow timestamp must be finite and non-negative"),
            Self::InvalidVelocity => write!(f, "signed flow velocity must be finite"),
            Self::InvalidVelocityUncertainty => {
                write!(
                    f,
                    "velocity standard deviation must be finite and non-negative"
                )
            }
            Self::EmptyObservationCount => write!(f, "flow observation count must be positive"),
            Self::InvalidCrossSectionArea => {
                write!(
                    f,
                    "stream cross-section area must be finite and positive when present"
                )
            }
            Self::InvalidSoluteConcentration => {
                write!(f, "solute concentration must be finite and non-negative")
            }
            Self::InvalidSampleDuration => {
                write!(
                    f,
                    "sample duration must be finite and positive when present"
                )
            }
            Self::MissingSegment(id) => {
                write!(f, "flow observation references missing segment {:?}", id)
            }
        }
    }
}

impl std::error::Error for FlowObservationError {}

impl CytoplasmicFlowObservation {
    pub fn validate(&self) -> Result<(), FlowObservationError> {
        if !self.time_hours.is_finite() || self.time_hours < 0.0 {
            return Err(FlowObservationError::InvalidTimestamp);
        }
        if !self.signed_velocity_mm_per_s.is_finite() {
            return Err(FlowObservationError::InvalidVelocity);
        }
        if self
            .velocity_std_dev_mm_per_s
            .is_some_and(|std_dev| !std_dev.is_finite() || std_dev < 0.0)
        {
            return Err(FlowObservationError::InvalidVelocityUncertainty);
        }
        if self.observation_count == 0 {
            return Err(FlowObservationError::EmptyObservationCount);
        }
        if self
            .cross_section_area_mm2
            .is_some_and(|area| !area.is_finite() || area <= 0.0)
        {
            return Err(FlowObservationError::InvalidCrossSectionArea);
        }
        if self.solute.is_some_and(|solute| {
            !solute.concentration_mg_per_mm3.is_finite() || solute.concentration_mg_per_mm3 < 0.0
        }) {
            return Err(FlowObservationError::InvalidSoluteConcentration);
        }
        if self
            .sample_duration_s
            .is_some_and(|duration| !duration.is_finite() || duration <= 0.0)
        {
            return Err(FlowObservationError::InvalidSampleDuration);
        }
        Ok(())
    }

    pub fn validate_segment_identity(
        &self,
        graph: &HyphalGraph,
    ) -> Result<(), FlowObservationError> {
        self.validate()?;
        if graph.segment(self.segment).is_none() {
            return Err(FlowObservationError::MissingSegment(self.segment));
        }
        Ok(())
    }

    pub fn direction(&self) -> FlowDirection {
        if self.signed_velocity_mm_per_s > 0.0 {
            FlowDirection::AlongSegment
        } else if self.signed_velocity_mm_per_s < 0.0 {
            FlowDirection::AgainstSegment
        } else {
            FlowDirection::Stationary
        }
    }

    /// Signed volumetric flow in mm^3/s.
    ///
    /// Returns None unless the velocity is explicitly qualified as a
    /// cross-section mean and the associated stream area is measured.
    pub fn signed_volume_flow_mm3_per_s(&self) -> Option<f64> {
        if self.velocity_support != FlowVelocitySupport::CrossSectionMean {
            return None;
        }
        self.cross_section_area_mm2
            .map(|area| self.signed_velocity_mm_per_s * area)
    }

    /// Signed solute mass rate in mg/s; requires both area and concentration.
    pub fn signed_solute_mass_rate_mg_per_s(&self) -> Option<f64> {
        Some(self.signed_volume_flow_mm3_per_s()? * self.solute?.concentration_mg_per_mm3)
    }

    pub fn integrated_volume_mm3(&self) -> Option<f64> {
        Some(self.signed_volume_flow_mm3_per_s()? * self.sample_duration_s?)
    }

    pub fn integrated_solute_amount_mg(&self) -> Option<f64> {
        Some(self.signed_solute_mass_rate_mg_per_s()? * self.sample_duration_s?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyphal_graph::{
        HyphalNode, HyphalNodeKind, HyphalSegment, HyphalSegmentState, NodeId, Position3Mm,
    };

    fn observation(
        id: u64,
        stream: u64,
        velocity: f64,
        area: Option<f64>,
    ) -> CytoplasmicFlowObservation {
        CytoplasmicFlowObservation {
            id: FlowObservationId(id),
            stream: FlowStreamId(stream),
            roi: FlowRoiId(5),
            segment: SegmentId(7),
            time_hours: 12.0,
            tracer: FlowTracerKind::UnspecifiedParticle,
            signed_velocity_mm_per_s: velocity,
            velocity_support: FlowVelocitySupport::TrajectoryParticle,
            velocity_std_dev_mm_per_s: Some(0.2),
            observation_count: 20,
            cross_section_area_mm2: area,
            solute: None,
            sample_duration_s: Some(2.0),
        }
    }

    fn graph() -> HyphalGraph {
        let mut graph = HyphalGraph::new();
        graph
            .add_node(HyphalNode {
                id: NodeId(1),
                kind: HyphalNodeKind::Anchor,
                position: Position3Mm::new(0.0, 0.0, 0.0),
            })
            .unwrap();
        graph
            .add_node(HyphalNode {
                id: NodeId(2),
                kind: HyphalNodeKind::ActiveTip,
                position: Position3Mm::new(1.0, 0.0, 0.0),
            })
            .unwrap();
        graph
            .add_segment(HyphalSegment {
                id: SegmentId(7),
                from: NodeId(1),
                to: NodeId(2),
                length_mm: 1.0,
                diameter_mm: Some(0.1),
                state: HyphalSegmentState::Active,
            })
            .unwrap();
        graph
    }

    #[test]
    fn velocity_only_observation_does_not_invent_volumetric_flow() {
        let item = observation(1, 1, 2.0, None);
        assert_eq!(item.direction(), FlowDirection::AlongSegment);
        assert_eq!(item.signed_volume_flow_mm3_per_s(), None);
        assert_eq!(item.signed_solute_mass_rate_mg_per_s(), None);
        assert_eq!(item.integrated_volume_mm3(), None);
    }

    #[test]
    fn measured_area_converts_velocity_to_signed_volume_flow() {
        let mut item = observation(2, 1, 2.0, Some(0.5));
        item.velocity_support = FlowVelocitySupport::CrossSectionMean;
        assert_eq!(item.signed_volume_flow_mm3_per_s(), Some(1.0));
        assert_eq!(item.integrated_volume_mm3(), Some(2.0));
    }

    #[test]
    fn solute_mass_rate_requires_area_and_concentration() {
        let mut item = observation(3, 1, 2.0, Some(0.5));
        item.velocity_support = FlowVelocitySupport::CrossSectionMean;
        item.solute = Some(FlowSoluteConcentration {
            solute: SoluteId(42),
            concentration_mg_per_mm3: 3.0,
        });
        assert_eq!(item.signed_solute_mass_rate_mg_per_s(), Some(3.0));
        assert_eq!(item.integrated_solute_amount_mg(), Some(6.0));
    }

    #[test]
    fn antiparallel_streams_are_preserved_as_distinct_observations() {
        let mut forward = observation(4, 10, 1.5, Some(0.2));
        let mut reverse = observation(5, 11, -0.75, Some(0.1));
        forward.velocity_support = FlowVelocitySupport::CrossSectionMean;
        reverse.velocity_support = FlowVelocitySupport::CrossSectionMean;
        assert_eq!(forward.segment, reverse.segment);
        assert_ne!(forward.stream, reverse.stream);
        assert_eq!(forward.direction(), FlowDirection::AlongSegment);
        assert_eq!(reverse.direction(), FlowDirection::AgainstSegment);
        assert!((forward.signed_volume_flow_mm3_per_s().unwrap() - 0.3).abs() < 1e-12);
        assert!((reverse.signed_volume_flow_mm3_per_s().unwrap() + 0.075).abs() < 1e-12);
    }

    #[test]
    fn trajectory_particle_velocity_is_not_promoted_to_bulk_flow() {
        let item = observation(12, 20, 3.0, Some(0.4));
        assert_eq!(
            item.velocity_support,
            FlowVelocitySupport::TrajectoryParticle
        );
        assert_eq!(item.signed_volume_flow_mm3_per_s(), None);
        assert_eq!(item.signed_solute_mass_rate_mg_per_s(), None);
    }

    #[test]
    fn bulk_cytoplasm_and_lipid_cargo_remain_distinct_tracer_classes() {
        let mut bulk = observation(13, 30, 6.0, None);
        let mut lipid = observation(14, 31, 3.0, None);
        bulk.tracer = FlowTracerKind::BulkCytoplasm;
        lipid.tracer = FlowTracerKind::FluorescentLipidCargo;

        assert_ne!(bulk.tracer, lipid.tracer);
        assert_eq!(bulk.signed_velocity_mm_per_s, 6.0);
        assert_eq!(lipid.signed_velocity_mm_per_s, 3.0);
        // Neither trajectory-particle measurement is silently promoted to
        // a cross-sectional bulk velocity or a nutrient mass flux.
        assert_eq!(bulk.signed_volume_flow_mm3_per_s(), None);
        assert_eq!(lipid.signed_volume_flow_mm3_per_s(), None);
        assert_eq!(bulk.signed_solute_mass_rate_mg_per_s(), None);
        assert_eq!(lipid.signed_solute_mass_rate_mg_per_s(), None);
    }

    #[test]
    fn stationary_stream_is_not_dropped() {
        let mut item = observation(6, 12, 0.0, Some(0.2));
        item.velocity_support = FlowVelocitySupport::CrossSectionMean;
        assert_eq!(item.direction(), FlowDirection::Stationary);
        assert_eq!(item.signed_volume_flow_mm3_per_s(), Some(0.0));
    }

    #[test]
    fn invalid_observation_values_are_rejected() {
        let mut item = observation(7, 1, f64::NAN, Some(0.2));
        assert_eq!(item.validate(), Err(FlowObservationError::InvalidVelocity));

        item = observation(8, 1, 1.0, Some(0.0));
        assert_eq!(
            item.validate(),
            Err(FlowObservationError::InvalidCrossSectionArea)
        );

        item = observation(9, 1, 1.0, Some(0.2));
        item.sample_duration_s = Some(f64::INFINITY);
        assert_eq!(
            item.validate(),
            Err(FlowObservationError::InvalidSampleDuration)
        );
    }

    #[test]
    fn missing_segment_identity_is_rejected() {
        let mut item = observation(10, 1, 1.0, None);
        item.segment = SegmentId(99);
        assert_eq!(
            item.validate_segment_identity(&graph()),
            Err(FlowObservationError::MissingSegment(SegmentId(99)))
        );
    }

    #[test]
    fn valid_flow_observation_maps_to_graph_segment() {
        let item = observation(11, 1, 1.0, Some(0.2));
        assert_eq!(item.validate_segment_identity(&graph()), Ok(()));
    }
}
