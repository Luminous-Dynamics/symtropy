use orbital_interface_conformance_bench::{
    evaluate as evaluate_nominal, ConformanceCase, DecisionStatus, Fixture as NominalFixture,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

const ROBUST_SCHEMA: &str = "luminous.orbital-robust-compatibility.corpus.v0.1";
const SOURCE_SCHEMA: &str = "luminous.orbital-robust-compatibility.uq-adapter-envelope.v0.1";
const CLAIM_CEILING: &str = "synthetic-profile-only";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UqDisposition { Qualified, Unknown, Inconclusive, Rejected }

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UqRepresentation {
    HardBoundedError,
    StandardUncertainty,
    ExpandedUncertainty,
    CoverageInterval,
    PredictionInterval,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependenceDisposition {
    JointModel,
    EvidenceBackedIndependence,
    NoSharedSourceAssessed,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RobustStatus {
    RobustlyCompatibleSynthetic,
    Rejected,
    Indeterminate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RobustFixture {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<RobustCase>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RobustCase {
    pub case_id: String,
    pub at_tick: u64,
    pub context_of_use: String,
    pub provider_lifecycle_state_id: String,
    pub client_lifecycle_state_id: String,
    pub nominal_case: ConformanceCase,
    pub uncertainty_receipts: Vec<UncertaintyReceipt>,
    pub expected_status: RobustStatus,
    pub expected_reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UncertaintyReceipt {
    pub receipt_id: String,
    pub source_schema: String,
    pub source_commit: String,
    pub artifact_sha256: String,
    pub profile_id: String,
    pub configuration_revision: String,
    pub lifecycle_state_id: String,
    pub measurand_id: String,
    pub unit: String,
    pub representation: UqRepresentation,
    pub disposition: UqDisposition,
    pub uncertainty_minus: f64,
    pub uncertainty_plus: f64,
    pub context_of_use: String,
    pub valid_from_tick: u64,
    pub valid_until_tick: u64,
    pub claim_ceiling: String,
    pub dependence: DependenceEvidence,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProfileBinding {
    pub profile_id: String,
    pub configuration_revision: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DependenceEvidence {
    pub group_id: String,
    pub profile_bindings: Vec<ProfileBinding>,
    pub context_of_use: String,
    pub disposition: DependenceDisposition,
    pub receipt_id: String,
    pub artifact_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarginResult {
    pub profile_id: String,
    pub measurand_id: String,
    pub unit: String,
    pub margin_lower: f64,
    pub margin_upper: Option<f64>,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub nominal_status: DecisionStatus,
    pub actual_status: RobustStatus,
    pub actual_reason: String,
    pub expected_status: RobustStatus,
    pub expected_reason: String,
    pub expected_result_matched: bool,
    pub margins: Vec<MarginResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub robustly_compatible_synthetic: usize,
    pub rejected: usize,
    pub indeterminate: usize,
    pub expected_result_mismatches: usize,
    pub false_robust_accepts: usize,
    pub authority_grants: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub authority_granted: bool,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
}

#[derive(Debug, Clone, Copy)]
struct Decision {
    status: RobustStatus,
    reason: &'static str,
}

impl Decision {
    fn compatible() -> Self { Self { status: RobustStatus::RobustlyCompatibleSynthetic, reason: "all_hard_bound_margins_nonnegative" } }
    fn rejected(reason: &'static str) -> Self { Self { status: RobustStatus::Rejected, reason } }
    fn indeterminate(reason: &'static str) -> Self { Self { status: RobustStatus::Indeterminate, reason } }
}

#[derive(Debug, Clone, Copy)]
struct Bound {
    minus: f64,
    plus: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScalarRule {
    ProviderMinimumMeetsClientMinimum,
    ProviderMaximumErrorBelowClientTolerance,
}

#[derive(Debug, Clone, Copy)]
struct MetricSpec<'a> {
    profile_id: &'a str,
    measurand_id: &'a str,
    unit: &'a str,
    scalar_rule: ScalarRule,
}

#[derive(Debug, Clone, Copy)]
struct DependencyClaim<'a> {
    receipt_id: &'a str,
    digest: &'a str,
    disposition: DependenceDisposition,
    profile_bindings: &'a [ProfileBinding],
}

#[derive(Debug, Clone, Copy)]
struct Requirement {
    measurand_id: &'static str,
    unit: &'static str,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        write!(&mut out, "{byte:02x}").expect("writing to String is infallible");
    }
    out
}

fn valid_hex(value: &str, len: usize) -> bool {
    value.len() == len && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn present(value: &str) -> bool { !value.trim().is_empty() }

fn profile_fields(
    case: &RobustCase,
    provider: bool,
) -> Result<(String, String, String), Decision> {
    let profile = if provider { &case.nominal_case.provider } else { &case.nominal_case.client };
    let lifecycle = if provider { &case.provider_lifecycle_state_id } else { &case.client_lifecycle_state_id };
    let profile_id = profile.profile_id.clone().ok_or(Decision::indeterminate("missing_profile_id"))?;
    let config = profile.config_revision.clone().ok_or(Decision::indeterminate("missing_configuration_revision"))?;
    if !present(&profile_id) || !present(&config) || !present(lifecycle) {
        return Err(Decision::indeterminate("incomplete_lifecycle_identity"));
    }
    Ok((profile_id, config, lifecycle.clone()))
}

fn receipt_bound(
    receipt: &UncertaintyReceipt,
    profile_id: &str,
    config_revision: &str,
    lifecycle_state_id: &str,
    partner_profile_id: &str,
    partner_config_revision: &str,
    requirement: Requirement,
    context: &str,
    at_tick: u64,
) -> Result<Bound, Decision> {
    if !present(&receipt.receipt_id)
        || !present(&receipt.source_schema)
        || !present(&receipt.source_commit)
        || !present(&receipt.artifact_sha256)
        || !present(&receipt.profile_id)
        || !present(&receipt.configuration_revision)
        || !present(&receipt.lifecycle_state_id)
        || !present(&receipt.measurand_id)
        || !present(&receipt.unit)
        || !present(&receipt.context_of_use)
        || !present(&receipt.claim_ceiling)
    {
        return Err(Decision::indeterminate("incomplete_uncertainty_receipt"));
    }
    if receipt.source_schema != SOURCE_SCHEMA {
        return Err(Decision::indeterminate("unsupported_uncertainty_source_schema"));
    }
    if !valid_hex(&receipt.source_commit, 40) || !valid_hex(&receipt.artifact_sha256, 64) {
        return Err(Decision::rejected("malformed_uncertainty_provenance"));
    }
    if receipt.claim_ceiling != CLAIM_CEILING {
        return Err(Decision::rejected("uncertainty_claim_ceiling_escalation"));
    }
    if receipt.profile_id != profile_id
        || receipt.configuration_revision != config_revision
        || receipt.lifecycle_state_id != lifecycle_state_id
        || receipt.measurand_id != requirement.measurand_id
        || receipt.unit != requirement.unit
        || receipt.context_of_use != context
    {
        return Err(Decision::indeterminate("uncertainty_scope_mismatch"));
    }
    if receipt.valid_from_tick > receipt.valid_until_tick {
        return Err(Decision::rejected("inverted_uncertainty_validity"));
    }
    if at_tick < receipt.valid_from_tick || at_tick > receipt.valid_until_tick {
        return Err(Decision::indeterminate("uncertainty_receipt_expired"));
    }

    match receipt.disposition {
        UqDisposition::Qualified => {}
        UqDisposition::Unknown | UqDisposition::Inconclusive => return Err(Decision::indeterminate("upstream_uncertainty_not_qualified")),
        UqDisposition::Rejected => return Err(Decision::rejected("upstream_uncertainty_rejected")),
    }
    if receipt.representation != UqRepresentation::HardBoundedError {
        return Err(Decision::indeterminate("probabilistic_or_expanded_uncertainty_needs_policy"));
    }
    if !receipt.uncertainty_minus.is_finite()
        || !receipt.uncertainty_plus.is_finite()
        || receipt.uncertainty_minus < 0.0
        || receipt.uncertainty_plus < 0.0
    {
        return Err(Decision::rejected("invalid_hard_bound_uncertainty"));
    }

    let dep = &receipt.dependence;
    if !present(&dep.group_id) || !present(&dep.receipt_id) || !valid_hex(&dep.artifact_sha256, 64) {
        return Err(Decision::indeterminate("missing_dependence_evidence"));
    }
    if dep.context_of_use != context || dep.profile_bindings.is_empty() {
        return Err(Decision::indeterminate("dependence_evidence_scope_mismatch"));
    }
    let mut seen_bindings = HashSet::new();
    for binding in &dep.profile_bindings {
        if !present(&binding.profile_id) || !present(&binding.configuration_revision) {
            return Err(Decision::indeterminate("dependence_evidence_scope_mismatch"));
        }
        if !seen_bindings.insert((binding.profile_id.as_str(), binding.configuration_revision.as_str())) {
            return Err(Decision::rejected("duplicate_dependence_profile_binding"));
        }
    }
    let current_profile_bound = dep.profile_bindings.iter().any(|binding| {
        binding.profile_id == profile_id && binding.configuration_revision == config_revision
    });
    let partner_profile_bound = dep.profile_bindings.iter().any(|binding| {
        binding.profile_id == partner_profile_id
            && binding.configuration_revision == partner_config_revision
    });
    if !current_profile_bound || !partner_profile_bound {
        return Err(Decision::indeterminate("dependence_evidence_scope_mismatch"));
    }
    if dep.disposition == DependenceDisposition::Unknown {
        return Err(Decision::indeterminate("unknown_measurement_dependence"));
    }
    Ok(Bound { minus: receipt.uncertainty_minus, plus: receipt.uncertainty_plus })
}

fn scalar_margin(
    provider_nominal: f64,
    client_nominal: f64,
    provider: Bound,
    client: Bound,
    rule: ScalarRule,
) -> Result<f64, Decision> {
    if !provider_nominal.is_finite() || !client_nominal.is_finite() {
        return Err(Decision::rejected("non_finite_nominal_value"));
    }
    match rule {
        ScalarRule::ProviderMinimumMeetsClientMinimum => {
            // Conservative provider lower capability must meet the client's upper requirement.
            Ok((provider_nominal - provider.minus) - (client_nominal + client.plus))
        }
        ScalarRule::ProviderMaximumErrorBelowClientTolerance => {
            // Provider's worst-case error must remain below the client's minimum tolerance.
            Ok((client_nominal - client.minus) - (provider_nominal + provider.plus))
        }
    }
}

fn check_scalar(
    p: f64,
    c: f64,
    p_uq: Bound,
    c_uq: Bound,
    spec: MetricSpec<'_>,
    margins: &mut Vec<MarginResult>,
) -> Result<(), Decision> {
    let margin = scalar_margin(p, c, p_uq, c_uq, spec.scalar_rule)?;
    let direction = match spec.scalar_rule {
        ScalarRule::ProviderMinimumMeetsClientMinimum =>
            "provider_conservative_lower_minus_client_conservative_upper",
        ScalarRule::ProviderMaximumErrorBelowClientTolerance =>
            "client_conservative_tolerance_minus_provider_worst_case_error",
    };
    margins.push(MarginResult {
        profile_id: spec.profile_id.to_owned(),
        measurand_id: spec.measurand_id.to_owned(),
        unit: spec.unit.to_owned(),
        margin_lower: margin,
        margin_upper: None,
        direction: direction.to_owned(),
    });
    if margin < 0.0 {
        return Err(Decision::rejected("uncertainty_guard_band_exceeds_scalar_margin"));
    }
    Ok(())
}

fn get_scalar(value: &Option<orbital_interface_conformance_bench::Scalar>) -> Result<f64, Decision> {
    let value = value.as_ref().and_then(|x| x.value).ok_or(Decision::indeterminate("missing_nominal_scalar"))?;
    if !value.is_finite() || value < 0.0 { return Err(Decision::rejected("invalid_nominal_scalar")); }
    Ok(value)
}

fn get_interval(value: &Option<orbital_interface_conformance_bench::Interval>) -> Result<(f64, f64), Decision> {
    let interval = value.as_ref().ok_or(Decision::indeterminate("missing_nominal_interval"))?;
    let lo = interval.min.ok_or(Decision::indeterminate("missing_nominal_interval_bound"))?;
    let hi = interval.max.ok_or(Decision::indeterminate("missing_nominal_interval_bound"))?;
    if !lo.is_finite() || !hi.is_finite() || lo < 0.0 || hi < lo {
        return Err(Decision::rejected("invalid_nominal_interval"));
    }
    Ok((lo, hi))
}

fn check_interval(
    p: (f64, f64),
    c: (f64, f64),
    p_uq: Bound,
    c_uq: Bound,
    spec: MetricSpec<'_>,
    margins: &mut Vec<MarginResult>,
) -> Result<(), Decision> {
    // Provider interval shrinks; client-required interval expands.
    let provider_min_safe = p.0 + p_uq.plus;
    let provider_max_safe = p.1 - p_uq.minus;
    let client_min_required = c.0 - c_uq.minus;
    let client_max_required = c.1 + c_uq.plus;
    let lower_margin = client_min_required - provider_min_safe;
    let upper_margin = provider_max_safe - client_max_required;
    margins.push(MarginResult {
        profile_id: spec.profile_id.to_owned(),
        measurand_id: spec.measurand_id.to_owned(),
        unit: spec.unit.to_owned(),
        margin_lower: lower_margin,
        margin_upper: Some(upper_margin),
        direction: "client_expanded_interval_inside_provider_shrunken_interval".to_owned(),
    });
    if lower_margin < 0.0 || upper_margin < 0.0 {
        return Err(Decision::rejected("uncertainty_guard_band_exceeds_interval_margin"));
    }
    Ok(())
}

fn find_receipt<'a>(
    case: &'a RobustCase,
    profile_id: &str,
    measurand_id: &str,
) -> Result<&'a UncertaintyReceipt, Decision> {
    let matches: Vec<_> = case.uncertainty_receipts.iter()
        .filter(|r| r.profile_id == profile_id && r.measurand_id == measurand_id)
        .collect();
    match matches.as_slice() {
        [one] => Ok(*one),
        [] => Err(Decision::indeterminate("missing_uncertainty_measurand_receipt")),
        _ => Err(Decision::rejected("duplicate_uncertainty_measurand_receipt")),
    }
}

fn evaluate_case(case: &RobustCase) -> (Decision, DecisionStatus, Vec<MarginResult>) {
    let nominal_fixture = NominalFixture {
        schema: "luminous.orbital-interface-conformance.corpus.v0.1".to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        cases: vec![case.nominal_case.clone()],
    };
    let nominal = match evaluate_nominal(&nominal_fixture) {
        Ok(report) => report.cases.first().map(|r| r.actual_status)
            .unwrap_or(DecisionStatus::Indeterminate),
        Err(_) => DecisionStatus::Indeterminate,
    };
    if nominal != DecisionStatus::CompatibleSyntheticProfile {
        let decision = match nominal {
            DecisionStatus::Rejected => Decision::rejected("nominal_compatibility_rejected"),
            DecisionStatus::Indeterminate => Decision::indeterminate("nominal_compatibility_indeterminate"),
            DecisionStatus::CompatibleSyntheticProfile => Decision::compatible(),
        };
        return (decision, nominal, Vec::new());
    }

    let (provider_id, provider_config, provider_state) = match profile_fields(case, true) {
        Ok(v) => v, Err(d) => return (d, nominal, Vec::new()),
    };
    let (client_id, client_config, client_state) = match profile_fields(case, false) {
        Ok(v) => v, Err(d) => return (d, nominal, Vec::new()),
    };

    let mut ids = HashSet::new();
    for receipt in &case.uncertainty_receipts {
        if !ids.insert(receipt.receipt_id.as_str()) {
            return (Decision::rejected("duplicate_uncertainty_receipt_id"), nominal, Vec::new());
        }
    }

    let mut dependence_groups: HashMap<&str, DependencyClaim<'_>> = HashMap::new();
    for receipt in &case.uncertainty_receipts {
        let dep = &receipt.dependence;
        if let Some(claim) = dependence_groups.get(dep.group_id.as_str()) {
            if claim.receipt_id != dep.receipt_id.as_str()
                || claim.digest != dep.artifact_sha256.as_str()
                || claim.disposition != dep.disposition
                || claim.profile_bindings != dep.profile_bindings.as_slice()
            {
                return (Decision::rejected("shared_dependency_group_receipt_mismatch"), nominal, Vec::new());
            }
        } else {
            dependence_groups.insert(
                dep.group_id.as_str(),
                DependencyClaim {
                    receipt_id: dep.receipt_id.as_str(),
                    digest: dep.artifact_sha256.as_str(),
                    disposition: dep.disposition,
                    profile_bindings: dep.profile_bindings.as_slice(),
                },
            );
        }
    }

    let mut margins = Vec::new();
    let pm = match case.nominal_case.provider.mechanical.as_ref() {
        Some(v) => v, None => return (Decision::indeterminate("missing_provider_mechanical_profile"), nominal, margins),
    };
    let cm = match case.nominal_case.client.mechanical.as_ref() {
        Some(v) => v, None => return (Decision::indeterminate("missing_client_mechanical_profile"), nominal, margins),
    };

    let mechanical_values = [
        ("mechanical.alignment_capacity", "m", &pm.max_alignment_error, &cm.max_alignment_error, ScalarRule::ProviderMaximumErrorBelowClientTolerance),
        ("mechanical.approach_speed_capacity", "m/s", &pm.max_approach_speed, &cm.max_approach_speed, ScalarRule::ProviderMinimumMeetsClientMinimum),
        ("mechanical.contact_force_capacity", "N", &pm.max_contact_force, &cm.max_contact_force, ScalarRule::ProviderMinimumMeetsClientMinimum),
        ("mechanical.retention_capacity", "N", &pm.min_retention_force, &cm.min_retention_force, ScalarRule::ProviderMinimumMeetsClientMinimum),
    ];
    for (measurand, unit, pv, cv, scalar_rule) in mechanical_values {
        let requirement = Requirement { measurand_id: measurand, unit };
        let p_receipt = match find_receipt(case, &provider_id, measurand) {
            Ok(r) => r, Err(d) => return (d, nominal, margins),
        };
        let c_receipt = match find_receipt(case, &client_id, measurand) {
            Ok(r) => r, Err(d) => return (d, nominal, margins),
        };
        let p_uq = match receipt_bound(p_receipt, &provider_id, &provider_config, &provider_state, &client_id, &client_config, requirement, &case.context_of_use, case.at_tick) {
            Ok(v) => v, Err(d) => return (d, nominal, margins),
        };
        let c_uq = match receipt_bound(c_receipt, &client_id, &client_config, &client_state, &provider_id, &provider_config, requirement, &case.context_of_use, case.at_tick) {
            Ok(v) => v, Err(d) => return (d, nominal, margins),
        };
        let (p, c) = match (get_scalar(pv), get_scalar(cv)) {
            (Ok(p), Ok(c)) => (p, c),
            (Err(d), _) | (_, Err(d)) => return (d, nominal, margins),
        };
        let spec = MetricSpec { profile_id: &provider_id, measurand_id: measurand, unit, scalar_rule };
        if let Err(d) = check_scalar(p, c, p_uq, c_uq, spec, &mut margins) {
            return (d, nominal, margins);
        }
    }

    if case.nominal_case.service == orbital_interface_conformance_bench::ServiceKind::CaptureAndStorableFluid {
        let pf = match case.nominal_case.provider.fluid.as_ref() {
            Some(v) => v, None => return (Decision::indeterminate("missing_provider_fluid_profile"), nominal, margins),
        };
        let cf = match case.nominal_case.client.fluid.as_ref() {
            Some(v) => v, None => return (Decision::indeterminate("missing_client_fluid_profile"), nominal, margins),
        };
        for (measurand, unit, p_interval, c_interval) in [
            ("fluid.temperature_envelope", "K", &pf.temperature, &cf.temperature),
            ("fluid.pressure_envelope", "Pa", &pf.pressure, &cf.pressure),
        ] {
            let requirement = Requirement { measurand_id: measurand, unit };
            let p_receipt = match find_receipt(case, &provider_id, measurand) { Ok(r) => r, Err(d) => return (d, nominal, margins) };
            let c_receipt = match find_receipt(case, &client_id, measurand) { Ok(r) => r, Err(d) => return (d, nominal, margins) };
            let p_uq = match receipt_bound(p_receipt, &provider_id, &provider_config, &provider_state, requirement, &case.context_of_use, case.at_tick) { Ok(v) => v, Err(d) => return (d, nominal, margins) };
            let c_uq = match receipt_bound(c_receipt, &client_id, &client_config, &client_state, requirement, &case.context_of_use, case.at_tick) { Ok(v) => v, Err(d) => return (d, nominal, margins) };
            let (p, c) = match (get_interval(p_interval), get_interval(c_interval)) {
                (Ok(p), Ok(c)) => (p, c),
                (Err(d), _) | (_, Err(d)) => return (d, nominal, margins),
            };
            let spec = MetricSpec { profile_id: &provider_id, measurand_id: measurand, unit };
            if let Err(d) = check_interval(p, c, p_uq, c_uq, spec, &mut margins) {
                return (d, nominal, margins);
            }
        }
        let measurand = "fluid.receiver_accepted_mass";
        let requirement = Requirement { measurand_id: measurand, unit: "kg" };
        let p_receipt = match find_receipt(case, &provider_id, measurand) { Ok(r) => r, Err(d) => return (d, nominal, margins) };
        let c_receipt = match find_receipt(case, &client_id, measurand) { Ok(r) => r, Err(d) => return (d, nominal, margins) };
        let p_uq = match receipt_bound(p_receipt, &provider_id, &provider_config, &provider_state, requirement, &case.context_of_use, case.at_tick) { Ok(v) => v, Err(d) => return (d, nominal, margins) };
        let c_uq = match receipt_bound(c_receipt, &client_id, &client_config, &client_state, requirement, &case.context_of_use, case.at_tick) { Ok(v) => v, Err(d) => return (d, nominal, margins) };
        let (p, c) = match (get_scalar(&pf.min_receiver_accepted_mass), get_scalar(&cf.min_receiver_accepted_mass)) {
            (Ok(p), Ok(c)) => (p, c),
            (Err(d), _) | (_, Err(d)) => return (d, nominal, margins),
        };
        let spec = MetricSpec { profile_id: &provider_id, measurand_id: measurand, unit: "kg", scalar_rule: ScalarRule::ProviderMinimumMeetsClientMinimum };
        if let Err(d) = check_scalar(p, c, p_uq, c_uq, spec, &mut margins) {
            return (d, nominal, margins);
        }
    }

    (Decision::compatible(), nominal, margins)
}

pub fn evaluate(fixture: &RobustFixture) -> Result<Report, String> {
    if fixture.schema != ROBUST_SCHEMA { return Err(format!("unsupported robust fixture schema: {}", fixture.schema)); }
    if fixture.evidence_class != "synthetic" || fixture.claim_ceiling != CLAIM_CEILING {
        return Err("v0.1 robust adapter accepts synthetic-only evidence".to_owned());
    }
    if fixture.cases.is_empty() { return Err("robust fixture has no cases".to_owned()); }

    let mut seen = HashSet::new();
    let mut results = Vec::with_capacity(fixture.cases.len());
    let mut compatible = 0usize;
    let mut rejected = 0usize;
    let mut indeterminate = 0usize;
    let mut mismatches = 0usize;
    let mut false_accepts = 0usize;

    for case in &fixture.cases {
        if !seen.insert(case.case_id.as_str()) {
            return Err(format!("duplicate robust case id: {}", case.case_id));
        }
        let (actual, nominal_status, margins) = evaluate_case(case);
        match actual.status {
            RobustStatus::RobustlyCompatibleSynthetic => compatible += 1,
            RobustStatus::Rejected => rejected += 1,
            RobustStatus::Indeterminate => indeterminate += 1,
        }
        if actual.status == RobustStatus::RobustlyCompatibleSynthetic && case.expected_status != RobustStatus::RobustlyCompatibleSynthetic {
            false_accepts += 1;
        }
        let matched = actual.status == case.expected_status && actual.reason == case.expected_reason;
        if !matched { mismatches += 1; }
        results.push(CaseResult {
            case_id: case.case_id.clone(),
            nominal_status,
            actual_status: actual.status,
            actual_reason: actual.reason.to_owned(),
            expected_status: case.expected_status,
            expected_reason: case.expected_reason.clone(),
            expected_result_matched: matched,
            margins,
        });
    }

    let summary = Summary {
        case_count: fixture.cases.len(),
        robustly_compatible_synthetic: compatible,
        rejected,
        indeterminate,
        expected_result_mismatches: mismatches,
        false_robust_accepts: false_accepts,
        authority_grants: 0,
    };
    Ok(Report {
        result: if mismatches == 0 { "pass" } else { "fail" }.to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        authority_granted: false,
        summary,
        cases: results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> RobustFixture {
        serde_json::from_str(include_str!("../fixtures/v0.1/cases.json"))
            .expect("frozen robust compatibility corpus parses")
    }

    #[test]
    fn frozen_hard_bound_corpus_matches_expected_results() {
        let report = evaluate(&fixture()).expect("robust fixture evaluates");
        assert_eq!(report.result, "pass");
        assert_eq!(report.summary.case_count, 31);
        assert_eq!(report.summary.expected_result_mismatches, 0);
        assert_eq!(report.summary.false_robust_accepts, 0);
        assert_eq!(report.summary.authority_grants, 0);
        assert!(!report.authority_granted);
    }

    #[test]
    fn uncertainty_inflation_never_improves_margin() {
        let fixture = fixture();
        let (_, _, base_margins) = evaluate_case(&fixture.cases[0]);
        let mut inflated = fixture.cases[0].clone();
        inflated.uncertainty_receipts[0].uncertainty_plus += 0.001;
        let (_, _, changed_margins) = evaluate_case(&inflated);
        assert_eq!(base_margins.len(), changed_margins.len());
        assert!(changed_margins[0].margin_lower <= base_margins[0].margin_lower);
    }


    #[test]
    fn alignment_error_uses_opposite_scalar_direction_from_minimum_capacities() {
        let mut case = fixture().cases[0].clone();
        case.nominal_case.provider.mechanical.as_mut().unwrap()
            .max_alignment_error.as_mut().unwrap().value = Some(0.003);
        case.nominal_case.client.mechanical.as_mut().unwrap()
            .max_alignment_error.as_mut().unwrap().value = Some(0.005);
        let p_id = case.nominal_case.provider.profile_id.clone().unwrap();
        let c_id = case.nominal_case.client.profile_id.clone().unwrap();
        let p_uq = case.uncertainty_receipts.iter_mut()
            .find(|r| r.profile_id == p_id && r.measurand_id == "mechanical.alignment_capacity").unwrap();
        p_uq.uncertainty_minus = 0.0;
        p_uq.uncertainty_plus = 0.001;
        let c_uq = case.uncertainty_receipts.iter_mut()
            .find(|r| r.profile_id == c_id && r.measurand_id == "mechanical.alignment_capacity").unwrap();
        c_uq.uncertainty_minus = 0.001;
        c_uq.uncertainty_plus = 0.0;
        let result = evaluate_case(&case).0;
        assert_eq!(result.status, RobustStatus::RobustlyCompatibleSynthetic);
    }

    #[test]
    fn alignment_error_guard_band_rejects_when_provider_worst_case_exceeds_tolerance() {
        let mut case = fixture().cases[0].clone();
        case.nominal_case.provider.mechanical.as_mut().unwrap()
            .max_alignment_error.as_mut().unwrap().value = Some(0.004);
        case.nominal_case.client.mechanical.as_mut().unwrap()
            .max_alignment_error.as_mut().unwrap().value = Some(0.005);
        let p_id = case.nominal_case.provider.profile_id.clone().unwrap();
        let c_id = case.nominal_case.client.profile_id.clone().unwrap();
        let p_uq = case.uncertainty_receipts.iter_mut()
            .find(|r| r.profile_id == p_id && r.measurand_id == "mechanical.alignment_capacity").unwrap();
        p_uq.uncertainty_plus = 0.001;
        let c_uq = case.uncertainty_receipts.iter_mut()
            .find(|r| r.profile_id == c_id && r.measurand_id == "mechanical.alignment_capacity").unwrap();
        c_uq.uncertainty_minus = 0.001;
        let result = evaluate_case(&case).0;
        assert_eq!(result.status, RobustStatus::Rejected);
        assert_eq!(result.reason, "uncertainty_guard_band_exceeds_scalar_margin");
    }

    #[test]
    fn state_change_invalidates_old_profile_receipt() {
        let mut case = fixture().cases[0].clone();
        case.provider_lifecycle_state_id = "post-maintenance-state".to_owned();
        let result = evaluate_case(&case).0;
        assert_eq!(result.status, RobustStatus::Indeterminate);
        assert_eq!(result.reason, "uncertainty_scope_mismatch");
    }

    #[test]
    fn probabilistic_coverage_is_not_a_hard_bound() {
        let mut case = fixture().cases[0].clone();
        case.uncertainty_receipts[0].representation = UqRepresentation::CoverageInterval;
        let result = evaluate_case(&case).0;
        assert_eq!(result.status, RobustStatus::Indeterminate);
        assert_eq!(result.reason, "probabilistic_or_expanded_uncertainty_needs_policy");
    }




    #[test]
    fn no_shared_source_assessment_binds_both_profile_configurations() {
        let data = fixture();
        let case = data.cases.iter()
            .find(|case| case.case_id == "HARD_BOUND_POSITIVE")
            .expect("baseline positive case exists");
        let receipt = case.uncertainty_receipts.iter()
            .find(|receipt| receipt.profile_id == "provider-port-profile-v1"
                && receipt.measurand_id == "mechanical.alignment_capacity")
            .expect("provider alignment receipt exists");
        assert_eq!(receipt.dependence.disposition, DependenceDisposition::NoSharedSourceAssessed);
        assert_eq!(receipt.dependence.profile_bindings.len(), 2);
        let outcome = evaluate_case(case).0;
        assert_eq!(outcome.status, RobustStatus::RobustlyCompatibleSynthetic);
    }

    #[test]
    fn shared_calibration_receipt_can_bind_provider_and_client_profiles() {
        let data = fixture();
        let case = data.cases.iter()
            .find(|case| case.case_id == "SHARED_CALIBRATION_ACROSS_PROVIDER_CLIENT")
            .expect("shared calibration positive case exists");
        let decision = evaluate_case(case).0;
        assert_eq!(decision.status, RobustStatus::RobustlyCompatibleSynthetic);
    }

    #[test]
    fn shared_dependence_receipt_missing_one_profile_binding_abstains() {
        let data = fixture();
        let case = data.cases.iter()
            .find(|case| case.case_id == "SHARED_DEPENDENCE_MISSING_CLIENT_BINDING")
            .expect("shared calibration negative case exists");
        let decision = evaluate_case(case).0;
        assert_eq!(decision.status, RobustStatus::Indeterminate);
        assert_eq!(decision.reason, "dependence_evidence_scope_mismatch");
    }

    #[test]
    fn robust_json_schema_is_parseable_and_versioned() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../schemas/cases.schema.json"))
                .expect("robust corpus schema parses");
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(
            schema["properties"]["schema"]["const"],
            "luminous.orbital-robust-compatibility.corpus.v0.1"
        );
    }

    #[test]
    fn receipt_digest_is_bound_to_fixed_byte_identity() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
