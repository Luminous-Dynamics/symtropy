//! Source-bounded pressure/tracer screening for the EHD low-density-air PIV cohort.
//!
//! This reports only source-table feasibility. It does not infer velocity
//! accuracy, particle passivity, a physical correction factor, or boundary-layer
//! resolution. Exact tested pressures only: no interpolation is permitted.

use std::error::Error;
use std::fmt;

const PRESSURE_MATCH_TOLERANCE_ATM: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TracerKind {
    Dehs,
    VaporizedFog,
    IncenseSticks,
    IncenseCones,
    Sio2,
}

impl TracerKind {
    fn index(self) -> usize {
        match self {
            Self::Dehs => 0,
            Self::VaporizedFog => 1,
            Self::IncenseSticks => 2,
            Self::IncenseCones => 3,
            Self::Sio2 => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TracerDataStatus {
    Available,
    Unavailable,
    NoData,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TracerResolutionEvidence {
    /// Minimum interrogation window reported for a whole-field extraction.
    pub full_field_min_window_px: Option<u16>,
    /// Minimum interrogation window reported only for an edge-case extraction.
    /// Parenthesized source values are not promoted to full-field evidence.
    pub edge_case_min_window_px: Option<u16>,
    pub status: TracerDataStatus,
}

const fn available(
    full_field_min_window_px: Option<u16>,
    edge_case_min_window_px: Option<u16>,
) -> TracerResolutionEvidence {
    TracerResolutionEvidence {
        status: TracerDataStatus::Available,
        full_field_min_window_px,
        edge_case_min_window_px,
    }
}

const fn unavailable() -> TracerResolutionEvidence {
    TracerResolutionEvidence {
        status: TracerDataStatus::Unavailable,
        full_field_min_window_px: None,
        edge_case_min_window_px: None,
    }
}

const fn no_data() -> TracerResolutionEvidence {
    TracerResolutionEvidence {
        status: TracerDataStatus::NoData,
        full_field_min_window_px: None,
        edge_case_min_window_px: None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PressureTracerRow {
    pub pressure_atm: f64,
    /// Order: DEHS, vaporized fog, incense sticks, incense cones, SiO2.
    pub tracers: [TracerResolutionEvidence; 5],
}

/// Exact transcription of the source pressure/tracer interrogation-area table.
/// Parenthesized values before a dash are edge-case-only; unparenthesized values
/// after a dash are whole-field values. A bare parenthesized value is edge-only.
pub const SOURCE_PRESSURE_TRACER_TABLE: [PressureTracerRow; 10] = [
    PressureTracerRow { pressure_atm: 1.00, tracers: [
        available(Some(128), Some(64)), available(Some(32), None),
        available(Some(64), Some(32)), available(Some(64), Some(32)),
        available(Some(64), Some(32)),
    ]},
    PressureTracerRow { pressure_atm: 0.90, tracers: [
        available(Some(128), None), available(Some(32), None),
        available(Some(64), Some(32)), available(Some(64), Some(32)),
        available(Some(64), None),
    ]},
    PressureTracerRow { pressure_atm: 0.80, tracers: [
        available(Some(128), None), available(Some(128), Some(64)),
        available(Some(64), Some(32)), available(Some(64), None),
        available(Some(64), None),
    ]},
    PressureTracerRow { pressure_atm: 0.70, tracers: [
        available(Some(128), None), available(None, Some(128)),
        available(Some(64), None), available(Some(64), None),
        available(Some(64), None),
    ]},
    PressureTracerRow { pressure_atm: 0.60, tracers: [
        available(None, Some(128)), unavailable(),
        available(Some(128), Some(64)), available(Some(64), None),
        available(Some(64), None),
    ]},
    PressureTracerRow { pressure_atm: 0.50, tracers: [
        available(None, Some(128)), unavailable(),
        available(Some(128), Some(64)), available(Some(128), Some(64)),
        available(Some(64), None),
    ]},
    PressureTracerRow { pressure_atm: 0.45, tracers: [
        unavailable(), unavailable(),
        available(Some(128), Some(64)), available(Some(128), None),
        no_data(),
    ]},
    PressureTracerRow { pressure_atm: 0.40, tracers: [
        unavailable(), unavailable(),
        available(Some(128), Some(64)), available(None, Some(128)),
        available(Some(128), Some(64)),
    ]},
    PressureTracerRow { pressure_atm: 0.35, tracers: [
        unavailable(), unavailable(),
        available(Some(128), Some(64)), available(None, Some(128)),
        available(Some(128), None),
    ]},
    PressureTracerRow { pressure_atm: 0.30, tracers: [
        unavailable(), unavailable(),
        available(Some(128), Some(64)), available(None, Some(128)),
        available(Some(128), None),
    ]},
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TracerScreenDisposition {
    FullFieldResolutionAvailable,
    EdgeCaseOnly,
    InsufficientResolution,
    UnavailableAtPressure,
    NoSourceData,
    UnsupportedSourcePressure,
    OutOfEnvelope,
    BoundaryLayerNotResolved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassivityStatus {
    NotAssessed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TracerScreenInput {
    pub pressure_atm: f64,
    pub tracer: TracerKind,
    /// Largest interrogation window acceptable for the intended use.
    /// A value of 32 requests finer resolution than 64 or 128.
    pub maximum_interrogation_window_px: u16,
    pub require_full_field: bool,
    pub require_boundary_layer_detail: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TracerScreenReport {
    pub pressure_atm: f64,
    pub tracer: TracerKind,
    pub source_pressure_atm: Option<f64>,
    pub evidence: Option<TracerResolutionEvidence>,
    pub requested_maximum_window_px: u16,
    pub require_full_field: bool,
    pub require_boundary_layer_detail: bool,
    pub disposition: TracerScreenDisposition,
    /// Optical feasibility does not establish electrical/passivity suitability.
    pub observer_perturbation_receipt_required: bool,
    pub passivity_status: PassivityStatus,
    /// This records resolution-screen eligibility only, never physical passivity.
    pub eligible_for_requested_resolution: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TracerScreenError {
    NonFinitePressure,
    InvalidMaximumWindow,
}

impl fmt::Display for TracerScreenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinitePressure => write!(f, "tracer-screen pressure must be finite"),
            Self::InvalidMaximumWindow => {
                write!(f, "maximum interrogation window must be a nonzero pixel size")
            }
        }
    }
}

impl Error for TracerScreenError {}

fn exact_source_row(pressure_atm: f64) -> Option<&'static PressureTracerRow> {
    SOURCE_PRESSURE_TRACER_TABLE.iter().find(|row| {
        (row.pressure_atm - pressure_atm).abs() <= PRESSURE_MATCH_TOLERANCE_ATM
    })
}

/// Screen one tracer at one exact pressure from the source's reported PIV
/// interrogation-area table. This selector deliberately does not interpolate.
pub fn screen_tracer_candidate(
    input: TracerScreenInput,
) -> Result<TracerScreenReport, TracerScreenError> {
    if !input.pressure_atm.is_finite() {
        return Err(TracerScreenError::NonFinitePressure);
    }
    if input.maximum_interrogation_window_px == 0 {
        return Err(TracerScreenError::InvalidMaximumWindow);
    }

    let mut source_pressure_atm = None;
    let mut evidence = None;
    let mut disposition = TracerScreenDisposition::UnsupportedSourcePressure;
    let mut eligible_for_requested_resolution = false;

    if !(0.30 - PRESSURE_MATCH_TOLERANCE_ATM..=1.00 + PRESSURE_MATCH_TOLERANCE_ATM)
        .contains(&input.pressure_atm)
    {
        disposition = TracerScreenDisposition::OutOfEnvelope;
    } else if let Some(row) = exact_source_row(input.pressure_atm) {
        source_pressure_atm = Some(row.pressure_atm);
        let selected = row.tracers[input.tracer.index()];
        evidence = Some(selected);

        disposition = match selected.status {
            TracerDataStatus::Unavailable => TracerScreenDisposition::UnavailableAtPressure,
            TracerDataStatus::NoData => TracerScreenDisposition::NoSourceData,
            TracerDataStatus::Available => {
                if input.require_boundary_layer_detail {
                    // The source explicitly says 32x32 px was insufficient for
                    // its boundary layer. Do not infer success from this table.
                    TracerScreenDisposition::BoundaryLayerNotResolved
                } else if selected.full_field_min_window_px.is_some_and(|window| {
                    window <= input.maximum_interrogation_window_px
                }) {
                    eligible_for_requested_resolution = true;
                    TracerScreenDisposition::FullFieldResolutionAvailable
                } else if selected.edge_case_min_window_px.is_some_and(|window| {
                    window <= input.maximum_interrogation_window_px
                }) {
                    eligible_for_requested_resolution = !input.require_full_field;
                    TracerScreenDisposition::EdgeCaseOnly
                } else {
                    TracerScreenDisposition::InsufficientResolution
                }
            }
        };
    }

    Ok(TracerScreenReport {
        pressure_atm: input.pressure_atm,
        tracer: input.tracer,
        source_pressure_atm,
        evidence,
        requested_maximum_window_px: input.maximum_interrogation_window_px,
        require_full_field: input.require_full_field,
        require_boundary_layer_detail: input.require_boundary_layer_detail,
        disposition,
        observer_perturbation_receipt_required: true,
        passivity_status: PassivityStatus::NotAssessed,
        eligible_for_requested_resolution,
    })
}

/// Manufactured self-checks prove exact lookup, edge-case handling and
/// fail-closed boundary-layer/passivity scope. No physical test is represented.
pub fn reference_self_check() -> Result<(), TracerScreenError> {
    let silica_low_pressure = screen_tracer_candidate(TracerScreenInput {
        pressure_atm: 0.30,
        tracer: TracerKind::Sio2,
        maximum_interrogation_window_px: 128,
        require_full_field: true,
        require_boundary_layer_detail: false,
    })?;
    if silica_low_pressure.disposition != TracerScreenDisposition::FullFieldResolutionAvailable
        || !silica_low_pressure.eligible_for_requested_resolution
        || !silica_low_pressure.observer_perturbation_receipt_required
        || silica_low_pressure.passivity_status != PassivityStatus::NotAssessed
    {
        return Err(TracerScreenError::InvalidMaximumWindow);
    }

    let fog_unavailable = screen_tracer_candidate(TracerScreenInput {
        pressure_atm: 0.50,
        tracer: TracerKind::VaporizedFog,
        maximum_interrogation_window_px: 128,
        require_full_field: true,
        require_boundary_layer_detail: false,
    })?;
    if fog_unavailable.disposition != TracerScreenDisposition::UnavailableAtPressure {
        return Err(TracerScreenError::InvalidMaximumWindow);
    }

    let incense_edge_only = screen_tracer_candidate(TracerScreenInput {
        pressure_atm: 0.30,
        tracer: TracerKind::IncenseSticks,
        maximum_interrogation_window_px: 64,
        require_full_field: true,
        require_boundary_layer_detail: false,
    })?;
    if incense_edge_only.disposition != TracerScreenDisposition::EdgeCaseOnly
        || incense_edge_only.eligible_for_requested_resolution
    {
        return Err(TracerScreenError::InvalidMaximumWindow);
    }

    let unmeasured_pressure = screen_tracer_candidate(TracerScreenInput {
        pressure_atm: 0.33,
        tracer: TracerKind::Sio2,
        maximum_interrogation_window_px: 128,
        require_full_field: true,
        require_boundary_layer_detail: false,
    })?;
    if unmeasured_pressure.disposition != TracerScreenDisposition::UnsupportedSourcePressure {
        return Err(TracerScreenError::InvalidMaximumWindow);
    }

    let boundary_layer = screen_tracer_candidate(TracerScreenInput {
        pressure_atm: 0.90,
        tracer: TracerKind::IncenseSticks,
        maximum_interrogation_window_px: 64,
        require_full_field: true,
        require_boundary_layer_detail: true,
    })?;
    if boundary_layer.disposition != TracerScreenDisposition::BoundaryLayerNotResolved {
        return Err(TracerScreenError::InvalidMaximumWindow);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(
        pressure_atm: f64,
        tracer: TracerKind,
        max_window: u16,
        require_full_field: bool,
        require_boundary_layer_detail: bool,
    ) -> TracerScreenInput {
        TracerScreenInput {
            pressure_atm,
            tracer,
            maximum_interrogation_window_px: max_window,
            require_full_field,
            require_boundary_layer_detail,
        }
    }

    #[test]
    fn silica_has_source_reported_full_field_feasibility_at_point_three_atm() {
        let report = screen_tracer_candidate(input(
            0.30, TracerKind::Sio2, 128, true, false,
        )).unwrap();
        assert_eq!(
            report.disposition,
            TracerScreenDisposition::FullFieldResolutionAvailable
        );
        assert_eq!(report.source_pressure_atm, Some(0.30));
        assert!(report.eligible_for_requested_resolution);
    }

    #[test]
    fn fog_is_unavailable_at_point_five_atm_in_the_source_table() {
        let report = screen_tracer_candidate(input(
            0.50, TracerKind::VaporizedFog, 128, true, false,
        )).unwrap();
        assert_eq!(report.disposition, TracerScreenDisposition::UnavailableAtPressure);
    }

    #[test]
    fn incense_sticks_at_point_three_atm_are_edge_case_only_for_sixty_four_px() {
        let report = screen_tracer_candidate(input(
            0.30, TracerKind::IncenseSticks, 64, true, false,
        )).unwrap();
        assert_eq!(report.disposition, TracerScreenDisposition::EdgeCaseOnly);
        assert!(!report.eligible_for_requested_resolution);
        assert_eq!(report.evidence.unwrap().full_field_min_window_px, Some(128));
    }

    #[test]
    fn dehs_is_unavailable_at_point_three_atm_in_the_source_table() {
        let report = screen_tracer_candidate(input(
            0.30, TracerKind::Dehs, 128, true, false,
        )).unwrap();
        assert_eq!(report.disposition, TracerScreenDisposition::UnavailableAtPressure);
    }

    #[test]
    fn edge_case_window_is_not_promoted_to_full_field_resolution() {
        let report = screen_tracer_candidate(input(
            0.80, TracerKind::VaporizedFog, 64, false, false,
        )).unwrap();
        assert_eq!(report.disposition, TracerScreenDisposition::EdgeCaseOnly);
        assert!(report.eligible_for_requested_resolution);
    }

    #[test]
    fn boundary_layer_request_abstains_even_when_velocity_field_is_source_feasible() {
        let report = screen_tracer_candidate(input(
            0.90, TracerKind::IncenseSticks, 64, true, true,
        )).unwrap();
        assert_eq!(report.disposition, TracerScreenDisposition::BoundaryLayerNotResolved);
        assert!(!report.eligible_for_requested_resolution);
    }

    #[test]
    fn pressure_between_source_points_is_not_interpolated() {
        let report = screen_tracer_candidate(input(
            0.33, TracerKind::Sio2, 128, true, false,
        )).unwrap();
        assert_eq!(
            report.disposition,
            TracerScreenDisposition::UnsupportedSourcePressure
        );
        assert!(report.evidence.is_none());
    }

    #[test]
    fn no_data_is_not_converted_to_unavailable_or_zero() {
        let report = screen_tracer_candidate(input(
            0.45, TracerKind::Sio2, 128, true, false,
        )).unwrap();
        assert_eq!(report.disposition, TracerScreenDisposition::NoSourceData);
    }

    #[test]
    fn optical_feasibility_never_qualifies_electrical_passivity() {
        let report = screen_tracer_candidate(input(
            0.30, TracerKind::Sio2, 128, true, false,
        )).unwrap();
        assert!(report.observer_perturbation_receipt_required);
        assert_eq!(report.passivity_status, PassivityStatus::NotAssessed);
    }

    #[test]
    fn pressure_outside_source_envelope_is_not_evaluated() {
        let report = screen_tracer_candidate(input(
            0.20, TracerKind::Sio2, 128, true, false,
        )).unwrap();
        assert_eq!(report.disposition, TracerScreenDisposition::OutOfEnvelope);
    }

    #[test]
    fn invalid_input_is_rejected() {
        assert_eq!(
            screen_tracer_candidate(input(
                f64::NAN, TracerKind::Sio2, 128, true, false,
            )).unwrap_err(),
            TracerScreenError::NonFinitePressure
        );
        assert_eq!(
            screen_tracer_candidate(input(
                0.30, TracerKind::Sio2, 0, true, false,
            )).unwrap_err(),
            TracerScreenError::InvalidMaximumWindow
        );
    }
}
