//! Fusion equilibrium interoperability contract, v0.1.
//!
//! This crate validates declared field semantics and performs only explicit,
//! reversible unit normalizations. It is not a Grad–Shafranov solver, an EFIT
//! validator, a plasma controller, or a physical qualification authority.

use std::f64::consts::TAU;

/// Contract version carried by every normalization receipt.
pub const CONTRACT_VERSION: &str = "fusion-equilibrium-interop/0.1";

/// Flags documented by FreeGSNKE for its internal COCOS-7-like convention.
/// This is not a complete arbitrary-COCOS conversion recipe.
pub const FREEGSNKE_DOCUMENTED_COCOS_FLAGS: CocosFlags = CocosFlags {
    exp_bp: 0,
    sigma_bp: -1,
    sigma_rp_z: 1,
    sigma_rhotp: 1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    StaticForward,
    StaticInverse,
    EvolvingForward,
    DatasetReconstruction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluxUnit {
    WeberPerRadian,
    Weber,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisUnit {
    Metres,
    Millimetres,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    Seconds,
    Milliseconds,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameLayout {
    RRowsZColumns,
    ZRowsRColumns,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinateBasis {
    CylindricalRPhiZRightHanded,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoloidalFieldDefinition {
    GradPsiCrossGradPhi,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CocosFlags {
    pub exp_bp: u8,
    pub sigma_bp: i8,
    pub sigma_rp_z: i8,
    pub sigma_rhotp: i8,
}

/// Explicit signs for quantities present in a source profile.
///
/// These are metadata, not values that this crate tries to infer or repair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuantitySigns {
    pub psi: i8,
    pub ip: i8,
    pub b_phi: i8,
    pub f: i8,
    pub q: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConventionProfile {
    pub coordinate_basis: CoordinateBasis,
    pub poloidal_field_definition: PoloidalFieldDefinition,
    pub cocos_flags: Option<CocosFlags>,
    pub quantity_signs: Option<QuantitySigns>,
}

/// Immutable declared identity for the convention definition consumed by an adapter.
/// The profile digest is metadata only; v0.1 does not fetch or hash its source bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConventionProfileRef {
    pub profile_id: String,
    pub profile_sha256: String,
    pub profile: ConventionProfile,
}

/// Complete declared identity and semantics for one equilibrium payload.
///
/// psi_frames is always expressed as frames -> R rows -> Z columns.
/// Other layouts are refused rather than silently transposed.
#[derive(Debug, Clone, PartialEq)]
pub struct EquilibriumInput {
    pub source_profile_id: String,
    pub producer_id: String,
    pub producer_revision: String,
    pub task_kind: TaskKind,
    pub geometry_sha256: String,
    pub source_artifact_sha256: String,
    pub axis_unit: AxisUnit,
    pub time_unit: TimeUnit,
    pub flux_unit: FluxUnit,
    pub frame_layout: FrameLayout,
    pub convention: ConventionProfile,
    pub r_axis: Vec<f64>,
    pub z_axis: Vec<f64>,
    pub timestamps: Vec<f64>,
    pub psi_frames: Vec<Vec<Vec<f64>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    EmptyIdentity(&'static str),
    InvalidSha256(&'static str),
    UnknownAxisUnit,
    UnknownTimeUnit,
    UnknownFluxUnit,
    UnsupportedFrameLayout,
    UnsupportedCoordinateBasis,
    UnsupportedPoloidalFieldDefinition,
    MissingCocosFlags,
    InvalidCocosFlags,
    MissingQuantitySigns,
    InvalidQuantitySigns,
    ConventionMismatch,
    EmptyAxis(&'static str),
    NonFiniteAxis(&'static str),
    NonPositiveMajorRadius,
    NonMonotonicAxis(&'static str),
    MissingFrames,
    StaticFrameCount,
    EvolvingFrameCount,
    TimestampCountMismatch,
    NonFiniteTimestamp,
    NonMonotonicTime,
    FrameShapeMismatch {
        frame: usize,
        r_rows: usize,
        expected_r_rows: usize,
        z_columns: usize,
        expected_z_columns: usize,
    },
    NonFiniteFlux {
        frame: usize,
        r_index: usize,
        z_index: usize,
    },
    StaleGeometryIdentity,
    StaleSourceArtifactIdentity,
    StaleProducerIdentity,
    StaleTaskIdentity,
    StaleConventionIdentity,
    StaleConventionProfileIdentity,
    StaleRepresentationIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluxTransform {
    Identity,
    DivideByTwoPi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthTransform {
    Identity,
    MillimetresToMetres,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeTransform {
    Identity,
    MillisecondsToSeconds,
}

/// Receipt records declared identities and exact transformations.
/// It does not claim that source bytes were fetched or independently hashed.
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizationReceipt {
    pub contract_version: &'static str,
    pub source_profile_id: String,
    pub producer_id: String,
    pub producer_revision: String,
    pub task_kind: TaskKind,
    pub geometry_sha256: String,
    pub source_artifact_sha256: String,
    /// Always false in v0.1: the crate checks declared identity, not raw source bytes.
    pub source_artifact_payload_verified: bool,
    pub convention_profile_id: String,
    pub convention_profile_sha256: String,
    /// Always false in v0.1: the adapter records the declared profile digest but does not fetch/hash its source bytes.
    pub convention_profile_payload_verified: bool,
    pub source_flux_unit: FluxUnit,
    pub source_axis_unit: AxisUnit,
    pub source_time_unit: TimeUnit,
    pub source_frame_layout: FrameLayout,
    pub convention: ConventionProfile,
    pub flux_transform: FluxTransform,
    pub length_transform: LengthTransform,
    pub time_transform: TimeTransform,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedEquilibrium {
    pub source_profile_id: String,
    pub producer_id: String,
    pub producer_revision: String,
    pub task_kind: TaskKind,
    pub convention: ConventionProfile,
    pub r_axis_m: Vec<f64>,
    pub z_axis_m: Vec<f64>,
    pub timestamps_s: Vec<f64>,
    pub psi_frames_wb_per_radian: Vec<Vec<Vec<f64>>>,
    pub geometry_sha256: String,
    pub source_artifact_sha256: String,
    pub receipt: NormalizationReceipt,
    pub claim_ceiling: ClaimCeiling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimCeiling {
    AdapterContractAndSyntheticFixtureOnly,
}

/// Normalize explicit units while preserving the complete convention
/// fingerprint. The expected convention must be supplied by the adapter
/// profile; this function never guesses it from a solver name or COCOS code.
pub fn normalize(
    input: &EquilibriumInput,
    expected_profile: &ConventionProfileRef,
) -> Result<NormalizedEquilibrium, Vec<ValidationError>> {
    let mut errors = Vec::new();

    require_nonempty(&input.source_profile_id, "source_profile_id", &mut errors);
    require_nonempty(&input.producer_id, "producer_id", &mut errors);
    require_nonempty(&input.producer_revision, "producer_revision", &mut errors);
    require_nonempty(&expected_profile.profile_id, "convention_profile_id", &mut errors);
    if !is_sha256(&expected_profile.profile_sha256) {
        errors.push(ValidationError::InvalidSha256("convention_profile_sha256"));
    }

    if !is_sha256(&input.geometry_sha256) {
        errors.push(ValidationError::InvalidSha256("geometry_sha256"));
    }
    if !is_sha256(&input.source_artifact_sha256) {
        errors.push(ValidationError::InvalidSha256("source_artifact_sha256"));
    }

    validate_convention(expected_profile.profile, &mut errors);
    if input.convention != expected_profile.profile {
        errors.push(ValidationError::ConventionMismatch);
    }

    let (length_scale, length_transform) = match input.axis_unit {
        AxisUnit::Metres => (1.0, LengthTransform::Identity),
        AxisUnit::Millimetres => (0.001, LengthTransform::MillimetresToMetres),
        AxisUnit::Unknown => {
            errors.push(ValidationError::UnknownAxisUnit);
            (1.0, LengthTransform::Identity)
        }
    };

    let (time_scale, time_transform) = match input.time_unit {
        TimeUnit::Seconds => (1.0, TimeTransform::Identity),
        TimeUnit::Milliseconds => (0.001, TimeTransform::MillisecondsToSeconds),
        TimeUnit::Unknown => {
            errors.push(ValidationError::UnknownTimeUnit);
            (1.0, TimeTransform::Identity)
        }
    };

    let (flux_scale, flux_transform) = match input.flux_unit {
        FluxUnit::WeberPerRadian => (1.0, FluxTransform::Identity),
        FluxUnit::Weber => (1.0 / TAU, FluxTransform::DivideByTwoPi),
        FluxUnit::Unknown => {
            errors.push(ValidationError::UnknownFluxUnit);
            (1.0, FluxTransform::Identity)
        }
    };

    if input.frame_layout != FrameLayout::RRowsZColumns {
        errors.push(ValidationError::UnsupportedFrameLayout);
    }

    let r_axis_m: Vec<f64> = input.r_axis.iter().map(|v| v * length_scale).collect();
    let z_axis_m: Vec<f64> = input.z_axis.iter().map(|v| v * length_scale).collect();
    let timestamps_s: Vec<f64> = input
        .timestamps
        .iter()
        .map(|v| v * time_scale)
        .collect();

    validate_axis(&r_axis_m, "R", true, &mut errors);
    validate_axis(&z_axis_m, "Z", false, &mut errors);

    if input.psi_frames.is_empty() {
        errors.push(ValidationError::MissingFrames);
    }

    if timestamps_s.len() != input.psi_frames.len() {
        errors.push(ValidationError::TimestampCountMismatch);
    }
    if timestamps_s.iter().any(|v| !v.is_finite()) {
        errors.push(ValidationError::NonFiniteTimestamp);
    }
    if timestamps_s.windows(2).any(|w| w[1] <= w[0]) {
        errors.push(ValidationError::NonMonotonicTime);
    }

    match input.task_kind {
        TaskKind::StaticForward | TaskKind::StaticInverse => {
            if input.psi_frames.len() != 1 {
                errors.push(ValidationError::StaticFrameCount);
            }
        }
        TaskKind::EvolvingForward => {
            if input.psi_frames.len() < 2 {
                errors.push(ValidationError::EvolvingFrameCount);
            }
        }
        TaskKind::DatasetReconstruction => {
            if input.psi_frames.is_empty() {
                errors.push(ValidationError::MissingFrames);
            }
        }
    }

    for (frame_index, frame) in input.psi_frames.iter().enumerate() {
        if frame.len() != r_axis_m.len()
            || frame.iter().any(|row| row.len() != z_axis_m.len())
        {
            errors.push(ValidationError::FrameShapeMismatch {
                frame: frame_index,
                r_rows: frame.len(),
                expected_r_rows: r_axis_m.len(),
                z_columns: frame.first().map_or(0, Vec::len),
                expected_z_columns: z_axis_m.len(),
            });
        }

        for (r_index, row) in frame.iter().enumerate() {
            for (z_index, value) in row.iter().enumerate() {
                if !value.is_finite() {
                    errors.push(ValidationError::NonFiniteFlux {
                        frame: frame_index,
                        r_index,
                        z_index,
                    });
                }
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let psi_frames_wb_per_radian = input
        .psi_frames
        .iter()
        .map(|frame| {
            frame
                .iter()
                .map(|row| row.iter().map(|value| value * flux_scale).collect())
                .collect()
        })
        .collect();

    let receipt = NormalizationReceipt {
        contract_version: CONTRACT_VERSION,
        source_profile_id: input.source_profile_id.clone(),
        producer_id: input.producer_id.clone(),
        producer_revision: input.producer_revision.clone(),
        task_kind: input.task_kind,
        geometry_sha256: input.geometry_sha256.clone(),
        source_artifact_sha256: input.source_artifact_sha256.clone(),
        source_artifact_payload_verified: false,
        convention_profile_id: expected_profile.profile_id.clone(),
        convention_profile_sha256: expected_profile.profile_sha256.clone(),
        convention_profile_payload_verified: false,
        source_flux_unit: input.flux_unit,
        source_axis_unit: input.axis_unit,
        source_time_unit: input.time_unit,
        source_frame_layout: input.frame_layout,
        convention: input.convention,
        flux_transform,
        length_transform,
        time_transform,
    };

    Ok(NormalizedEquilibrium {
        source_profile_id: input.source_profile_id.clone(),
        producer_id: input.producer_id.clone(),
        producer_revision: input.producer_revision.clone(),
        task_kind: input.task_kind,
        convention: input.convention,
        r_axis_m,
        z_axis_m,
        timestamps_s,
        psi_frames_wb_per_radian,
        geometry_sha256: input.geometry_sha256.clone(),
        source_artifact_sha256: input.source_artifact_sha256.clone(),
        receipt,
        claim_ceiling: ClaimCeiling::AdapterContractAndSyntheticFixtureOnly,
    })
}

/// Check that an existing normalization receipt still names the exact source
/// identity and representation. This checks declared metadata, not source bytes.
pub fn verify_receipt(
    input: &EquilibriumInput,
    expected_profile: &ConventionProfileRef,
    receipt: &NormalizationReceipt,
) -> Result<(), ValidationError> {
    if input.geometry_sha256 != receipt.geometry_sha256 {
        return Err(ValidationError::StaleGeometryIdentity);
    }
    if input.source_artifact_sha256 != receipt.source_artifact_sha256 {
        return Err(ValidationError::StaleSourceArtifactIdentity);
    }
    if input.source_profile_id != receipt.source_profile_id
        || input.producer_id != receipt.producer_id
        || input.producer_revision != receipt.producer_revision
    {
        return Err(ValidationError::StaleProducerIdentity);
    }
    if input.task_kind != receipt.task_kind {
        return Err(ValidationError::StaleTaskIdentity);
    }
    if input.convention != receipt.convention {
        return Err(ValidationError::StaleConventionIdentity);
    }
    if expected_profile.profile_id != receipt.convention_profile_id
        || expected_profile.profile_sha256 != receipt.convention_profile_sha256
        || expected_profile.profile != receipt.convention
        || !is_sha256(&expected_profile.profile_sha256)
    {
        return Err(ValidationError::StaleConventionProfileIdentity);
    }
    if input.flux_unit != receipt.source_flux_unit
        || input.axis_unit != receipt.source_axis_unit
        || input.time_unit != receipt.source_time_unit
        || input.frame_layout != receipt.source_frame_layout
    {
        return Err(ValidationError::StaleRepresentationIdentity);
    }
    Ok(())
}

fn validate_convention(convention: ConventionProfile, errors: &mut Vec<ValidationError>) {
    if convention.coordinate_basis != CoordinateBasis::CylindricalRPhiZRightHanded {
        errors.push(ValidationError::UnsupportedCoordinateBasis);
    }
    if convention.poloidal_field_definition != PoloidalFieldDefinition::GradPsiCrossGradPhi {
        errors.push(ValidationError::UnsupportedPoloidalFieldDefinition);
    }

    match convention.cocos_flags {
        None => errors.push(ValidationError::MissingCocosFlags),
        Some(flags)
            if flags.exp_bp > 1
                || !is_sign(flags.sigma_bp)
                || !is_sign(flags.sigma_rp_z)
                || !is_sign(flags.sigma_rhotp) =>
        {
            errors.push(ValidationError::InvalidCocosFlags);
        }
        Some(_) => {}
    }

    match convention.quantity_signs {
        None => errors.push(ValidationError::MissingQuantitySigns),
        Some(signs)
            if !is_sign(signs.psi)
                || !is_sign(signs.ip)
                || !is_sign(signs.b_phi)
                || !is_sign(signs.f)
                || !is_sign(signs.q) =>
        {
            errors.push(ValidationError::InvalidQuantitySigns);
        }
        Some(_) => {}
    }
}

fn validate_axis(
    values: &[f64],
    name: &'static str,
    require_positive: bool,
    errors: &mut Vec<ValidationError>,
) {
    if values.is_empty() {
        errors.push(ValidationError::EmptyAxis(name));
        return;
    }
    if values.iter().any(|v| !v.is_finite()) {
        errors.push(ValidationError::NonFiniteAxis(name));
    }
    if require_positive && values.iter().any(|v| *v <= 0.0) {
        errors.push(ValidationError::NonPositiveMajorRadius);
    }
    if values.windows(2).any(|w| w[1] <= w[0]) {
        errors.push(ValidationError::NonMonotonicAxis(name));
    }
}

fn require_nonempty(value: &str, field: &'static str, errors: &mut Vec<ValidationError>) {
    if value.trim().is_empty() {
        errors.push(ValidationError::EmptyIdentity(field));
    }
}

fn is_sign(value: i8) -> bool {
    value == -1 || value == 1
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convention() -> ConventionProfile {
        ConventionProfile {
            coordinate_basis: CoordinateBasis::CylindricalRPhiZRightHanded,
            poloidal_field_definition: PoloidalFieldDefinition::GradPsiCrossGradPhi,
            cocos_flags: Some(FREEGSNKE_DOCUMENTED_COCOS_FLAGS),
            // Synthetic explicit sign metadata; not inferred from the solver name.
            quantity_signs: Some(QuantitySigns {
                psi: 1,
                ip: 1,
                b_phi: 1,
                f: 1,
                q: 1,
            }),
        }
    }

    fn expected_profile() -> ConventionProfileRef {
        ConventionProfileRef {
            profile_id: "synthetic/freegsnke-3.1.1-convention-v1".into(),
            profile_sha256: "f".repeat(64),
            profile: convention(),
        }
    }

    fn input() -> EquilibriumInput {
        EquilibriumInput {
            source_profile_id: "synthetic/freegsnke-like-v1".into(),
            producer_id: "FreeGSNKE".into(),
            producer_revision: "3.1.1".into(),
            task_kind: TaskKind::StaticForward,
            geometry_sha256: "a".repeat(64),
            source_artifact_sha256: "b".repeat(64),
            axis_unit: AxisUnit::Metres,
            time_unit: TimeUnit::Seconds,
            flux_unit: FluxUnit::WeberPerRadian,
            frame_layout: FrameLayout::RRowsZColumns,
            convention: convention(),
            r_axis: vec![1.0, 2.0],
            z_axis: vec![-1.0, 1.0],
            timestamps: vec![0.0],
            psi_frames: vec![vec![vec![0.1, 0.2], vec![0.3, 0.4]]],
        }
    }

    fn contains_error(errors: &[ValidationError], expected: ValidationError) -> bool {
        errors.contains(&expected)
    }

    #[test]
    fn documented_freegsnke_flags_are_explicit() {
        assert_eq!(
            FREEGSNKE_DOCUMENTED_COCOS_FLAGS,
            CocosFlags {
                exp_bp: 0,
                sigma_bp: -1,
                sigma_rp_z: 1,
                sigma_rhotp: 1,
            }
        );
    }

    #[test]
    fn wb_per_radian_passes_without_scaling() {
        let normalized = normalize(&input(), &expected_profile()).unwrap();
        assert_eq!(
            normalized.receipt.flux_transform,
            FluxTransform::Identity
        );
        assert_eq!(normalized.psi_frames_wb_per_radian[0][0][0], 0.1);
        assert_eq!(
            normalized.claim_ceiling,
            ClaimCeiling::AdapterContractAndSyntheticFixtureOnly
        );
    }

    #[test]
    fn declared_digest_is_not_misrepresented_as_payload_verification() {
        let normalized = normalize(&input(), convention()).unwrap();
        assert!(!normalized.receipt.source_artifact_payload_verified);
    }

    #[test]
    fn convention_profile_identity_is_bound_but_payload_is_not_verified() {
        let normalized = normalize(&input(), &expected_profile()).unwrap();
        assert_eq!(
            normalized.receipt.convention_profile_id,
            "synthetic/freegsnke-3.1.1-convention-v1"
        );
        assert!(!normalized.receipt.convention_profile_payload_verified);
    }

    #[test]
    fn verify_receipt_rejects_a_changed_convention_profile_identity() {
        let source = input();
        let profile = expected_profile();
        let normalized = normalize(&source, &profile).unwrap();
        let mut changed_profile = profile;
        changed_profile.profile_sha256 = "e".repeat(64);
        assert_eq!(
            verify_receipt(&source, &changed_profile, &normalized.receipt),
            Err(ValidationError::StaleConventionProfileIdentity)
        );
    }

    #[test]
    fn explicitly_declared_weber_flux_is_divided_by_two_pi_once() {
        let mut source = input();
        source.flux_unit = FluxUnit::Weber;
        source.psi_frames[0][0][0] = TAU;
        let normalized = normalize(&source, &expected_profile()).unwrap();
        assert_eq!(
            normalized.receipt.flux_transform,
            FluxTransform::DivideByTwoPi
        );
        assert!((normalized.psi_frames_wb_per_radian[0][0][0] - 1.0).abs() < 1e-14);
    }

    #[test]
    fn explicit_millimetres_and_milliseconds_are_normalized() {
        let mut source = input();
        source.axis_unit = AxisUnit::Millimetres;
        source.r_axis = vec![1000.0, 2000.0];
        source.z_axis = vec![-1000.0, 1000.0];
        source.time_unit = TimeUnit::Milliseconds;
        source.timestamps = vec![250.0];

        let normalized = normalize(&source, convention()).unwrap();
        assert_eq!(normalized.r_axis_m, vec![1.0, 2.0]);
        assert_eq!(normalized.z_axis_m, vec![-1.0, 1.0]);
        assert_eq!(normalized.timestamps_s, vec![0.25]);
    }

    #[test]
    fn unknown_flux_unit_is_rejected() {
        let mut source = input();
        source.flux_unit = FluxUnit::Unknown;
        let errors = normalize(&source, convention()).unwrap_err();
        assert!(contains_error(&errors, ValidationError::UnknownFluxUnit));
    }

    #[test]
    fn missing_cocos_flags_are_rejected() {
        let mut source = input();
        source.convention.cocos_flags = None;
        let errors = normalize(&source, convention()).unwrap_err();
        assert!(contains_error(&errors, ValidationError::MissingCocosFlags));
    }

    #[test]
    fn missing_quantity_signs_are_rejected() {
        let mut source = input();
        source.convention.quantity_signs = None;
        let errors = normalize(&source, convention()).unwrap_err();
        assert!(contains_error(&errors, ValidationError::MissingQuantitySigns));
    }

    #[test]
    fn sign_or_cocos_mismatch_is_not_silently_corrected() {
        let mut source = input();
        source.convention.quantity_signs.as_mut().unwrap().q = -1;
        let errors = normalize(&source, convention()).unwrap_err();
        assert!(contains_error(&errors, ValidationError::ConventionMismatch));
    }

    #[test]
    fn transposed_grid_layout_is_rejected_instead_of_reordered() {
        let mut source = input();
        source.frame_layout = FrameLayout::ZRowsRColumns;
        let errors = normalize(&source, convention()).unwrap_err();
        assert!(contains_error(&errors, ValidationError::UnsupportedFrameLayout));
    }

    #[test]
    fn non_monotonic_r_axis_is_rejected() {
        let mut source = input();
        source.r_axis = vec![2.0, 1.0];
        let errors = normalize(&source, convention()).unwrap_err();
        assert!(contains_error(
            &errors,
            ValidationError::NonMonotonicAxis("R")
        ));
    }

    #[test]
    fn non_monotonic_z_axis_is_rejected() {
        let mut source = input();
        source.z_axis = vec![1.0, -1.0];
        let errors = normalize(&source, convention()).unwrap_err();
        assert!(contains_error(
            &errors,
            ValidationError::NonMonotonicAxis("Z")
        ));
    }

    #[test]
    fn malformed_frame_shape_is_rejected() {
        let mut source = input();
        source.psi_frames[0][0].pop();
        let errors = normalize(&source, convention()).unwrap_err();
        assert!(errors.iter().any(|error| matches!(
            error,
            ValidationError::FrameShapeMismatch { frame: 0, .. }
        )));
    }

    #[test]
    fn non_finite_flux_is_rejected() {
        let mut source = input();
        source.psi_frames[0][0][0] = f64::NAN;
        let errors = normalize(&source, convention()).unwrap_err();
        assert!(errors.iter().any(|error| matches!(
            error,
            ValidationError::NonFiniteFlux { frame: 0, .. }
        )));
    }

    #[test]
    fn static_task_with_multiple_frames_is_rejected() {
        let mut source = input();
        source.psi_frames.push(source.psi_frames[0].clone());
        source.timestamps.push(1.0);
        let errors = normalize(&source, convention()).unwrap_err();
        assert!(contains_error(&errors, ValidationError::StaticFrameCount));
    }

    #[test]
    fn evolving_task_requires_at_least_two_frames() {
        let mut source = input();
        source.task_kind = TaskKind::EvolvingForward;
        let errors = normalize(&source, convention()).unwrap_err();
        assert!(contains_error(&errors, ValidationError::EvolvingFrameCount));
    }

    #[test]
    fn non_monotonic_timestamps_are_rejected() {
        let mut source = input();
        source.task_kind = TaskKind::DatasetReconstruction;
        source.psi_frames.push(source.psi_frames[0].clone());
        source.timestamps = vec![1.0, 0.5];
        let errors = normalize(&source, convention()).unwrap_err();
        assert!(contains_error(&errors, ValidationError::NonMonotonicTime));
    }

    #[test]
    fn receipt_rejects_changed_geometry_identity() {
        let source = input();
        let normalized = normalize(&source, convention()).unwrap();
        let mut changed = source;
        changed.geometry_sha256 = "c".repeat(64);
        assert_eq!(
            verify_receipt(&changed, &expected_profile(), &normalized.receipt),
            Err(ValidationError::StaleGeometryIdentity)
        );
    }

    #[test]
    fn receipt_rejects_changed_source_artifact_identity() {
        let source = input();
        let normalized = normalize(&source, convention()).unwrap();
        let mut changed = source;
        changed.source_artifact_sha256 = "d".repeat(64);
        assert_eq!(
            verify_receipt(&changed, &normalized.receipt),
            Err(ValidationError::StaleSourceArtifactIdentity)
        );
    }

    #[test]
    fn dataset_reconstruction_keeps_multiple_irregular_times() {
        let mut source = input();
        source.task_kind = TaskKind::DatasetReconstruction;
        source.timestamps = vec![0.0, 0.013];
        source.psi_frames.push(source.psi_frames[0].clone());
        let normalized = normalize(&source, convention()).unwrap();
        assert_eq!(normalized.timestamps_s, vec![0.0, 0.013]);
        assert_eq!(normalized.psi_frames_wb_per_radian.len(), 2);
    }
}
