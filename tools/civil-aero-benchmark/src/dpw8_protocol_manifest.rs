//! Source-aware DPW-8 ONERA OAT15A Verification 1 protocol manifests.
//!
//! V1a (steady polar), V1b (unsteady buffet polar) and V1c (scatter-reduction reprise)
//! are distinct protocol lanes. This module freezes the public protocol facts but does not
//! admit external source bytes, prove solver results, or establish physical validation.

use crate::integrity::sha256_hex;

pub const DPW8_PROTOCOL_MANIFEST_SCHEMA: &str = "dpw8-oat15a-protocol-manifest/0.1.0";
pub const DPW8_PROTOCOL_EVIDENCE_LEVEL: &str = "canonical_protocol_spec_and_source_admission_state_not_solver_execution_or_physical_validation";

pub const DPW8_TEST_CASES_URL: &str = "https://www.aiaa-dpw.org/TestCases/testcases.html";
pub const DPW8_GEOMETRY_POLICY_URL: &str = "https://www.aiaa-dpw.org/geometry.html";
pub const DPW8_GRID_INDEX_URL: &str = "https://dpw.larc.nasa.gov/DPW8/ONERA_OAT15A/";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Dpw8Oat15aLane {
    V1aSteadyGridConvergencePolar,
    V1bUnsteadyExtendedPolarBuffet,
    V1cScatterReductionReprise,
}

