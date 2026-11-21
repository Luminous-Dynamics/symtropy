use power_restoration_benchmark_case::{BenchmarkCase, canonical_case_sha256};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const FLOAT_TOLERANCE: f64 = 1e-8;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Conductor {
    A,
    B,
    C,
    Neutral,
    ProtectiveEarth,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GroundingType {
    Solid,
    ResistanceGrounded,
    ReactanceGrounded,
    Ungrounded,
    MultiGroundedNeutral,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BranchEquipmentKind {
    OverheadLine,
    Cable,
    Transformer,
    Regulator,
    SeriesReactor,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveMatrixUnit {
    OhmPerKm,
    OhmOnHighSideBase,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WindingConnection {
    GroundedWye,
    UngroundedWye,
    Delta,
    ZigZag,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ZeroSequenceTransfer {
    Transferred,
    Blocked,
    DependsOnGrounding,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FaultType {
    SingleLineToGround,
    LineToLine,
    DoubleLineToGround,
    ThreePhase,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProtectionDeviceKind {
    CircuitBreaker,
    Fuse,
    Recloser,
    Relay,
    Sectionalizer,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessScope {
    ThreePhaseLoadFlow,
    ShortCircuit,
    ProtectionCoordination,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessState {
    ReadyForIndependentDomainSolver,
    InconclusiveMissingOrInvalidNativeInputs,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComplexValue {
    pub real: f64,
    pub imaginary: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactRef {
    pub artifact_uri: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceRef {
    pub artifact_uri: String,
    pub sha256: String,
    pub producer_id: String,
    pub producer_profile_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageProducer {
    pub producer_id: String,
    pub source_commit: String,
    pub configuration_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ElectricalBase {
    pub frequency_hz: f64,
    pub base_mva: f64,
    pub nominal_line_line_kv: f64,
    pub voltage_min_pu: f64,
    pub voltage_max_pu: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NodePhaseModel {
    pub node_id: String,
    pub conductors: Vec<Conductor>,
    pub grounding_type: GroundingType,
    pub neutral_continuous: bool,
    pub grounding_impedance_ohm: Option<ComplexValue>,
    pub evidence: EvidenceRef,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrimitiveImpedanceMatrix {
    pub unit: PrimitiveMatrixUnit,
    pub conductor_order: Vec<Conductor>,
    pub resistance_ohm_per_km_or_base: Vec<Vec<f64>>,
    pub reactance_ohm_per_km_or_base: Vec<Vec<f64>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequenceImpedance {
    pub positive: ComplexValue,
    pub negative: ComplexValue,
    pub zero: ComplexValue,
    pub unit: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConductorCurrentRating {
    pub conductor: Conductor,
    pub continuous_ampacity_a: f64,
    pub emergency_ampacity_a: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BranchPhaseModel {
    pub branch_id: String,
    pub equipment_kind: BranchEquipmentKind,
    pub conductors: Vec<Conductor>,
    pub primitive_impedance: Option<PrimitiveImpedanceMatrix>,
    pub sequence_impedance: Option<SequenceImpedance>,
    pub current_ratings: Vec<ConductorCurrentRating>,
    pub evidence: EvidenceRef,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransformerModel {
    pub branch_id: String,
    pub high_voltage_connection: WindingConnection,
    pub low_voltage_connection: WindingConnection,
    pub vector_group: String,
    pub clock_number: u8,
    pub high_voltage_nominal_kv: f64,
    pub low_voltage_nominal_kv: f64,
    pub rated_power_mva: f64,
    pub leakage_sequence_impedance: Option<SequenceImpedance>,
    pub zero_sequence_transfer: ZeroSequenceTransfer,
    pub high_voltage_neutral_grounded: bool,
    pub low_voltage_neutral_grounded: bool,
    pub evidence: EvidenceRef,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhasePower {
    pub phase: Conductor,
    pub active_power_kw: f64,
    pub reactive_power_kvar: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoadPhaseAllocation {
    pub load_id: String,
    pub phases: Vec<PhasePower>,
    pub evidence: EvidenceRef,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhaseVoltage {
    pub phase: Conductor,
    pub line_neutral_kv: f64,
    pub angle_degrees: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GridSourceModel {
    pub source_id: String,
    pub phase_voltages: Vec<PhaseVoltage>,
    pub sequence_impedance: Option<SequenceImpedance>,
    pub short_circuit_power_mva: Option<f64>,
    pub x_over_r_ratio: Option<f64>,
    pub evidence: EvidenceRef,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FaultScenario {
    pub fault_id: String,
    pub fault_type: FaultType,
    pub phases_involved: Vec<Conductor>,
    pub fault_impedance_ohm: ComplexValue,
    pub contributing_source_ids: Vec<String>,
    pub evidence: EvidenceRef,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimeCurrentPoint {
    pub current_multiple_of_pickup: f64,
    pub trip_time_seconds: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProtectionDeviceModel {
    pub device_id: String,
    pub device_kind: ProtectionDeviceKind,
    pub protected_branch_ids: Vec<String>,
    pub current_transformer_primary_a: Option<f64>,
    pub current_transformer_secondary_a: Option<f64>,
    pub voltage_transformer_ratio: Option<f64>,
    pub phase_pickup_primary_a: Option<f64>,
    pub ground_pickup_primary_a: Option<f64>,
    pub interruption_capacity_ka: Option<f64>,
    pub phase_time_current_curve: Vec<TimeCurrentPoint>,
    pub ground_time_current_curve: Vec<TimeCurrentPoint>,
    pub coordination_group_id: String,
    pub evidence: EvidenceRef,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProtectionExemption {
    pub branch_id: String,
    pub reason: String,
    pub approved_by: String,
    pub approval_evidence: EvidenceRef,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThreePhaseInputPackage {
    pub schema_version: String,
    pub package_id: String,
    pub case_sha256: String,
    pub base: ElectricalBase,
    pub producer: PackageProducer,
    pub node_models: Vec<NodePhaseModel>,
    pub branch_models: Vec<BranchPhaseModel>,
    pub transformer_models: Vec<TransformerModel>,
    pub load_phase_allocations: Vec<LoadPhaseAllocation>,
    pub grid_source_models: Vec<GridSourceModel>,
    pub fault_scenarios: Vec<FaultScenario>,
    pub protection_devices: Vec<ProtectionDeviceModel>,
    pub protection_exemptions: Vec<ProtectionExemption>,
    pub source_artifacts: Vec<ArtifactRef>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessBlocker {
    pub scope: ReadinessScope,
    pub object_kind: String,
    pub object_id: String,
    pub field_path: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScopeReadiness {
    pub state: ReadinessState,
    pub blocker_count: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThreePhaseReadinessReport {
    pub package_id: String,
    pub case_sha256: String,
    pub package_sha256: String,
    pub source_artifact_count: usize,
    pub three_phase_load_flow: ScopeReadiness,
    pub short_circuit: ScopeReadiness,
    pub protection_coordination: ScopeReadiness,
    pub blocker_count: usize,
    pub blockers: Vec<ReadinessBlocker>,
    pub result_status: String,
    pub claim_boundary: String,
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
}

fn lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, String> {
    let value = serde_json::to_value(value)
        .map_err(|error| format!("JSON serialization failed: {error}"))?;
    let bytes = serde_json::to_vec(&value)
        .map_err(|error| format!("canonical JSON serialization failed: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn blocker(
    blockers: &mut Vec<ReadinessBlocker>,
    scope: ReadinessScope,
    kind: &str,
    id: &str,
    field: &str,
    reason: &str,
) {
    blockers.push(ReadinessBlocker {
        scope,
        object_kind: kind.to_string(),
        object_id: id.to_string(),
        field_path: field.to_string(),
        reason: reason.to_string(),
    });
}

fn unique_conductors(values: &[Conductor]) -> bool {
    let set: BTreeSet<_> = values.iter().copied().collect();
    set.len() == values.len()
}

fn has_phase(values: &[Conductor], phase: Conductor) -> bool {
    values.contains(&phase)
}

fn valid_complex(value: &ComplexValue, allow_zero: bool) -> bool {
    value.real.is_finite()
        && value.imaginary.is_finite()
        && value.real >= 0.0
        && (allow_zero || value.real.abs() + value.imaginary.abs() > FLOAT_TOLERANCE)
}

fn validate_sequence(
    sequence: Option<&SequenceImpedance>,
    blockers: &mut Vec<ReadinessBlocker>,
    kind: &str,
    id: &str,
    field: &str,
    scope: ReadinessScope,
) {
    let Some(sequence) = sequence else {
        blocker(
            blockers,
            scope,
            kind,
            id,
            field,
            "native positive-, negative- and zero-sequence impedance data is missing",
        );
        return;
    };
    if sequence.unit != "ohm"
        || !valid_complex(&sequence.positive, false)
        || !valid_complex(&sequence.negative, false)
        || !valid_complex(&sequence.zero, false)
    {
        blocker(
            blockers,
            scope,
            kind,
            id,
            field,
            "positive-, negative- and zero-sequence impedances must be finite, nonzero and explicitly in ohms",
        );
    }
}

fn validate_evidence(
    evidence: &EvidenceRef,
    sources: &BTreeMap<&str, &str>,
    referenced: &mut BTreeSet<String>,
    blockers: &mut Vec<ReadinessBlocker>,
    scope: ReadinessScope,
    kind: &str,
    id: &str,
    field: &str,
) {
    if !valid_id(&evidence.artifact_uri)
        || !lower_hex(&evidence.sha256, 64)
        || !valid_id(&evidence.producer_id)
        || !lower_hex(&evidence.producer_profile_sha256, 64)
    {
        blocker(
            blockers,
            scope,
            kind,
            id,
            field,
            "evidence URI, SHA-256, producer identity or producer profile digest is invalid",
        );
        return;
    }
    referenced.insert(evidence.artifact_uri.clone());
    if sources.get(evidence.artifact_uri.as_str()).copied() != Some(evidence.sha256.as_str()) {
        blocker(
            blockers,
            scope,
            kind,
            id,
            field,
            "evidence artifact is absent from source_artifacts or its digest differs",
        );
    }
}

fn validate_phase_list(
    phases: &[Conductor],
    allow_neutral: bool,
    blockers: &mut Vec<ReadinessBlocker>,
    kind: &str,
    id: &str,
    field: &str,
    scope: ReadinessScope,
) {
    let has_abc = phases.iter().any(|p| matches!(p, Conductor::A | Conductor::B | Conductor::C));
    if phases.is_empty()
        || !has_abc
        || !unique_conductors(phases)
        || (!allow_neutral && phases.iter().any(|p| matches!(p, Conductor::Neutral | Conductor::ProtectiveEarth)))
    {
        blocker(
            blockers,
            scope,
            kind,
            id,
            field,
            "conductor set must contain unique phase conductors and only permitted neutral/earth conductors",
        );
    }
}

fn validate_node_model(
    node: &NodePhaseModel,
    blockers: &mut Vec<ReadinessBlocker>,
    sources: &BTreeMap<&str, &str>,
    referenced: &mut BTreeSet<String>,
) {
    validate_phase_list(
        &node.conductors,
        true,
        blockers,
        "node",
        &node.node_id,
        "conductors",
        ReadinessScope::ThreePhaseLoadFlow,
    );
    match node.grounding_type {
        GroundingType::Ungrounded => {
            if node.neutral_continuous || node.grounding_impedance_ohm.is_some() {
                blocker(
                    blockers,
                    ReadinessScope::ThreePhaseLoadFlow,
                    "node",
                    &node.node_id,
                    "grounding",
                    "ungrounded node must not claim a continuous grounded neutral or grounding impedance",
                );
            }
        }
        _ => {
            if !node.conductors.contains(&Conductor::Neutral) {
                blocker(
                    blockers,
                    ReadinessScope::ThreePhaseLoadFlow,
                    "node",
                    &node.node_id,
                    "conductors",
                    "grounded node requires an explicit neutral conductor in the native model",
                );
            }
            if node.grounding_type == GroundingType::MultiGroundedNeutral && !node.neutral_continuous {
                blocker(
                    blockers,
                    ReadinessScope::ThreePhaseLoadFlow,
                    "node",
                    &node.node_id,
                    "neutralContinuous",
                    "multi-grounded neutral must declare neutral continuity",
                );
            }
            if node.grounding_impedance_ohm.as_ref().is_none_or(|z| !z.real.is_finite() || !z.imaginary.is_finite() || z.real < 0.0) {
                blocker(
                    blockers,
                    ReadinessScope::ThreePhaseLoadFlow,
                    "node",
                    &node.node_id,
                    "groundingImpedanceOhm",
                    "grounded node requires finite non-negative grounding impedance",
                );
            }
        }
    }
    validate_evidence(
        &node.evidence,
        sources,
        referenced,
        blockers,
        ReadinessScope::ThreePhaseLoadFlow,
        "node",
        &node.node_id,
        "evidence",
    );
}

fn is_positive_definite_symmetric(matrix: &[Vec<f64>]) -> bool {
    let n = matrix.len();
    if n == 0 || matrix.iter().any(|row| row.len() != n) {
        return false;
    }
    let mut lower = vec![vec![0.0_f64; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut value = matrix[i][j];
            for k in 0..j {
                value -= lower[i][k] * lower[j][k];
            }
            if i == j {
                if !value.is_finite() || value <= FLOAT_TOLERANCE {
                    return false;
                }
                lower[i][j] = value.sqrt();
            } else {
                lower[i][j] = value / lower[j][j];
            }
        }
    }
    true
}

fn validate_matrix(
    branch: &BranchPhaseModel,
    matrix: &PrimitiveImpedanceMatrix,
    blockers: &mut Vec<ReadinessBlocker>,
) {
    let n = branch.conductors.len();
    let valid_dimensions = n > 0
        && matrix.conductor_order == branch.conductors
        && matrix.resistance_ohm_per_km_or_base.len() == n
        && matrix.reactance_ohm_per_km_or_base.len() == n
        && matrix.resistance_ohm_per_km_or_base.iter().all(|row| row.len() == n)
        && matrix.reactance_ohm_per_km_or_base.iter().all(|row| row.len() == n);
    if !valid_dimensions {
        blocker(
            blockers,
            ReadinessScope::ThreePhaseLoadFlow,
            "branch",
            &branch.branch_id,
            "primitiveImpedance",
            "primitive R/X matrices must be square and dimensionally match the declared conductor order",
        );
        return;
    }

    if branch.conductors.windows(2).any(|pair| pair[0] >= pair[1]) {
        blocker(
            blockers,
            ReadinessScope::ThreePhaseLoadFlow,
            "branch",
            &branch.branch_id,
            "conductors",
            "branch conductors must be unique and in canonical phase/neutral/earth order",
        );
    }

    for row in 0..n {
        for col in 0..n {
            let resistance = matrix.resistance_ohm_per_km_or_base[row][col];
            let reactance = matrix.reactance_ohm_per_km_or_base[row][col];
            if !resistance.is_finite() || !reactance.is_finite() {
                blocker(
                    blockers,
                    ReadinessScope::ThreePhaseLoadFlow,
                    "branch",
                    &branch.branch_id,
                    "primitiveImpedance",
                    "primitive impedance entries must be finite",
                );
                return;
            }
            if (resistance - matrix.resistance_ohm_per_km_or_base[col][row]).abs() > FLOAT_TOLERANCE
                || (reactance - matrix.reactance_ohm_per_km_or_base[col][row]).abs() > FLOAT_TOLERANCE
            {
                blocker(
                    blockers,
                    ReadinessScope::ThreePhaseLoadFlow,
                    "branch",
                    &branch.branch_id,
                    "primitiveImpedance",
                    "primitive R/X matrices must be symmetric for this v0.1 model contract",
                );
                return;
            }
            if row == col && resistance <= 0.0 {
                blocker(
                    blockers,
                    ReadinessScope::ThreePhaseLoadFlow,
                    "branch",
                    &branch.branch_id,
                    "primitiveImpedance",
                    "primitive resistance diagonal must be strictly positive",
                );
                return;
            }
        }
    }

    if !is_positive_definite_symmetric(&matrix.resistance_ohm_per_km_or_base) {
        blocker(
            blockers,
            ReadinessScope::ThreePhaseLoadFlow,
            "branch",
            &branch.branch_id,
            "primitiveImpedance.resistance",
            "primitive resistance matrix must be symmetric positive definite for the v0.1 passive series-element contract",
        );
    }

    let expected_unit = if branch.equipment_kind == BranchEquipmentKind::Transformer {
        PrimitiveMatrixUnit::OhmOnHighSideBase
    } else {
        PrimitiveMatrixUnit::OhmPerKm
    };
    if matrix.unit != expected_unit {
        blocker(
            blockers,
            ReadinessScope::ThreePhaseLoadFlow,
            "branch",
            &branch.branch_id,
            "primitiveImpedance.unit",
            "primitive matrix unit must match equipment kind (ohm/km or ohm on high-side base)",
        );
    }
}

fn validate_current_ratings(
    branch: &BranchPhaseModel,
    blockers: &mut Vec<ReadinessBlocker>,
) {
    let mut actual = BTreeMap::new();
    for rating in &branch.current_ratings {
        if actual.insert(rating.conductor, rating).is_some() {
            blocker(
                blockers,
                ReadinessScope::ThreePhaseLoadFlow,
                "branch",
                &branch.branch_id,
                "currentRatings",
                "duplicate conductor current rating",
            );
        }
        if !rating.continuous_ampacity_a.is_finite()
            || !rating.emergency_ampacity_a.is_finite()
            || rating.continuous_ampacity_a <= 0.0
            || rating.emergency_ampacity_a < rating.continuous_ampacity_a
        {
            blocker(
                blockers,
                ReadinessScope::ThreePhaseLoadFlow,
                "branch",
                &branch.branch_id,
                "currentRatings",
                "per-conductor ampacity must be finite, positive and emergency rating must be >= continuous rating",
            );
        }
    }
    for conductor in &branch.conductors {
        if !actual.contains_key(conductor) {
            blocker(
                blockers,
                ReadinessScope::ThreePhaseLoadFlow,
                "branch",
                &branch.branch_id,
                "currentRatings",
                "missing current rating for a declared conductor",
            );
        }
    }
    if actual.len() != branch.conductors.len() {
        blocker(
            blockers,
            ReadinessScope::ThreePhaseLoadFlow,
            "branch",
            &branch.branch_id,
            "currentRatings",
            "current rating set must exactly cover the declared conductor set",
        );
    }
}

fn validate_curve(
    curve: &[TimeCurrentPoint],
    device_id: &str,
    curve_name: &str,
    blockers: &mut Vec<ReadinessBlocker>,
) {
    let monotonic = curve.len() >= 2
        && curve.iter().all(|point| {
            point.current_multiple_of_pickup.is_finite()
                && point.current_multiple_of_pickup > 0.0
                && point.trip_time_seconds.is_finite()
                && point.trip_time_seconds > 0.0
        })
        && curve.windows(2).all(|pair| {
            pair[0].current_multiple_of_pickup < pair[1].current_multiple_of_pickup
                && pair[0].trip_time_seconds > pair[1].trip_time_seconds
        });
    if !monotonic {
        blocker(
            blockers,
            ReadinessScope::ProtectionCoordination,
            "protectionDevice",
            device_id,
            curve_name,
            "time-current curve requires >=2 finite points, strictly increasing current multiple and strictly decreasing trip time",
        );
    }
}

fn add_blocker_if(
    condition: bool,
    blockers: &mut Vec<ReadinessBlocker>,
    scope: ReadinessScope,
    kind: &str,
    id: &str,
    field: &str,
    reason: &str,
) {
    if condition {
        blocker(blockers, scope, kind, id, field, reason);
    }
}

pub fn audit_three_phase_inputs(
    case: &BenchmarkCase,
    package: &ThreePhaseInputPackage,
) -> Result<ThreePhaseReadinessReport, Vec<String>> {
    let case_sha = canonical_case_sha256(case)
        .map_err(|error| vec![format!("benchmark case canonicalization failed: {error}")])?;
    let package_sha = canonical_sha256(package)
        .map_err(|error| vec![format!("three-phase package canonicalization failed: {error}")])?;
    let mut blockers = Vec::new();

    add_blocker_if(
        package.schema_version != "0.1.0"
            || !valid_id(&package.package_id)
            || package.case_sha256 != case_sha,
        &mut blockers,
        ReadinessScope::ThreePhaseLoadFlow,
        "package",
        &package.package_id,
        "identity",
        "package schema, package ID or frozen case SHA-256 is invalid/mismatched",
    );
    add_blocker_if(
        !valid_id(&package.producer.producer_id)
            || !lower_hex(&package.producer.source_commit, 40)
            || !lower_hex(&package.producer.configuration_sha256, 64),
        &mut blockers,
        ReadinessScope::ThreePhaseLoadFlow,
        "package",
        &package.package_id,
        "producer",
        "package producer identity, source commit or configuration digest is invalid",
    );
    let base = &package.base;
    add_blocker_if(
        !base.frequency_hz.is_finite()
            || base.frequency_hz <= 0.0
            || !base.base_mva.is_finite()
            || base.base_mva <= 0.0
            || !base.nominal_line_line_kv.is_finite()
            || base.nominal_line_line_kv <= 0.0
            || !base.voltage_min_pu.is_finite()
            || !base.voltage_max_pu.is_finite()
            || base.voltage_min_pu <= 0.0
            || base.voltage_max_pu <= base.voltage_min_pu,
        &mut blockers,
        ReadinessScope::ThreePhaseLoadFlow,
        "package",
        &package.package_id,
        "base",
        "frequency, MVA/kV bases and voltage limits must be finite, positive and ordered",
    );

    let mut source_map = BTreeMap::<&str, &str>::new();
    for artifact in &package.source_artifacts {
        if !valid_id(&artifact.artifact_uri)
            || !lower_hex(&artifact.sha256, 64)
            || source_map.insert(artifact.artifact_uri.as_str(), artifact.sha256.as_str()).is_some()
        {
            blocker(
                &mut blockers,
                ReadinessScope::ThreePhaseLoadFlow,
                "sourceArtifact",
                &artifact.artifact_uri,
                "identity",
                "source artifact URI must be unique/non-empty with a lower-case SHA-256",
            );
        }
    }
    let mut referenced_sources = BTreeSet::<String>::new();

    let expected_node_ids: BTreeSet<&str> = case.network.nodes.iter().map(|node| node.node_id.as_str()).collect();
    let mut node_ids = BTreeSet::new();
    let mut node_map = BTreeMap::<&str, &NodePhaseModel>::new();
    for node in &package.node_models {
        if !node_ids.insert(node.node_id.as_str()) {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "node", &node.node_id, "nodeId", "duplicate node phase model");
        }
        node_map.insert(node.node_id.as_str(), node);
        if !expected_node_ids.contains(node.node_id.as_str()) {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "node", &node.node_id, "nodeId", "node phase model references node absent from frozen case");
        }
        validate_node_model(node, &mut blockers, &source_map, &mut referenced_sources);
    }
    for node_id in &expected_node_ids {
        if !node_ids.contains(node_id) {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "node", node_id, "nodeModels", "missing native phase and grounding model for case node");
        }
    }

    let expected_branch_ids: BTreeSet<&str> = case.network.branches.iter().map(|branch| branch.branch_id.as_str()).collect();
    let mut branch_ids = BTreeSet::new();
    let mut branch_kinds = BTreeMap::<&str, BranchEquipmentKind>::new();
    for branch in &package.branch_models {
        if !branch_ids.insert(branch.branch_id.as_str()) {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "branch", &branch.branch_id, "branchId", "duplicate branch phase model");
        }
        branch_kinds.insert(branch.branch_id.as_str(), branch.equipment_kind);
        if !expected_branch_ids.contains(branch.branch_id.as_str()) {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "branch", &branch.branch_id, "branchId", "branch model references branch absent from frozen case");
        }
        validate_phase_list(&branch.conductors, true, &mut blockers, "branch", &branch.branch_id, "conductors", ReadinessScope::ThreePhaseLoadFlow);
        match branch.primitive_impedance.as_ref() {
            Some(matrix) => validate_matrix(branch, matrix, &mut blockers),
            None => blocker(
                &mut blockers,
                ReadinessScope::ThreePhaseLoadFlow,
                "branch",
                &branch.branch_id,
                "primitiveImpedance",
                "native phase primitive impedance matrix is missing; balanced scalar R/X is not a substitute",
            ),
        }
        validate_current_ratings(branch, &mut blockers);
        validate_evidence(&branch.evidence, &source_map, &mut referenced_sources, &mut blockers, ReadinessScope::ThreePhaseLoadFlow, "branch", &branch.branch_id, "evidence");
        validate_sequence(branch.sequence_impedance.as_ref(), &mut blockers, "branch", &branch.branch_id, "sequenceImpedance", ReadinessScope::ShortCircuit);

        if let Some(case_branch) = case.network.branches.iter().find(|item| item.branch_id == branch.branch_id) {
            for endpoint in [&case_branch.from_node_id, &case_branch.to_node_id] {
                match node_map.get(endpoint.as_str()) {
                    Some(node) => {
                        for conductor in &branch.conductors {
                            if !node.conductors.contains(conductor) {
                                blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "branch", &branch.branch_id, "conductors", "branch conductor is absent from one endpoint node model");
                                break;
                            }
                        }
                    }
                    None => blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "branch", &branch.branch_id, "endpoints", "branch endpoint lacks a native node phase model"),
                }
            }
        }
    }
    for branch_id in &expected_branch_ids {
        if !branch_ids.contains(branch_id) {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "branch", branch_id, "branchModels", "missing native primitive impedance/phase model for case branch");
            blocker(&mut blockers, ReadinessScope::ShortCircuit, "branch", branch_id, "sequenceImpedance", "missing native sequence impedance for case branch");
        }
    }

    let mut transformer_ids = BTreeSet::new();
    for transformer in &package.transformer_models {
        if !transformer_ids.insert(transformer.branch_id.as_str()) {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "transformer", &transformer.branch_id, "branchId", "duplicate transformer model");
        }
        if branch_kinds.get(transformer.branch_id.as_str()) != Some(&BranchEquipmentKind::Transformer) {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "transformer", &transformer.branch_id, "branchId", "transformer model does not match a transformer-classified branch");
        }
        add_blocker_if(
            !valid_id(&transformer.vector_group)
                || transformer.vector_group.bytes().any(|byte| byte.is_ascii_whitespace())
                || transformer.clock_number > 11
                || !transformer.high_voltage_nominal_kv.is_finite()
                || !transformer.low_voltage_nominal_kv.is_finite()
                || !transformer.rated_power_mva.is_finite()
                || transformer.high_voltage_nominal_kv <= 0.0
                || transformer.low_voltage_nominal_kv <= 0.0
                || transformer.rated_power_mva <= 0.0,
            &mut blockers,
            ReadinessScope::ThreePhaseLoadFlow,
            "transformer",
            &transformer.branch_id,
            "vectorGroupAndRatings",
            "transformer vector group, clock number and winding ratings are incomplete or invalid",
        );
        if transformer.high_voltage_connection == WindingConnection::GroundedWye && !transformer.high_voltage_neutral_grounded {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "transformer", &transformer.branch_id, "highVoltageNeutralGrounded", "grounded-wye high side must declare neutral grounding");
        }
        if transformer.low_voltage_connection == WindingConnection::GroundedWye && !transformer.low_voltage_neutral_grounded {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "transformer", &transformer.branch_id, "lowVoltageNeutralGrounded", "grounded-wye low side must declare neutral grounding");
        }
        validate_sequence(transformer.leakage_sequence_impedance.as_ref(), &mut blockers, "transformer", &transformer.branch_id, "leakageSequenceImpedance", ReadinessScope::ShortCircuit);
        validate_evidence(&transformer.evidence, &source_map, &mut referenced_sources, &mut blockers, ReadinessScope::ThreePhaseLoadFlow, "transformer", &transformer.branch_id, "evidence");
    }
    for (branch_id, kind) in &branch_kinds {
        if *kind == BranchEquipmentKind::Transformer && !transformer_ids.contains(branch_id) {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "transformer", branch_id, "transformerModels", "transformer-classified branch lacks vector-group and winding/grounding model");
        }
    }

    let expected_load_ids: BTreeSet<&str> = case.network.loads.iter().map(|load| load.load_id.as_str()).collect();
    let mut load_ids = BTreeSet::new();
    for allocation in &package.load_phase_allocations {
        if !load_ids.insert(allocation.load_id.as_str()) {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "load", &allocation.load_id, "loadId", "duplicate phase-resolved load allocation");
        }
        let Some(load) = case.network.loads.iter().find(|item| item.load_id == allocation.load_id) else {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "load", &allocation.load_id, "loadId", "phase allocation references load absent from frozen case");
            continue;
        };
        let mut phases = BTreeSet::new();
        let mut active_sum = 0.0;
        let mut reactive_sum = 0.0;
        for phase in &allocation.phases {
            if !matches!(phase.phase, Conductor::A | Conductor::B | Conductor::C) || !phases.insert(phase.phase) {
                blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "load", &allocation.load_id, "phases", "load phase assignments must be unique A/B/C conductors");
            }
            if !phase.active_power_kw.is_finite() || phase.active_power_kw < 0.0 || !phase.reactive_power_kvar.is_finite() {
                blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "load", &allocation.load_id, "phases", "phase-resolved P/Q must be finite and active power non-negative");
            }
            active_sum += phase.active_power_kw;
            reactive_sum += phase.reactive_power_kvar;
        }
        if allocation.phases.is_empty()
            || (active_sum - load.active_power_kw).abs() > FLOAT_TOLERANCE
            || (reactive_sum - load.reactive_power_kvar).abs() > FLOAT_TOLERANCE
        {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "load", &allocation.load_id, "phasePq", "phase P/Q allocation must exactly reconcile with the frozen case load total");
        }
        if let Some(node) = node_map.get(load.node_id.as_str()) {
            for phase in &allocation.phases {
                if !node.conductors.contains(&phase.phase) {
                    blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "load", &allocation.load_id, "phases", "load is allocated to a phase absent from its node model");
                }
            }
        } else {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "load", &allocation.load_id, "nodeId", "load node lacks a native phase model");
        }
        validate_evidence(&allocation.evidence, &source_map, &mut referenced_sources, &mut blockers, ReadinessScope::ThreePhaseLoadFlow, "load", &allocation.load_id, "evidence");
    }
    for load_id in &expected_load_ids {
        if !load_ids.contains(load_id) {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "load", load_id, "loadPhaseAllocations", "missing phase-resolved P/Q assignment");
        }
    }

    let expected_source_ids: BTreeSet<&str> = case.network.grid_sources.iter().map(|source| source.source_id.as_str()).collect();
    let mut source_ids = BTreeSet::new();
    for source in &package.grid_source_models {
        if !source_ids.insert(source.source_id.as_str()) {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "gridSource", &source.source_id, "sourceId", "duplicate grid source model");
        }
        let Some(case_source) = case.network.grid_sources.iter().find(|item| item.source_id == source.source_id) else {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "gridSource", &source.source_id, "sourceId", "source model references source absent from frozen case");
            continue;
        };
        let mut phases = BTreeSet::new();
        for voltage in &source.phase_voltages {
            if !matches!(voltage.phase, Conductor::A | Conductor::B | Conductor::C)
                || !phases.insert(voltage.phase)
                || !voltage.line_neutral_kv.is_finite()
                || voltage.line_neutral_kv <= 0.0
                || !voltage.angle_degrees.is_finite()
            {
                blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "gridSource", &source.source_id, "phaseVoltages", "source phase voltages must be unique, finite and positive");
            }
        }
        if phases != BTreeSet::from([Conductor::A, Conductor::B, Conductor::C]) {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "gridSource", &source.source_id, "phaseVoltages", "three-phase grid source must declare A, B and C voltage phasors");
        }
        if let Some(node) = node_map.get(case_source.node_id.as_str()) {
            for phase in &phases {
                if !node.conductors.contains(phase) {
                    blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "gridSource", &source.source_id, "phaseVoltages", "source phase is absent from source node model");
                }
            }
        } else {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "gridSource", &source.source_id, "nodeId", "source node lacks native phase model");
        }
        add_blocker_if(
            source.short_circuit_power_mva.is_none_or(|value| !value.is_finite() || value <= 0.0)
                || source.x_over_r_ratio.is_none_or(|value| !value.is_finite() || value <= 0.0),
            &mut blockers,
            ReadinessScope::ShortCircuit,
            "gridSource",
            &source.source_id,
            "shortCircuitSource",
            "source short-circuit MVA and X/R ratio must be finite and positive",
        );
        validate_sequence(source.sequence_impedance.as_ref(), &mut blockers, "gridSource", &source.source_id, "sequenceImpedance", ReadinessScope::ShortCircuit);
        validate_evidence(&source.evidence, &source_map, &mut referenced_sources, &mut blockers, ReadinessScope::ThreePhaseLoadFlow, "gridSource", &source.source_id, "evidence");
    }
    for source_id in &expected_source_ids {
        if !source_ids.contains(source_id) {
            blocker(&mut blockers, ReadinessScope::ThreePhaseLoadFlow, "gridSource", source_id, "gridSourceModels", "missing phase voltage and source impedance model");
            blocker(&mut blockers, ReadinessScope::ShortCircuit, "gridSource", source_id, "shortCircuitSource", "missing source short-circuit strength/XR and sequence contribution");
        }
    }

    let required_fault_types = BTreeSet::from([
        FaultType::SingleLineToGround,
        FaultType::LineToLine,
        FaultType::DoubleLineToGround,
        FaultType::ThreePhase,
    ]);
    let expected_faults: BTreeSet<&str> = case.faults.iter().map(|fault| fault.fault_id.as_str()).collect();
    let mut actual_faults = BTreeSet::new();
    for fault in &package.fault_scenarios {
        if !expected_faults.contains(fault.fault_id.as_str()) {
            blocker(&mut blockers, ReadinessScope::ShortCircuit, "faultScenario", &fault.fault_id, "faultId", "fault scenario references fault absent from frozen case");
        }
        if !actual_faults.insert((fault.fault_id.as_str(), fault.fault_type)) {
            blocker(&mut blockers, ReadinessScope::ShortCircuit, "faultScenario", &fault.fault_id, "faultType", "duplicate fault ID/type scenario");
        }
        let required_phase_count = match fault.fault_type {
            FaultType::SingleLineToGround => 1,
            FaultType::LineToLine | FaultType::DoubleLineToGround => 2,
            FaultType::ThreePhase => 3,
        };
        let unique_phases: BTreeSet<_> = fault.phases_involved.iter().copied().collect();
        if unique_phases.len() != fault.phases_involved.len()
            || unique_phases.len() != required_phase_count
            || fault.phases_involved.iter().any(|p| !matches!(p, Conductor::A | Conductor::B | Conductor::C))
        {
            blocker(&mut blockers, ReadinessScope::ShortCircuit, "faultScenario", &fault.fault_id, "phasesInvolved", "fault type requires the exact number of unique A/B/C phases");
        }
        if !fault.fault_impedance_ohm.real.is_finite()
            || fault.fault_impedance_ohm.real < 0.0
            || !fault.fault_impedance_ohm.imaginary.is_finite()
            || fault.contributing_source_ids.is_empty()
            || fault.contributing_source_ids.iter().any(|id| !expected_source_ids.contains(id.as_str()))
        {
            blocker(&mut blockers, ReadinessScope::ShortCircuit, "faultScenario", &fault.fault_id, "faultImpedanceOrSources", "fault impedance or contributing-source set is invalid");
        }
        validate_evidence(&fault.evidence, &source_map, &mut referenced_sources, &mut blockers, ReadinessScope::ShortCircuit, "faultScenario", &fault.fault_id, "evidence");
    }
    for fault_id in &expected_faults {
        for fault_type in &required_fault_types {
            if !actual_faults.contains(&(*fault_id, *fault_type)) {
                blocker(&mut blockers, ReadinessScope::ShortCircuit, "faultScenario", fault_id, "faultTypeCoverage", "missing required single-line-ground/line-line/double-line-ground/three-phase fault scenario");
            }
        }
    }

    let mut protection_device_ids = BTreeSet::new();
    let mut protected_branches = BTreeSet::new();
    for device in &package.protection_devices {
        if !protection_device_ids.insert(device.device_id.as_str()) {
            blocker(&mut blockers, ReadinessScope::ProtectionCoordination, "protectionDevice", &device.device_id, "deviceId", "duplicate protection device ID");
        }
        if !valid_id(&device.coordination_group_id)
            || device.protected_branch_ids.is_empty()
            || !device.protected_branch_ids.iter().all(|id| expected_branch_ids.contains(id.as_str()))
        {
            blocker(&mut blockers, ReadinessScope::ProtectionCoordination, "protectionDevice", &device.device_id, "protectedBranchIds", "protection device requires a coordination group and known protected branches");
        }
        for branch_id in &device.protected_branch_ids {
            protected_branches.insert(branch_id.as_str());
        }
        add_blocker_if(
            device.current_transformer_primary_a.is_none_or(|value| !value.is_finite() || value <= 0.0)
                || device.current_transformer_secondary_a.is_none_or(|value| !value.is_finite() || value <= 0.0)
                || device.voltage_transformer_ratio.is_some_and(|ratio| !ratio.is_finite() || ratio <= 0.0)
                || device.phase_pickup_primary_a.is_none_or(|value| !value.is_finite() || value <= 0.0)
                || device.ground_pickup_primary_a.is_none_or(|value| !value.is_finite() || value <= 0.0)
                || device.interruption_capacity_ka.is_none_or(|value| !value.is_finite() || value <= 0.0),
            &mut blockers,
            ReadinessScope::ProtectionCoordination,
            "protectionDevice",
            &device.device_id,
            "settings",
            "CT/VT ratios, phase/ground pickup and interruption rating must be finite and positive",
        );
        validate_curve(&device.phase_time_current_curve, &device.device_id, "phaseTimeCurrentCurve", &mut blockers);
        validate_curve(&device.ground_time_current_curve, &device.device_id, "groundTimeCurrentCurve", &mut blockers);
        validate_evidence(&device.evidence, &source_map, &mut referenced_sources, &mut blockers, ReadinessScope::ProtectionCoordination, "protectionDevice", &device.device_id, "evidence");
    }

    let mut exempt_branches = BTreeSet::new();
    for exemption in &package.protection_exemptions {
        if !exempt_branches.insert(exemption.branch_id.as_str()) {
            blocker(&mut blockers, ReadinessScope::ProtectionCoordination, "protectionExemption", &exemption.branch_id, "branchId", "duplicate protection exemption");
        }
        if !expected_branch_ids.contains(exemption.branch_id.as_str())
            || !valid_id(&exemption.reason)
            || !valid_id(&exemption.approved_by)
        {
            blocker(&mut blockers, ReadinessScope::ProtectionCoordination, "protectionExemption", &exemption.branch_id, "approval", "exemption must reference a known branch and include reason/approver");
        }
        if protected_branches.contains(exemption.branch_id.as_str()) {
            blocker(&mut blockers, ReadinessScope::ProtectionCoordination, "protectionExemption", &exemption.branch_id, "branchId", "branch cannot be both protected and exempted");
        }
        validate_evidence(&exemption.approval_evidence, &source_map, &mut referenced_sources, &mut blockers, ReadinessScope::ProtectionCoordination, "protectionExemption", &exemption.branch_id, "approvalEvidence");
    }
    for branch_id in &expected_branch_ids {
        if !protected_branches.contains(branch_id) && !exempt_branches.contains(branch_id) {
            blocker(&mut blockers, ReadinessScope::ProtectionCoordination, "branch", branch_id, "protectionCoverage", "branch has neither a protection-device mapping nor an approved exemption");
        }
    }

    for source in &package.source_artifacts {
        if !referenced_sources.contains(&source.artifact_uri) {
            blocker(
                &mut blockers,
                ReadinessScope::ThreePhaseLoadFlow,
                "sourceArtifact",
                &source.artifact_uri,
                "sourceArtifacts",
                "source inventory contains an unreferenced artifact",
            );
        }
    }

    blockers.sort();
    blockers.dedup();
    let load_flow_count = blockers.iter().filter(|item| item.scope == ReadinessScope::ThreePhaseLoadFlow).count();
    let short_circuit_count = blockers.iter().filter(|item| item.scope == ReadinessScope::ShortCircuit).count();
    let protection_count = blockers.iter().filter(|item| item.scope == ReadinessScope::ProtectionCoordination).count();
    let load_flow_ready = load_flow_count == 0;
    let short_ready = load_flow_ready && short_circuit_count == 0;
    let protection_ready = short_ready && protection_count == 0;

    let load_flow = ScopeReadiness {
        state: if load_flow_ready { ReadinessState::ReadyForIndependentDomainSolver } else { ReadinessState::InconclusiveMissingOrInvalidNativeInputs },
        blocker_count: load_flow_count,
    };
    let short_circuit = ScopeReadiness {
        state: if short_ready { ReadinessState::ReadyForIndependentDomainSolver } else { ReadinessState::InconclusiveMissingOrInvalidNativeInputs },
        blocker_count: load_flow_count + short_circuit_count,
    };
    let protection = ScopeReadiness {
        state: if protection_ready { ReadinessState::ReadyForIndependentDomainSolver } else { ReadinessState::InconclusiveMissingOrInvalidNativeInputs },
        blocker_count: load_flow_count + short_circuit_count + protection_count,
    };
    let result_status = if protection_ready {
        "native_three_phase_and_protection_inputs_ready_for_independent_domain_solver"
    } else {
        "inconclusive_missing_or_invalid_native_inputs"
    };

    Ok(ThreePhaseReadinessReport {
        package_id: package.package_id.clone(),
        case_sha256: case_sha,
        package_sha256: package_sha,
        source_artifact_count: package.source_artifacts.len(),
        three_phase_load_flow: load_flow,
        short_circuit,
        protection_coordination: protection,
        blocker_count: blockers.len(),
        blockers,
        result_status: result_status.to_string(),
        claim_boundary: "Native three-phase, sequence, grounding, fault and protection input readiness only. This report does not solve AC power flow, short-circuit current, relay coordination, switching transients or prove physical safety. Positive-sequence/balanced scalar inputs are not substituted for native phase/sequence data. A ready report only authorizes hand-off to an independently qualified domain solver.".to_string(),
    })
}

pub fn parse_and_audit(
    package_json: &str,
    case_json: &str,
) -> Result<ThreePhaseReadinessReport, Vec<String>> {
    let package: ThreePhaseInputPackage = serde_json::from_str(package_json)
        .map_err(|error| vec![format!("three-phase package JSON decode error: {error}")])?;
    let case: BenchmarkCase = serde_json::from_str(case_json)
        .map_err(|error| vec![format!("benchmark case JSON decode error: {error}")])?;
    audit_three_phase_inputs(&case, &package)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case() -> BenchmarkCase {
        serde_json::from_str(include_str!(
            "../../power-restoration-benchmark-case/fixtures/synthetic-restoration-case-v0.1.json"
        )).unwrap()
    }

    fn evidence(id: &str) -> EvidenceRef {
        EvidenceRef {
            artifact_uri: format!("evidence://threephase/{id}"),
            sha256: "d".repeat(64),
            producer_id: "synthetic-utility-data-provider".to_string(),
            producer_profile_sha256: "e".repeat(64),
        }
    }

    fn build_valid_package(case: &BenchmarkCase) -> ThreePhaseInputPackage {
        let nodes = case.network.nodes.iter().map(|node| NodePhaseModel {
            node_id: node.node_id.clone(),
            conductors: vec![Conductor::A, Conductor::B, Conductor::C, Conductor::Neutral],
            grounding_type: GroundingType::MultiGroundedNeutral,
            neutral_continuous: true,
            grounding_impedance_ohm: Some(ComplexValue { real: 0.1, imaginary: 0.05 }),
            evidence: evidence(&format!("node-{}", node.node_id)),
        }).collect::<Vec<_>>();

        let branches = case.network.branches.iter().map(|branch| {
            let conductors = vec![Conductor::A, Conductor::B, Conductor::C, Conductor::Neutral];
            let n = conductors.len();
            let mut r = vec![vec![0.01; n]; n];
            let mut x = vec![vec![0.05; n]; n];
            for i in 0..n {
                r[i][i] = 0.2 + i as f64 * 0.01;
                x[i][i] = 0.4 + i as f64 * 0.01;
            }
            BranchPhaseModel {
                branch_id: branch.branch_id.clone(),
                equipment_kind: BranchEquipmentKind::OverheadLine,
                conductors: conductors.clone(),
                primitive_impedance: Some(PrimitiveImpedanceMatrix {
                    unit: PrimitiveMatrixUnit::OhmPerKm,
                    conductor_order: conductors.clone(),
                    resistance_ohm_per_km_or_base: r,
                    reactance_ohm_per_km_or_base: x,
                }),
                sequence_impedance: Some(SequenceImpedance {
                    positive: ComplexValue { real: 0.2, imaginary: 0.4 },
                    negative: ComplexValue { real: 0.21, imaginary: 0.41 },
                    zero: ComplexValue { real: 0.5, imaginary: 1.2 },
                    unit: "ohm".to_string(),
                }),
                current_ratings: conductors.iter().map(|conductor| ConductorCurrentRating {
                    conductor: *conductor,
                    continuous_ampacity_a: 400.0,
                    emergency_ampacity_a: 500.0,
                }).collect(),
                evidence: evidence(&format!("branch-{}", branch.branch_id)),
            }
        }).collect::<Vec<_>>();

        let loads = case.network.loads.iter().map(|load| LoadPhaseAllocation {
            load_id: load.load_id.clone(),
            phases: [Conductor::A, Conductor::B, Conductor::C].into_iter().map(|phase| PhasePower {
                phase,
                active_power_kw: load.active_power_kw / 3.0,
                reactive_power_kvar: load.reactive_power_kvar / 3.0,
            }).collect(),
            evidence: evidence(&format!("load-{}", load.load_id)),
        }).collect::<Vec<_>>();

        let sources = case.network.grid_sources.iter().map(|source| GridSourceModel {
            source_id: source.source_id.clone(),
            phase_voltages: vec![
                PhaseVoltage { phase: Conductor::A, line_neutral_kv: 7.2, angle_degrees: 0.0 },
                PhaseVoltage { phase: Conductor::B, line_neutral_kv: 7.2, angle_degrees: -120.0 },
                PhaseVoltage { phase: Conductor::C, line_neutral_kv: 7.2, angle_degrees: 120.0 },
            ],
            sequence_impedance: Some(SequenceImpedance {
                positive: ComplexValue { real: 0.05, imaginary: 0.4 },
                negative: ComplexValue { real: 0.05, imaginary: 0.4 },
                zero: ComplexValue { real: 0.15, imaginary: 0.9 },
                unit: "ohm".to_string(),
            }),
            short_circuit_power_mva: Some(250.0),
            x_over_r_ratio: Some(8.0),
            evidence: evidence(&format!("source-{}", source.source_id)),
        }).collect::<Vec<_>>();

        let contributing_source_ids = case.network.grid_sources.iter()
            .map(|source| source.source_id.clone())
            .collect::<Vec<_>>();
        let fault_scenarios = case.faults.iter().flat_map(|fault| {
            let contributing_source_ids = contributing_source_ids.clone();
            [
                (FaultType::SingleLineToGround, vec![Conductor::A]),
                (FaultType::LineToLine, vec![Conductor::A, Conductor::B]),
                (FaultType::DoubleLineToGround, vec![Conductor::B, Conductor::C]),
                (FaultType::ThreePhase, vec![Conductor::A, Conductor::B, Conductor::C]),
            ].into_iter().map(move |(fault_type, phases)| FaultScenario {
                fault_id: fault.fault_id.clone(),
                fault_type,
                phases_involved: phases,
                fault_impedance_ohm: ComplexValue { real: 0.01, imaginary: 0.0 },
                contributing_source_ids: contributing_source_ids.clone(),
                evidence: evidence(&format!("fault-{}-{fault_type:?}", fault.fault_id)),
            })
        }).collect::<Vec<_>>();

        let protection_devices = case.network.branches.iter().map(|branch| ProtectionDeviceModel {
            device_id: format!("relay-{}", branch.branch_id),
            device_kind: ProtectionDeviceKind::Relay,
            protected_branch_ids: vec![branch.branch_id.clone()],
            current_transformer_primary_a: Some(400.0),
            current_transformer_secondary_a: Some(5.0),
            voltage_transformer_ratio: Some(60.0),
            phase_pickup_primary_a: Some(240.0),
            ground_pickup_primary_a: Some(60.0),
            interruption_capacity_ka: Some(25.0),
            phase_time_current_curve: vec![
                TimeCurrentPoint { current_multiple_of_pickup: 1.5, trip_time_seconds: 1.0 },
                TimeCurrentPoint { current_multiple_of_pickup: 5.0, trip_time_seconds: 0.2 },
                TimeCurrentPoint { current_multiple_of_pickup: 10.0, trip_time_seconds: 0.08 },
            ],
            ground_time_current_curve: vec![
                TimeCurrentPoint { current_multiple_of_pickup: 1.5, trip_time_seconds: 1.2 },
                TimeCurrentPoint { current_multiple_of_pickup: 5.0, trip_time_seconds: 0.25 },
                TimeCurrentPoint { current_multiple_of_pickup: 10.0, trip_time_seconds: 0.1 },
            ],
            coordination_group_id: "feeder-zone-1".to_string(),
            evidence: evidence(&format!("protection-{}", branch.branch_id)),
        }).collect::<Vec<_>>();

        let mut package = ThreePhaseInputPackage {
            schema_version: "0.1.0".to_string(),
            package_id: "synthetic-threephase-package-v1".to_string(),
            case_sha256: canonical_case_sha256(case).unwrap(),
            base: ElectricalBase {
                frequency_hz: 60.0,
                base_mva: 10.0,
                nominal_line_line_kv: 12.47,
                voltage_min_pu: 0.95,
                voltage_max_pu: 1.05,
            },
            producer: PackageProducer {
                producer_id: "synthetic-threephase-data-converter".to_string(),
                source_commit: "1".repeat(40),
                configuration_sha256: "2".repeat(64),
            },
            node_models: nodes,
            branch_models: branches,
            transformer_models: Vec::new(),
            load_phase_allocations: loads,
            grid_source_models: sources,
            fault_scenarios,
            protection_devices,
            protection_exemptions: Vec::new(),
            source_artifacts: Vec::new(),
        };
        package.source_artifacts = collect_source_artifacts(&package);
        package
    }

    fn collect_source_artifacts(package: &ThreePhaseInputPackage) -> Vec<ArtifactRef> {
        let mut by_uri = BTreeMap::<String, String>::new();
        let mut add = |e: &EvidenceRef| { by_uri.insert(e.artifact_uri.clone(), e.sha256.clone()); };
        package.node_models.iter().for_each(|x| add(&x.evidence));
        package.branch_models.iter().for_each(|x| add(&x.evidence));
        package.transformer_models.iter().for_each(|x| add(&x.evidence));
        package.load_phase_allocations.iter().for_each(|x| add(&x.evidence));
        package.grid_source_models.iter().for_each(|x| add(&x.evidence));
        package.fault_scenarios.iter().for_each(|x| add(&x.evidence));
        package.protection_devices.iter().for_each(|x| add(&x.evidence));
        package.protection_exemptions.iter().for_each(|x| add(&x.approval_evidence));
        by_uri.into_iter().map(|(artifact_uri, sha256)| ArtifactRef { artifact_uri, sha256 }).collect()
    }

    #[test]
    fn complete_native_threephase_package_is_ready_for_solver_handoff_only() {
        let case = case();
        let package = build_valid_package(&case);
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert_eq!(report.result_status, "native_three_phase_and_protection_inputs_ready_for_independent_domain_solver");
        assert_eq!(report.blocker_count, 0);
        assert_eq!(report.three_phase_load_flow.state, ReadinessState::ReadyForIndependentDomainSolver);
        assert_eq!(report.short_circuit.state, ReadinessState::ReadyForIndependentDomainSolver);
        assert_eq!(report.protection_coordination.state, ReadinessState::ReadyForIndependentDomainSolver);
    }

    #[test]
    fn topology_only_case_is_not_promoted_without_native_extension() {
        let case = case();
        let package = build_valid_package(&case);
        let mut broken = package;
        broken.branch_models.clear();
        let report = audit_three_phase_inputs(&case, &broken).unwrap();
        assert_eq!(report.three_phase_load_flow.state, ReadinessState::InconclusiveMissingOrInvalidNativeInputs);
        assert!(report.blockers.iter().any(|b| b.field_path == "branchModels"));
    }

    #[test]
    fn missing_negative_or_zero_sequence_data_blocks_short_circuit_readiness() {
        let case = case();
        let mut package = build_valid_package(&case);
        let sequence = package.branch_models[0].sequence_impedance.as_mut().unwrap();
        sequence.zero.real = 0.0;
        sequence.zero.imaginary = 0.0;
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert_eq!(report.three_phase_load_flow.state, ReadinessState::ReadyForIndependentDomainSolver);
        assert_eq!(report.short_circuit.state, ReadinessState::InconclusiveMissingOrInvalidNativeInputs);
        assert!(report.blockers.iter().any(|b| b.field_path == "sequenceImpedance"));
    }

    #[test]
    fn balanced_scalar_impedance_cannot_replace_native_matrix() {
        let case = case();
        let mut package = build_valid_package(&case);
        package.branch_models[0].primitive_impedance.as_mut().unwrap().resistance_ohm_per_km_or_base.clear();
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert_eq!(report.three_phase_load_flow.state, ReadinessState::InconclusiveMissingOrInvalidNativeInputs);
        assert!(report.blockers.iter().any(|b| b.field_path == "primitiveImpedance"));
    }

    #[test]
    fn phase_load_pq_must_reconcile_with_frozen_case_load() {
        let case = case();
        let mut package = build_valid_package(&case);
        package.load_phase_allocations[0].phases[0].active_power_kw += 10.0;
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert_eq!(report.three_phase_load_flow.state, ReadinessState::InconclusiveMissingOrInvalidNativeInputs);
        assert!(report.blockers.iter().any(|b| b.field_path == "phasePq"));
    }

    #[test]
    fn fault_scenario_matrix_requires_every_fault_type_per_fault_location() {
        let case = case();
        let mut package = build_valid_package(&case);
        package.fault_scenarios.retain(|f| f.fault_type != FaultType::DoubleLineToGround);
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert_eq!(report.short_circuit.state, ReadinessState::InconclusiveMissingOrInvalidNativeInputs);
        assert!(report.blockers.iter().any(|b| b.field_path == "faultTypeCoverage"));
    }

    #[test]
    fn protection_input_missing_curve_blocks_protection_only() {
        let case = case();
        let mut package = build_valid_package(&case);
        package.protection_devices[0].ground_time_current_curve.clear();
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert_eq!(report.three_phase_load_flow.state, ReadinessState::ReadyForIndependentDomainSolver);
        assert_eq!(report.short_circuit.state, ReadinessState::ReadyForIndependentDomainSolver);
        assert_eq!(report.protection_coordination.state, ReadinessState::InconclusiveMissingOrInvalidNativeInputs);
    }

    #[test]
    fn every_branch_requires_protection_device_or_approved_exemption() {
        let case = case();
        let mut package = build_valid_package(&case);
        package.protection_devices.retain(|device| device.protected_branch_ids[0] != "B12");
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert_eq!(report.protection_coordination.state, ReadinessState::InconclusiveMissingOrInvalidNativeInputs);
        assert!(report.blockers.iter().any(|b| b.field_path == "protectionCoverage" && b.object_id == "B12"));
    }

    #[test]
    fn transformer_branch_requires_vector_group_and_winding_model() {
        let case = case();
        let mut package = build_valid_package(&case);
        package.branch_models[0].equipment_kind = BranchEquipmentKind::Transformer;
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert!(report.blockers.iter().any(|b| b.field_path == "transformerModels"));
    }

    #[test]
    fn post_verifier_source_digest_substitution_is_detected_by_inventory_check() {
        let case = case();
        let mut package = build_valid_package(&case);
        let original = package.branch_models[0].evidence.artifact_uri.clone();
        package.source_artifacts.iter_mut().find(|a| a.artifact_uri == original).unwrap().sha256 = "9".repeat(64);
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert!(report.blockers.iter().any(|b| b.reason.contains("digest differs")));
    }

    #[test]
    fn duplicate_branch_and_node_models_are_rejected() {
        let case = case();
        let mut package = build_valid_package(&case);
        package.branch_models.push(package.branch_models[0].clone());
        package.node_models.push(package.node_models[0].clone());
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert!(report.blockers.iter().any(|b| b.reason.contains("duplicate branch")));
        assert!(report.blockers.iter().any(|b| b.reason.contains("duplicate node")));
    }

    #[test]
    fn impedance_matrix_requires_symmetric_primitive_entries() {
        let case = case();
        let mut package = build_valid_package(&case);
        package.branch_models[0].primitive_impedance.as_mut().unwrap().resistance_ohm_per_km_or_base[0][1] = 0.2;
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert!(report.blockers.iter().any(|b| b.reason.contains("symmetric")));
    }

    #[test]
    fn non_passive_resistance_matrix_is_rejected() {
        let case = case();
        let mut package = build_valid_package(&case);
        let resistance = &mut package.branch_models[0]
            .primitive_impedance.as_mut().unwrap().resistance_ohm_per_km_or_base;
        resistance[0][1] = 0.3;
        resistance[1][0] = 0.3;
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert!(report.blockers.iter().any(|b| b.field_path == "primitiveImpedance.resistance"));
    }

    #[test]
    fn missing_source_short_circuit_strength_blocks_short_circuit_only() {
        let case = case();
        let mut package = build_valid_package(&case);
        package.grid_source_models[0].short_circuit_power_mva = None;
        package.grid_source_models[0].x_over_r_ratio = None;
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert_eq!(report.three_phase_load_flow.state, ReadinessState::ReadyForIndependentDomainSolver);
        assert_eq!(report.short_circuit.state, ReadinessState::InconclusiveMissingOrInvalidNativeInputs);
    }

    #[test]
    fn missing_protection_ct_or_interruption_rating_blocks_protection_only() {
        let case = case();
        let mut package = build_valid_package(&case);
        package.protection_devices[0].current_transformer_primary_a = None;
        package.protection_devices[0].interruption_capacity_ka = None;
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert_eq!(report.three_phase_load_flow.state, ReadinessState::ReadyForIndependentDomainSolver);
        assert_eq!(report.short_circuit.state, ReadinessState::ReadyForIndependentDomainSolver);
        assert_eq!(report.protection_coordination.state, ReadinessState::InconclusiveMissingOrInvalidNativeInputs);
    }

    #[test]
    fn missing_native_matrix_and_sequence_are_reported_as_blockers() {
        let case = case();
        let mut package = build_valid_package(&case);
        package.branch_models[0].primitive_impedance = None;
        package.branch_models[0].sequence_impedance = None;
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert!(report.blockers.iter().any(|b| b.field_path == "primitiveImpedance"));
        assert!(report.blockers.iter().any(|b| b.field_path == "sequenceImpedance"));
    }

    #[test]
    fn evidence_not_in_closed_source_inventory_is_rejected() {
        let case = case();
        let mut package = build_valid_package(&case);
        package.source_artifacts.pop();
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert!(report.blockers.iter().any(|b| b.reason.contains("absent from source_artifacts")));
    }

    #[test]
    fn invalid_non_monotonic_time_current_curve_blocks_protection() {
        let case = case();
        let mut package = build_valid_package(&case);
        package.protection_devices[0].phase_time_current_curve.swap(0, 1);
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert!(report.blockers.iter().any(|b| b.field_path == "phaseTimeCurrentCurve"));
    }

    #[test]
    fn phase_grounding_node_neutral_missing_is_blocked() {
        let case = case();
        let mut package = build_valid_package(&case);
        package.node_models[0].neutral_continuous = false;
        package.node_models[0].conductors.retain(|c| *c != Conductor::Neutral);
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert!(report.blockers.iter().any(|b| b.field_path == "conductors"));
        assert!(report.blockers.iter().any(|b| b.field_path == "endpoints" || b.field_path == "conductors"));
    }

    #[test]
    fn wrong_frozen_case_digest_blocks_all_qualification_scopes() {
        let case = case();
        let mut package = build_valid_package(&case);
        package.case_sha256 = "f".repeat(64);
        let report = audit_three_phase_inputs(&case, &package).unwrap();
        assert_eq!(report.three_phase_load_flow.state, ReadinessState::InconclusiveMissingOrInvalidNativeInputs);
        assert_eq!(report.short_circuit.state, ReadinessState::InconclusiveMissingOrInvalidNativeInputs);
        assert_eq!(report.protection_coordination.state, ReadinessState::InconclusiveMissingOrInvalidNativeInputs);
    }
}