impl Dpw8Oat15aLane {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::V1aSteadyGridConvergencePolar => "V1a",
            Self::V1bUnsteadyExtendedPolarBuffet => "V1b",
            Self::V1cScatterReductionReprise => "V1c",
        }
    }

    fn flow_mode(self) -> &'static str {
        match self {
            Self::V1aSteadyGridConvergencePolar => "steady",
            Self::V1bUnsteadyExtendedPolarBuffet => "unsteady",
            Self::V1cScatterReductionReprise => "steady_scatter_reproduction",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dpw8AngleSchedule {
    /// Exact angle list is specified by the official V1a/V1b case definition.
    ExactOfficialList,
    /// The currently admitted public V1c test-case page does not define a separate AoA list.
    NotSpecifiedByThisSource,
}

impl Dpw8AngleSchedule {
    fn as_str(self) -> &'static str {
        match self {
            Self::ExactOfficialList => "exact_official_list",
            Self::NotSpecifiedByThisSource => "not_specified_by_this_source",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dpw8GridFamilyPolicy {
    SixLevelsFourRequiredSixDesirable,
    UpdatedFamilyDefinitionMustBeAdmitted,
}

impl Dpw8GridFamilyPolicy {
    fn as_str(self) -> &'static str {
        match self {
            Self::SixLevelsFourRequiredSixDesirable => "six_levels_four_required_six_desirable",
            Self::UpdatedFamilyDefinitionMustBeAdmitted => {
                "updated_family_definition_must_be_admitted"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dpw8WallThermalCondition {
    Adiabatic,
    Isothermal,
}

impl Dpw8WallThermalCondition {
    fn as_str(self) -> &'static str {
        match self {
            Self::Adiabatic => "adiabatic",
            Self::Isothermal => "isothermal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dpw8SourceAdmissionState {
    NotAdmitted,
    BytePinnedReferenceOnly,
    BytePinnedRedistributionPermissionDocumentReferenced,
    PermissionPending,
}

impl Dpw8SourceAdmissionState {
    fn as_str(self) -> &'static str {
        match self {
            Self::NotAdmitted => "not_admitted",
            Self::BytePinnedReferenceOnly => "byte_pinned_reference_only",
            Self::BytePinnedRedistributionPermissionDocumentReferenced => {
                "byte_pinned_redistribution_permission_document_referenced"
            }
            Self::PermissionPending => "permission_pending",
        }
    }

    fn is_byte_pinned(self) -> bool {
        matches!(
            self,
            Self::BytePinnedReferenceOnly
                | Self::BytePinnedRedistributionPermissionDocumentReferenced
        )
    }

    /// This only records that a permission-document reference was supplied. It is not a legal
    /// determination, a signature check, or proof that the referenced document grants permission.
    fn has_permission_document_reference(self) -> bool {
        self == Self::BytePinnedRedistributionPermissionDocumentReferenced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dpw8SourceAdmission {
    pub source_id: String,
    pub source_url: String,
    pub source_kind: String,
    pub state: Dpw8SourceAdmissionState,
    pub byte_length: Option<u64>,
    pub sha256: Option<String>,
    /// Git commit, source revision, published revision or explicit "web page snapshot" identifier.
    pub source_revision: String,
    /// Human-readable usage/redistribution condition. This is recorded policy metadata, not a
    /// license verifier; actual permission remains a separately adjudicated source-admission step.
    pub license_applicability: String,
    pub acknowledgement: String,
    /// Optional source reference for a permission document; its bytes are separately digest-bound.
    pub permission_evidence_reference: Option<String>,
    pub permission_evidence_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dpw8Oat15aProtocolSpec {
    pub lane: Dpw8Oat15aLane,
    pub flow_mode: String,
    pub angle_schedule: Dpw8AngleSchedule,
    pub angle_of_attack_degrees: Vec<f64>,
    pub mach: Option<f64>,
    pub chord_reynolds: Option<f64>,
    pub static_temperature_kelvin: Option<f64>,
    pub grid_policy: Dpw8GridFamilyPolicy,
    pub sidewall_boundary_condition: String,
    pub common_farfield_chords: Option<f64>,
    pub turbulence_model: Option<String>,
    pub sutherland_law: Option<bool>,
    pub wall_thermal_condition: Option<Dpw8WallThermalCondition>,
    pub residual_target_log10: Option<f64>,
    pub gamma: Option<f64>,
    pub prandtl: Option<f64>,
    pub turbulent_prandtl: Option<f64>,
    /// The public page writes χ = ν / ν~ = 3. Preserve the native notation and meaning.
    pub farfield_kinematic_viscosity_ratio_chi: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dpw8Oat15aProtocolManifest {
    pub schema: String,
    pub protocol_revision: String,
    /// Outer lane identity, independently bound to the nested solver/settings specification.
    pub protocol_lane: Dpw8Oat15aLane,
    pub geometry_designation: String,
    pub geometry_ip_holder: String,
    pub geometry_acknowledgement_required_each_share: bool,
    pub geometry_share_condition: String,
    pub sources: Vec<Dpw8SourceAdmission>,
    pub spec: Dpw8Oat15aProtocolSpec,
    pub manifest_sha256: String,
    pub evidence_level: String,
}

fn v1a_angles() -> Vec<f64> {
    vec![1.36, 1.50, 2.50, 3.00, 3.10]
}

fn v1b_angles() -> Vec<f64> {
    vec![3.25, 3.40, 3.50, 3.60, 3.90]
}

fn source_shell(
    source_id: &str,
    source_url: &str,
    source_kind: &str,
    acknowledgement: &str,
    license_applicability: &str,
) -> Dpw8SourceAdmission {
    Dpw8SourceAdmission {
        source_id: source_id.to_owned(),
        source_url: source_url.to_owned(),
        source_kind: source_kind.to_owned(),
        state: Dpw8SourceAdmissionState::NotAdmitted,
        byte_length: None,
        sha256: None,
        source_revision: "not-yet-byte-pinned".to_owned(),
        license_applicability: license_applicability.to_owned(),
        acknowledgement: acknowledgement.to_owned(),
        permission_evidence_reference: None,
        permission_evidence_sha256: None,
    }
}

fn initial_sources() -> Vec<Dpw8SourceAdmission> {
    vec![
        source_shell(
            "dpw8-test-case-definition",
            DPW8_TEST_CASES_URL,
            "official_protocol_definition",
            "AIAA Drag Prediction Workshop",
            "Official public protocol page; snapshot and revision still require byte admission",
        ),
        source_shell(
            "onera-oat15a-geometry-policy",
            DPW8_GEOMETRY_POLICY_URL,
            "geometry_ip_and_acknowledgement_policy",
            "ONERA",
            "ONERA retains geometry IP; identify as ONERA OAT15A; acknowledge on every share and include the conditions when sharing",
        ),
        source_shell(
            "dpw8-onera-oat15a-grid-index",
            DPW8_GRID_INDEX_URL,
            "official_grid_and_data_index",
            "AIAA Drag Prediction Workshop / NASA DPW-8 data hosting",
            "Grid/data directory index; each actual grid/data artifact requires its own byte digest and applicability review",
        ),
    ]
}

pub fn dpw8_oat15a_v1a_spec() -> Dpw8Oat15aProtocolSpec {
    Dpw8Oat15aProtocolSpec {
        lane: Dpw8Oat15aLane::V1aSteadyGridConvergencePolar,
        flow_mode: "steady".to_owned(),
        angle_schedule: Dpw8AngleSchedule::ExactOfficialList,
        angle_of_attack_degrees: v1a_angles(),
        mach: Some(0.73),
        chord_reynolds: Some(3.0e6),
        static_temperature_kelvin: Some(271.0),
        grid_policy: Dpw8GridFamilyPolicy::SixLevelsFourRequiredSixDesirable,
        sidewall_boundary_condition: "periodic".to_owned(),
        common_farfield_chords: None,
        turbulence_model: None,
        sutherland_law: None,
        wall_thermal_condition: None,
        residual_target_log10: None,
        gamma: None,
        prandtl: None,
        turbulent_prandtl: None,
        farfield_kinematic_viscosity_ratio_chi: None,
    }
}

pub fn dpw8_oat15a_v1b_spec() -> Dpw8Oat15aProtocolSpec {
    Dpw8Oat15aProtocolSpec {
        lane: Dpw8Oat15aLane::V1bUnsteadyExtendedPolarBuffet,
        flow_mode: "unsteady".to_owned(),
        angle_schedule: Dpw8AngleSchedule::ExactOfficialList,
        angle_of_attack_degrees: v1b_angles(),
        mach: Some(0.73),
        chord_reynolds: Some(3.0e6),
        static_temperature_kelvin: Some(271.0),
        grid_policy: Dpw8GridFamilyPolicy::SixLevelsFourRequiredSixDesirable,
        sidewall_boundary_condition: "periodic".to_owned(),
        common_farfield_chords: None,
        turbulence_model: None,
        sutherland_law: None,
        wall_thermal_condition: None,
        residual_target_log10: None,
        gamma: None,
        prandtl: None,
        turbulent_prandtl: None,
        farfield_kinematic_viscosity_ratio_chi: None,
    }
}

pub fn dpw8_oat15a_v1c_spec() -> Dpw8Oat15aProtocolSpec {
    Dpw8Oat15aProtocolSpec {
        lane: Dpw8Oat15aLane::V1cScatterReductionReprise,
        flow_mode: "steady_scatter_reproduction".to_owned(),
        angle_schedule: Dpw8AngleSchedule::NotSpecifiedByThisSource,
        angle_of_attack_degrees: Vec::new(),
        mach: None,
        chord_reynolds: None,
        static_temperature_kelvin: None,
        grid_policy: Dpw8GridFamilyPolicy::UpdatedFamilyDefinitionMustBeAdmitted,
        sidewall_boundary_condition: "periodic".to_owned(),
        common_farfield_chords: Some(1000.0),
        turbulence_model: Some("Spalart-Allmaras negative variant, all terms".to_owned()),
        sutherland_law: Some(true),
        wall_thermal_condition: Some(Dpw8WallThermalCondition::Adiabatic),
        residual_target_log10: Some(-10.0),
        gamma: Some(1.4),
        prandtl: Some(0.72),
        turbulent_prandtl: Some(0.9),
        farfield_kinematic_viscosity_ratio_chi: Some(3.0),
    }
}

impl Dpw8Oat15aProtocolManifest {
    pub fn new(protocol_revision: impl Into<String>, lane: Dpw8Oat15aLane) -> Self {
        let spec = match lane {
            Dpw8Oat15aLane::V1aSteadyGridConvergencePolar => dpw8_oat15a_v1a_spec(),
            Dpw8Oat15aLane::V1bUnsteadyExtendedPolarBuffet => dpw8_oat15a_v1b_spec(),
            Dpw8Oat15aLane::V1cScatterReductionReprise => dpw8_oat15a_v1c_spec(),
        };
        Self {
            schema: DPW8_PROTOCOL_MANIFEST_SCHEMA.to_owned(),
            protocol_revision: protocol_revision.into(),
            protocol_lane: lane,
            geometry_designation: "ONERA OAT15A".to_owned(),
            geometry_ip_holder: "ONERA".to_owned(),
            geometry_acknowledgement_required_each_share: true,
            geometry_share_condition: "ONERA retains intellectual property; use designation ONERA OAT15A; acknowledge ONERA each time shared with a partner; include these conditions when sharing".to_owned(),
            sources: initial_sources(),
            spec,
            manifest_sha256: String::new(),
            evidence_level: DPW8_PROTOCOL_EVIDENCE_LEVEL.to_owned(),
        }
    }

    /// This reports protocol-source readiness only. Per-grid/experimental artifact admission and
    /// actual solver runs are separate gates.
    pub fn protocol_sources_byte_pinned(&self) -> bool {
        self.sources
            .iter()
            .all(|source| source.state.is_byte_pinned())
    }

    /// Geometry redistribution is allowed only when the geometry source itself was admitted
    /// under an explicit redistribution permission state.
    /// True only when a permission-document reference and digest are recorded. This is not
    /// legal authorization; an independent person must review the referenced terms.
    pub fn geometry_permission_document_referenced(&self) -> bool {
        self.sources.iter().any(|source| {
            source.source_kind == "geometry_ip_and_acknowledgement_policy"
                && source.state.has_permission_document_reference()
                && source
                    .permission_evidence_reference
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                && source
                    .permission_evidence_sha256
                    .as_deref()
                    .is_some_and(is_digest)
        })
    }
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && value == value.to_ascii_lowercase()
}

fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    append_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn append_f64_option(bytes: &mut Vec<u8>, value: Option<f64>) {
    match value {
        Some(value) => {
            bytes.push(1);
            bytes.extend_from_slice(&value.to_bits().to_le_bytes());
        }
        None => bytes.push(0),
    }
}

fn validate_source(source: &Dpw8SourceAdmission) -> Result<(), String> {
    for (name, value) in [
        ("source_id", source.source_id.as_str()),
        ("source_url", source.source_url.as_str()),
        ("source_kind", source.source_kind.as_str()),
        ("source_revision", source.source_revision.as_str()),
        (
            "license_applicability",
            source.license_applicability.as_str(),
        ),
        ("acknowledgement", source.acknowledgement.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("DPW-8 source {name} must not be empty"));
        }
    }
    match source.state {
        Dpw8SourceAdmissionState::NotAdmitted => {
            if source.byte_length.is_some()
                || source.sha256.is_some()
                || source.permission_evidence_reference.is_some()
                || source.permission_evidence_sha256.is_some()
            {
                return Err(format!(
                    "DPW-8 source {} is not admitted but contains byte or permission metadata",
                    source.source_id
                ));
            }
        }
        Dpw8SourceAdmissionState::PermissionPending => {
            if source.permission_evidence_reference.is_some()
                || source.permission_evidence_sha256.is_some()
            {
                return Err(format!(
                    "DPW-8 source {} is permission-pending and cannot carry a permission-document admission reference",
                    source.source_id
                ));
            }
            match (source.byte_length, source.sha256.as_deref()) {
                (None, None) => {}
                (Some(length), Some(value))
                    if length > 0
                        && is_digest(value)
                        && source.source_revision != "not-yet-byte-pinned" => {}
                _ => {
                    return Err(format!(
                        "DPW-8 source {} permission-pending byte metadata must be entirely absent or fully pinned",
                        source.source_id
                    ));
                }
            }
        }
        Dpw8SourceAdmissionState::BytePinnedReferenceOnly => {
            if source.byte_length.is_none_or(|length| length == 0)
                || source
                    .sha256
                    .as_deref()
                    .is_none_or(|value| !is_digest(value))
                || source.source_revision == "not-yet-byte-pinned"
                || source.permission_evidence_reference.is_some()
                || source.permission_evidence_sha256.is_some()
            {
                return Err(format!(
                    "DPW-8 source {} reference-only admission has malformed byte or permission metadata",
                    source.source_id
                ));
            }
        }
        Dpw8SourceAdmissionState::BytePinnedRedistributionPermissionDocumentReferenced => {
            if source.byte_length.is_none_or(|length| length == 0)
                || source
                    .sha256
                    .as_deref()
                    .is_none_or(|value| !is_digest(value))
                || source.source_revision == "not-yet-byte-pinned"
                || source
                    .permission_evidence_reference
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                || source
                    .permission_evidence_sha256
                    .as_deref()
                    .is_none_or(|value| !is_digest(value))
            {
                return Err(format!(
                    "DPW-8 source {} has no complete pinned permission-document reference",
                    source.source_id
                ));
            }
        }
    }
    if source.source_kind == "geometry_ip_and_acknowledgement_policy"
        && (source.acknowledgement != "ONERA"
            || !source.license_applicability.contains("ONERA retains"))
    {
        return Err(
            "ONERA geometry source must retain ONERA attribution and IP conditions".to_owned(),
        );
    }
    Ok(())
}

fn validate_spec(spec: &Dpw8Oat15aProtocolSpec) -> Result<(), String> {
    if spec.flow_mode != spec.lane.flow_mode() {
        return Err("DPW-8 lane and flow-mode identity disagree".to_owned());
    }
    if spec
        .angle_of_attack_degrees
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err("DPW-8 angle-of-attack list contains a non-finite value".to_owned());
    }
    if spec.sidewall_boundary_condition.trim().is_empty() {
        return Err("DPW-8 sidewall boundary condition must not be empty".to_owned());
    }
    for (name, value) in [
        ("Mach", spec.mach),
        ("chord Reynolds number", spec.chord_reynolds),
        ("static temperature", spec.static_temperature_kelvin),
        ("common farfield chords", spec.common_farfield_chords),
        ("gamma", spec.gamma),
        ("Prandtl number", spec.prandtl),
        ("turbulent Prandtl number", spec.turbulent_prandtl),
        (
            "farfield viscosity ratio",
            spec.farfield_kinematic_viscosity_ratio_chi,
        ),
    ] {
        if value.is_some_and(|value| !value.is_finite() || value <= 0.0) {
            return Err(format!(
                "DPW-8 {name} must be finite and positive when specified"
            ));
        }
    }
    if spec
        .residual_target_log10
        .is_some_and(|value| !value.is_finite() || value >= 0.0)
    {
        return Err("DPW-8 log10 residual target must be finite and negative".to_owned());
    }
    let mut angles = spec.angle_of_attack_degrees.clone();
    angles.sort_by(f64::total_cmp);
    if angles.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("DPW-8 angle-of-attack list contains a duplicate".to_owned());
    }
    match spec.angle_schedule {
        Dpw8AngleSchedule::ExactOfficialList => {
            if angles.is_empty() || angles != spec.angle_of_attack_degrees {
                // Exact protocol order is the specified order; no silent row reordering.
                return Err(
                    "DPW-8 official angle schedule must be non-empty and strictly increasing"
                        .to_owned(),
                );
            }
        }
        Dpw8AngleSchedule::NotSpecifiedByThisSource => {
            if !spec.angle_of_attack_degrees.is_empty() {
                return Err("DPW-8 AoA values must remain empty when the admitted source does not specify them".to_owned());
            }
        }
    }
    match spec.lane {
        Dpw8Oat15aLane::V1aSteadyGridConvergencePolar => {
            if spec.angle_of_attack_degrees != v1a_angles()
                || spec.mach != Some(0.73)
                || spec.chord_reynolds != Some(3.0e6)
                || spec.static_temperature_kelvin != Some(271.0)
                || spec.grid_policy != Dpw8GridFamilyPolicy::SixLevelsFourRequiredSixDesirable
                || spec.sidewall_boundary_condition != "periodic"
            {
                return Err(
                    "DPW-8 V1a settings differ from the admitted official protocol".to_owned(),
                );
            }
            if spec.common_farfield_chords.is_some()
                || spec.turbulence_model.is_some()
                || spec.sutherland_law.is_some()
                || spec.wall_thermal_condition.is_some()
                || spec.residual_target_log10.is_some()
                || spec.gamma.is_some()
                || spec.prandtl.is_some()
                || spec.turbulent_prandtl.is_some()
                || spec.farfield_kinematic_viscosity_ratio_chi.is_some()
            {
                return Err("V1a must not inherit V1c-only scatter-reprise settings".to_owned());
            }
        }
        Dpw8Oat15aLane::V1bUnsteadyExtendedPolarBuffet => {
            if spec.angle_of_attack_degrees != v1b_angles()
                || spec.mach != Some(0.73)
                || spec.chord_reynolds != Some(3.0e6)
                || spec.static_temperature_kelvin != Some(271.0)
                || spec.grid_policy != Dpw8GridFamilyPolicy::SixLevelsFourRequiredSixDesirable
                || spec.sidewall_boundary_condition != "periodic"
            {
                return Err(
                    "DPW-8 V1b settings differ from the admitted official protocol".to_owned(),
                );
            }
            if spec.common_farfield_chords.is_some()
                || spec.turbulence_model.is_some()
                || spec.sutherland_law.is_some()
                || spec.wall_thermal_condition.is_some()
                || spec.residual_target_log10.is_some()
                || spec.gamma.is_some()
                || spec.prandtl.is_some()
                || spec.turbulent_prandtl.is_some()
                || spec.farfield_kinematic_viscosity_ratio_chi.is_some()
            {
                return Err("V1b must not inherit V1c-only scatter-reprise settings".to_owned());
            }
        }
        Dpw8Oat15aLane::V1cScatterReductionReprise => {
            if spec.mach.is_some()
                || spec.chord_reynolds.is_some()
                || spec.static_temperature_kelvin.is_some()
            {
                return Err("V1c base flow values are not separately stated in the admitted scatter-reprise subsection; do not invent them here".to_owned());
            }
            if spec.angle_schedule != Dpw8AngleSchedule::NotSpecifiedByThisSource
                || !spec.angle_of_attack_degrees.is_empty()
                || spec.grid_policy != Dpw8GridFamilyPolicy::UpdatedFamilyDefinitionMustBeAdmitted
                || spec.common_farfield_chords != Some(1000.0)
                || spec.turbulence_model.as_deref()
                    != Some("Spalart-Allmaras negative variant, all terms")
                || spec.sutherland_law != Some(true)
                || spec.wall_thermal_condition != Some(Dpw8WallThermalCondition::Adiabatic)
                || spec.residual_target_log10 != Some(-10.0)
                || spec.gamma != Some(1.4)
                || spec.prandtl != Some(0.72)
                || spec.turbulent_prandtl != Some(0.9)
                || spec.farfield_kinematic_viscosity_ratio_chi != Some(3.0)
            {
                return Err(
                    "DPW-8 V1c settings differ from the admitted scatter-reduction protocol"
                        .to_owned(),
                );
            }
        }
    }
    Ok(())
}

fn canonical_bytes(manifest: &Dpw8Oat15aProtocolManifest) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        manifest.schema.as_str(),
        manifest.protocol_revision.as_str(),
        manifest.geometry_designation.as_str(),
        manifest.geometry_ip_holder.as_str(),
        manifest.geometry_share_condition.as_str(),
        manifest.evidence_level.as_str(),
        manifest.protocol_lane.as_str(),
        manifest.spec.lane.as_str(),
        manifest.spec.flow_mode.as_str(),
        manifest.spec.angle_schedule.as_str(),
        manifest.spec.grid_policy.as_str(),
        manifest.spec.sidewall_boundary_condition.as_str(),
    ] {
        append_string(&mut bytes, value);
    }
    bytes.push(u8::from(
        manifest.geometry_acknowledgement_required_each_share,
    ));
    append_u64(
        &mut bytes,
        manifest.spec.angle_of_attack_degrees.len() as u64,
    );
    for value in &manifest.spec.angle_of_attack_degrees {
        bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }
    for value in [
        manifest.spec.mach,
        manifest.spec.chord_reynolds,
        manifest.spec.static_temperature_kelvin,
        manifest.spec.common_farfield_chords,
        manifest.spec.residual_target_log10,
        manifest.spec.gamma,
        manifest.spec.prandtl,
        manifest.spec.turbulent_prandtl,
        manifest.spec.farfield_kinematic_viscosity_ratio_chi,
    ] {
        append_f64_option(&mut bytes, value);
    }
    for value in [
        manifest.spec.turbulence_model.as_deref().unwrap_or(""),
        manifest
            .spec
            .sutherland_law
            .map_or("", |v| if v { "true" } else { "false" }),
        manifest
            .spec
            .wall_thermal_condition
            .map_or("", Dpw8WallThermalCondition::as_str),
    ] {
        append_string(&mut bytes, value);
    }

    let mut sources = manifest.sources.iter().collect::<Vec<_>>();
    sources.sort_by(|left, right| left.source_id.cmp(&right.source_id));
    append_u64(&mut bytes, sources.len() as u64);
    for source in sources {
        for value in [
            source.source_id.as_str(),
            source.source_url.as_str(),
            source.source_kind.as_str(),
            source.state.as_str(),
            source.source_revision.as_str(),
            source.license_applicability.as_str(),
            source.acknowledgement.as_str(),
            source.sha256.as_deref().unwrap_or(""),
            source
                .permission_evidence_reference
                .as_deref()
                .unwrap_or(""),
            source.permission_evidence_sha256.as_deref().unwrap_or(""),
        ] {
            append_string(&mut bytes, value);
        }
        match source.byte_length {
            Some(length) => {
                bytes.push(1);
                append_u64(&mut bytes, length);
            }
            None => bytes.push(0),
        }
    }
    bytes
}

pub fn seal_dpw8_oat15a_manifest(
    mut manifest: Dpw8Oat15aProtocolManifest,
) -> Result<Dpw8Oat15aProtocolManifest, String> {
    if manifest.schema != DPW8_PROTOCOL_MANIFEST_SCHEMA
        || manifest.evidence_level != DPW8_PROTOCOL_EVIDENCE_LEVEL
    {
        return Err("unsupported DPW-8 protocol manifest schema/evidence ceiling".to_owned());
    }
    if manifest.protocol_revision.trim().is_empty()
        || manifest.geometry_designation != "ONERA OAT15A"
        || manifest.geometry_ip_holder != "ONERA"
        || !manifest.geometry_acknowledgement_required_each_share
        || !manifest
            .geometry_share_condition
            .contains("acknowledge ONERA each time")
    {
        return Err(
            "DPW-8 ONERA geometry designation/attribution/IP contract is incomplete".to_owned(),
        );
    }
    if manifest.protocol_lane != manifest.spec.lane {
        return Err("DPW-8 protocol lane envelope/spec identity mismatch".to_owned());
    }
    validate_spec(&manifest.spec)?;
    let mut ids = std::collections::BTreeSet::new();
    for source in &manifest.sources {
        validate_source(source)?;
        if !ids.insert(source.source_id.as_str()) {
            return Err(format!("duplicate DPW-8 source ID {}", source.source_id));
        }
    }
    for required in [
        "dpw8-test-case-definition",
        "onera-oat15a-geometry-policy",
        "dpw8-onera-oat15a-grid-index",
    ] {
        if !ids.contains(required) {
            return Err(format!(
                "DPW-8 manifest is missing required source {required}"
            ));
        }
    }
    manifest.manifest_sha256 = sha256_hex(&canonical_bytes(&manifest))?;
    Ok(manifest)
}

pub fn verify_dpw8_oat15a_manifest(manifest: &Dpw8Oat15aProtocolManifest) -> Result<(), String> {
    let sealed = seal_dpw8_oat15a_manifest(manifest.clone())?;
    if sealed != *manifest {
        return Err("DPW-8 protocol manifest canonical digest mismatch".to_owned());
    }
    Ok(())
}

pub fn dpw8_oat15a_protocol_sources_byte_pinned(manifest: &Dpw8Oat15aProtocolManifest) -> bool {
    manifest
        .sources
        .iter()
        .all(|source| source.state.is_byte_pinned())
}

pub fn dpw8_oat15a_geometry_permission_document_referenced(
    manifest: &Dpw8Oat15aProtocolManifest,
) -> bool {
    manifest.geometry_permission_document_referenced()
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn csv_row(fields: Vec<String>) -> String {
    fields
        .iter()
        .map(|field| csv_escape(field))
        .collect::<Vec<_>>()
        .join(",")
}

fn optional_f64(value: Option<f64>) -> String {
    value.map(|number| number.to_string()).unwrap_or_default()
}

/// Normalized protocol/source export for an independent non-Rust oracle.
pub fn dpw8_oat15a_manifest_to_csv(manifest: &Dpw8Oat15aProtocolManifest) -> String {
    let header = "record_type,schema,protocol_revision,geometry_designation,geometry_ip_holder,geometry_acknowledgement_required_each_share,geometry_share_condition,evidence_level,protocol_lane,lane,flow_mode,angle_schedule,angle_of_attack_degrees,mach,chord_reynolds,static_temperature_kelvin,grid_policy,sidewall_boundary_condition,common_farfield_chords,turbulence_model,sutherland_law,wall_thermal_condition,residual_target_log10,gamma,prandtl,turbulent_prandtl,farfield_kinematic_viscosity_ratio_chi,manifest_sha256,source_id,source_url,source_kind,source_state,source_byte_length,source_sha256,source_revision,license_applicability,acknowledgement,permission_evidence_reference,permission_evidence_sha256";
    let mut output = format!("{header}\n");
    let mut row = vec![String::new(); 39];
    row[0] = "manifest".to_owned();
    row[1] = manifest.schema.clone();
    row[2] = manifest.protocol_revision.clone();
    row[3] = manifest.geometry_designation.clone();
    row[4] = manifest.geometry_ip_holder.clone();
    row[5] = manifest
        .geometry_acknowledgement_required_each_share
        .to_string();
    row[6] = manifest.geometry_share_condition.clone();
    row[7] = manifest.evidence_level.clone();
    row[8] = manifest.protocol_lane.as_str().to_owned();
    row[9] = manifest.spec.lane.as_str().to_owned();
    row[10] = manifest.spec.flow_mode.clone();
    row[11] = manifest.spec.angle_schedule.as_str().to_owned();
    row[12] = manifest
        .spec
        .angle_of_attack_degrees
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(";");
    row[13] = optional_f64(manifest.spec.mach);
    row[14] = optional_f64(manifest.spec.chord_reynolds);
    row[15] = optional_f64(manifest.spec.static_temperature_kelvin);
    row[16] = manifest.spec.grid_policy.as_str().to_owned();
    row[17] = manifest.spec.sidewall_boundary_condition.clone();
    row[18] = optional_f64(manifest.spec.common_farfield_chords);
    row[19] = manifest.spec.turbulence_model.clone().unwrap_or_default();
    row[20] = manifest
        .spec
        .sutherland_law
        .map(|value| value.to_string())
        .unwrap_or_default();
    row[21] = manifest
        .spec
        .wall_thermal_condition
        .map(Dpw8WallThermalCondition::as_str)
        .unwrap_or_default()
        .to_owned();
    row[22] = optional_f64(manifest.spec.residual_target_log10);
    row[23] = optional_f64(manifest.spec.gamma);
    row[24] = optional_f64(manifest.spec.prandtl);
    row[25] = optional_f64(manifest.spec.turbulent_prandtl);
    row[26] = optional_f64(manifest.spec.farfield_kinematic_viscosity_ratio_chi);
    row[27] = manifest.manifest_sha256.clone();
    output.push_str(&csv_row(row));
    output.push('\n');

    let mut sources = manifest.sources.iter().collect::<Vec<_>>();
    sources.sort_by(|left, right| left.source_id.cmp(&right.source_id));
    for source in sources {
        let mut row = vec![String::new(); 39];
        row[0] = "source".to_owned();
        row[28] = source.source_id.clone();
        row[29] = source.source_url.clone();
        row[30] = source.source_kind.clone();
        row[31] = source.state.as_str().to_owned();
        row[32] = source
            .byte_length
            .map(|value| value.to_string())
            .unwrap_or_default();
        row[33] = source.sha256.clone().unwrap_or_default();
        row[34] = source.source_revision.clone();
        row[35] = source.license_applicability.clone();
        row[36] = source.acknowledgement.clone();
        row[37] = source
            .permission_evidence_reference
            .clone()
            .unwrap_or_default();
        row[38] = source
            .permission_evidence_sha256
            .clone()
            .unwrap_or_default();
        output.push_str(&csv_row(row));
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(lane: Dpw8Oat15aLane) -> Dpw8Oat15aProtocolManifest {
        Dpw8Oat15aProtocolManifest::new("AIAA-DPW8-web-protocol-snapshot-2026-11-03", lane)
    }

    #[test]
    fn v1a_and_v1b_remain_distinct_protocols_and_angle_schedules() {
        let v1a =
            seal_dpw8_oat15a_manifest(manifest(Dpw8Oat15aLane::V1aSteadyGridConvergencePolar))
                .unwrap();
        let v1b =
            seal_dpw8_oat15a_manifest(manifest(Dpw8Oat15aLane::V1bUnsteadyExtendedPolarBuffet))
                .unwrap();
        assert_eq!(
            v1a.spec.angle_of_attack_degrees,
            vec![1.36, 1.50, 2.50, 3.00, 3.10]
        );
        assert_eq!(
            v1b.spec.angle_of_attack_degrees,
            vec![3.25, 3.40, 3.50, 3.60, 3.90]
        );
        assert_eq!(v1a.spec.flow_mode, "steady");
        assert_eq!(v1b.spec.flow_mode, "unsteady");
        assert_ne!(v1a.manifest_sha256, v1b.manifest_sha256);
    }

    #[test]
    fn v1c_preserves_scatter_settings_and_does_not_invent_aoa_points() {
        let v1c = seal_dpw8_oat15a_manifest(manifest(Dpw8Oat15aLane::V1cScatterReductionReprise))
            .unwrap();
        assert_eq!(
            v1c.spec.angle_schedule,
            Dpw8AngleSchedule::NotSpecifiedByThisSource
        );
        assert!(v1c.spec.angle_of_attack_degrees.is_empty());
        assert_eq!(v1c.spec.common_farfield_chords, Some(1000.0));
        assert_eq!(v1c.spec.residual_target_log10, Some(-10.0));
        assert_eq!(v1c.spec.farfield_kinematic_viscosity_ratio_chi, Some(3.0));
    }

    #[test]
    fn cross_lane_protocol_substitution_fails_closed() {
        let mut v1a = manifest(Dpw8Oat15aLane::V1aSteadyGridConvergencePolar);
        v1a.spec = dpw8_oat15a_v1b_spec();
        assert!(
            seal_dpw8_oat15a_manifest(v1a)
                .expect_err("V1b settings must not enter V1a")
                .to_string()
                .contains("envelope/spec identity mismatch")
        );
        let mut envelope = manifest(Dpw8Oat15aLane::V1bUnsteadyExtendedPolarBuffet);
        envelope.protocol_lane = Dpw8Oat15aLane::V1aSteadyGridConvergencePolar;
        assert!(
            seal_dpw8_oat15a_manifest(envelope)
                .expect_err("outer protocol lane must bind the nested specification")
                .to_string()
                .contains("envelope/spec identity mismatch")
        );
        let mut v1b = manifest(Dpw8Oat15aLane::V1bUnsteadyExtendedPolarBuffet);
        v1b.spec = dpw8_oat15a_v1a_spec();
        assert!(seal_dpw8_oat15a_manifest(v1b).is_err());
    }

    #[test]
    fn v1c_cannot_inherit_an_inferred_v1a_aoa_schedule() {
        let mut v1c = manifest(Dpw8Oat15aLane::V1cScatterReductionReprise);
        v1c.spec.angle_schedule = Dpw8AngleSchedule::ExactOfficialList;
        v1c.spec.angle_of_attack_degrees = v1a_angles();
        assert!(seal_dpw8_oat15a_manifest(v1c).is_err());
    }

    #[test]
    fn source_admission_is_explicit_and_does_not_assume_geometry_redistribution() {
        let v1a =
            seal_dpw8_oat15a_manifest(manifest(Dpw8Oat15aLane::V1aSteadyGridConvergencePolar))
                .unwrap();
        assert!(!dpw8_oat15a_protocol_sources_byte_pinned(&v1a));
        assert!(!dpw8_oat15a_geometry_permission_document_referenced(&v1a));
    }

    #[test]
    fn tampered_source_or_lane_changes_canonical_manifest_identity() {
        let original =
            seal_dpw8_oat15a_manifest(manifest(Dpw8Oat15aLane::V1cScatterReductionReprise))
                .unwrap();
        let mut tampered = original.clone();
        tampered
            .geometry_share_condition
            .push_str("; omit attribution");
        assert!(verify_dpw8_oat15a_manifest(&tampered).is_err());
    }

    #[test]
    fn geometry_permission_document_reference_is_not_inferred_from_source_url() {
        let mut manifest = manifest(Dpw8Oat15aLane::V1aSteadyGridConvergencePolar);
        let geometry = manifest
            .sources
            .iter_mut()
            .find(|source| source.source_kind == "geometry_ip_and_acknowledgement_policy")
            .unwrap();
        geometry.state =
            Dpw8SourceAdmissionState::BytePinnedRedistributionPermissionDocumentReferenced;
        geometry.byte_length = Some(4096);
        geometry.sha256 = Some("a".repeat(64));
        geometry.source_revision = "permission-policy-review-v1".to_owned();
        geometry.permission_evidence_reference =
            Some("internal-review://onera-oat15a-redistribution-decision".to_owned());
        geometry.permission_evidence_sha256 = Some("b".repeat(64));
        let manifest =
            seal_dpw8_oat15a_manifest(manifest).expect("well-formed permission-document reference");
        assert!(dpw8_oat15a_geometry_permission_document_referenced(
            &manifest
        ));

        let mut missing_digest = manifest.clone();
        let geometry = missing_digest
            .sources
            .iter_mut()
            .find(|source| source.source_kind == "geometry_ip_and_acknowledgement_policy")
            .unwrap();
        geometry.permission_evidence_sha256 = None;
        assert!(seal_dpw8_oat15a_manifest(missing_digest).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn independent_python_oracle_replays_each_dp_w8_protocol_lane() {
        let lanes = [
            Dpw8Oat15aLane::V1aSteadyGridConvergencePolar,
            Dpw8Oat15aLane::V1bUnsteadyExtendedPolarBuffet,
            Dpw8Oat15aLane::V1cScatterReductionReprise,
        ];
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "dpw8-protocol-oracle-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("create oracle fixture");
        let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("scripts/verify-dpw8-protocol-independent.py");
        for lane in lanes {
            let manifest = seal_dpw8_oat15a_manifest(manifest(lane))
                .expect("seal lane-specific DPW-8 protocol");
            let csv_path = root.join(format!("{}.csv", lane.as_str()));
            std::fs::write(&csv_path, dpw8_oat15a_manifest_to_csv(&manifest))
                .expect("write DPW-8 protocol CSV");
            let output = std::process::Command::new("python3")
                .arg(&script)
                .arg(&csv_path)
                .output()
                .expect("execute independent DPW-8 protocol oracle");
            assert!(
                output.status.success(),
                "independent DPW-8 oracle failed for {}: {}{}",
                lane.as_str(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::fs::remove_dir_all(root).expect("remove DPW-8 oracle fixture");
    }

    #[test]
    fn byte_pinned_source_requires_revision_digest_and_length() {
        let mut v1a = manifest(Dpw8Oat15aLane::V1aSteadyGridConvergencePolar);
        let source = v1a
            .sources
            .iter_mut()
            .find(|s| s.source_id == "dpw8-test-case-definition")
            .unwrap();
        source.state = Dpw8SourceAdmissionState::BytePinnedReferenceOnly;
        assert!(seal_dpw8_oat15a_manifest(v1a).is_err());
    }
}
