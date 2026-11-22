//! Exact finite-world decision identifiability for synthetic benchmark controls.
//!
//! This module compares finite probability laws exactly. It deliberately does not
//! infer physical truth from finite samples, authenticate upstream evidence, or
//! authorize real-world interventions.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const CORPUS_SCHEMA: &str = "luminous.orbital-decision-identifiability.corpus.v0.1";
const CLAIM_CEILING: &str = "exact-finite-world-decision-set-algebra-only";
const MAX_CASES: usize = 256;
const MAX_WORLDS: usize = 64;
const MAX_PROBES: usize = 64;
const MAX_OUTCOMES: usize = 64;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Rational {
    pub numerator: u64,
    pub denominator: u64,
}

impl Rational {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.denominator == 0
            || self.numerator > self.denominator
            || self.numerator > MAX_SAFE_INTEGER
            || self.denominator > MAX_SAFE_INTEGER
        {
            return Err(
                "rational values must be in [0,1] with safe nonzero denominators".to_owned(),
            );
        }
        Ok(())
    }

    fn is_zero(&self) -> bool {
        self.numerator == 0
    }

    pub(crate) fn exactly_equals(&self, other: &Self) -> bool {
        u128::from(self.numerator) * u128::from(other.denominator)
            == u128::from(other.numerator) * u128::from(self.denominator)
    }

    pub(crate) fn canonicalized(&self) -> Self {
        let numerator = u128::from(self.numerator);
        let denominator = u128::from(self.denominator);
        let divisor = gcd_u128(numerator, denominator);
        Self {
            numerator: (numerator / divisor) as u64,
            denominator: (denominator / divisor) as u64,
        }
    }

    pub(crate) fn compare_exact(&self, other: &Self) -> std::cmp::Ordering {
        (u128::from(self.numerator) * u128::from(other.denominator))
            .cmp(&(u128::from(other.numerator) * u128::from(self.denominator)))
    }

    pub(crate) fn checked_sub(&self, other: &Self) -> Result<Self, String> {
        let common = gcd_u128(u128::from(self.denominator), u128::from(other.denominator));
        let left_scale = u128::from(other.denominator) / common;
        let right_scale = u128::from(self.denominator) / common;
        let left = u128::from(self.numerator)
            .checked_mul(left_scale)
            .ok_or_else(|| "exact rational subtraction overflowed u128".to_owned())?;
        let right = u128::from(other.numerator)
            .checked_mul(right_scale)
            .ok_or_else(|| "exact rational subtraction overflowed u128".to_owned())?;
        if left < right {
            return Err("exact rational subtraction would be negative".to_owned());
        }
        let denominator = u128::from(self.denominator)
            .checked_mul(left_scale)
            .ok_or_else(|| "exact rational denominator overflowed u128".to_owned())?;
        let numerator = left - right;
        let divisor = gcd_u128(numerator, denominator);
        let reduced_numerator = numerator / divisor;
        let reduced_denominator = denominator / divisor;
        if reduced_numerator > u128::from(MAX_SAFE_INTEGER)
            || reduced_denominator > u128::from(MAX_SAFE_INTEGER)
        {
            return Err(
                "exact rational subtraction exceeds the portable integer envelope".to_owned(),
            );
        }
        Ok(Self {
            numerator: u64::try_from(reduced_numerator)
                .map_err(|_| "exact rational numerator does not fit u64".to_owned())?,
            denominator: u64::try_from(reduced_denominator)
                .map_err(|_| "exact rational denominator does not fit u64".to_owned())?,
        })
    }

    fn checked_add(&self, other: &Self) -> Result<Self, String> {
        let common = gcd_u128(u128::from(self.denominator), u128::from(other.denominator));
        let left_scale = u128::from(other.denominator) / common;
        let right_scale = u128::from(self.denominator) / common;
        let numerator = u128::from(self.numerator)
            .checked_mul(left_scale)
            .and_then(|left| {
                u128::from(other.numerator)
                    .checked_mul(right_scale)
                    .and_then(|right| left.checked_add(right))
            })
            .ok_or_else(|| "exact probability sum overflowed u128".to_owned())?;
        let denominator = u128::from(self.denominator)
            .checked_mul(left_scale)
            .ok_or_else(|| "exact probability denominator overflowed u128".to_owned())?;
        let divisor = gcd_u128(numerator, denominator);
        let reduced_numerator = numerator / divisor;
        let reduced_denominator = denominator / divisor;
        if reduced_numerator > u128::from(MAX_SAFE_INTEGER)
            || reduced_denominator > u128::from(MAX_SAFE_INTEGER)
        {
            return Err("exact probability sum exceeds the portable integer envelope".to_owned());
        }
        Ok(Self {
            numerator: u64::try_from(reduced_numerator)
                .map_err(|_| "exact probability numerator does not fit u64".to_owned())?,
            denominator: u64::try_from(reduced_denominator)
                .map_err(|_| "exact probability denominator does not fit u64".to_owned())?,
        })
    }
}

fn gcd_u128(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProbabilityMass {
    pub outcome: String,
    pub probability: Rational,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProbabilityLaw {
    pub outcomes: Vec<ProbabilityMass>,
}

impl ProbabilityLaw {
    fn validate(&self) -> Result<(), String> {
        if self.outcomes.is_empty() || self.outcomes.len() > MAX_OUTCOMES {
            return Err("probability law must contain between 1 and 64 outcomes".to_owned());
        }
        let mut names = BTreeSet::new();
        let mut total = Rational {
            numerator: 0,
            denominator: 1,
        };
        for mass in &self.outcomes {
            if mass.outcome.trim().is_empty() || !names.insert(mass.outcome.as_str()) {
                return Err("probability-law outcome names must be nonempty and unique".to_owned());
            }
            mass.probability.validate()?;
            total = total.checked_add(&mass.probability)?;
        }
        if !total.exactly_equals(&Rational {
            numerator: 1,
            denominator: 1,
        }) {
            return Err("probability law must sum exactly to one".to_owned());
        }
        Ok(())
    }

    fn exactly_equals(&self, other: &Self) -> bool {
        let left = self
            .outcomes
            .iter()
            .map(|mass| (mass.outcome.as_str(), &mass.probability))
            .collect::<BTreeMap<_, _>>();
        let right = other
            .outcomes
            .iter()
            .map(|mass| (mass.outcome.as_str(), &mass.probability))
            .collect::<BTreeMap<_, _>>();
        let names = left
            .keys()
            .chain(right.keys())
            .copied()
            .collect::<BTreeSet<_>>();
        names.into_iter().all(|name| {
            let zero = Rational {
                numerator: 0,
                denominator: 1,
            };
            match (left.get(name), right.get(name)) {
                (Some(a), Some(b)) => a.exactly_equals(b),
                (Some(a), None) => a.exactly_equals(&zero),
                (None, Some(b)) => b.exactly_equals(&zero),
                (None, None) => true,
            }
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub scope_id: String,
    pub configuration_id: String,
    pub admissibility_status: String,
    pub admissibility_receipt_id: Option<String>,
    pub observation_law: ProbabilityLaw,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct World {
    pub world_id: String,
    pub parameter_signature: BTreeMap<String, String>,
    pub observation_law: ProbabilityLaw,
    pub decision_class: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeCost {
    pub reference_trials: u64,
    pub exposure_micro_units: u64,
    pub elapsed_millis: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Probe {
    pub probe_id: String,
    pub admissibility_status: String,
    pub admissibility_receipt_id: Option<String>,
    pub cost: ProbeCost,
    pub world_outcome_laws: BTreeMap<String, ProbabilityLaw>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionCase {
    pub case_id: String,
    pub evidence: Evidence,
    pub worlds: Vec<World>,
    pub probes: Vec<Probe>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionCorpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<DecisionCase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeRank {
    pub rank: usize,
    pub probe_id: String,
    pub worst_case_decision_class_count: usize,
    pub worst_case_compatible_world_count: usize,
    pub cost: ProbeCost,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeSelection {
    pub status: String,
    pub selected_probe_id: Option<String>,
    pub selected_worst_case_decision_class_count: Option<usize>,
    pub admissible_probe_count: usize,
    pub excluded_probe_ids: Vec<String>,
    pub ranked_admissible_probes: Vec<ProbeRank>,
}

impl ProbeSelection {
    fn not_evaluated() -> Self {
        Self {
            status: "not_evaluated".to_owned(),
            selected_probe_id: None,
            selected_worst_case_decision_class_count: None,
            admissible_probe_count: 0,
            excluded_probe_ids: Vec::new(),
            ranked_admissible_probes: Vec::new(),
        }
    }

    fn not_required() -> Self {
        Self {
            status: "not_required_decision_already_identified".to_owned(),
            ..Self::not_evaluated()
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: String,
    pub evidence_scope_id: String,
    pub compatible_world_ids: Vec<String>,
    pub compatible_world_count: usize,
    pub decision_classes: Vec<String>,
    pub parameter_identified: Option<bool>,
    pub probe_selection: ProbeSelection,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecisionSummary {
    pub case_count: usize,
    pub decision_identified_cases: usize,
    pub parameter_unidentified_decision_identified_cases: usize,
    pub decision_not_identified_cases: usize,
    pub model_incompatible_cases: usize,
    pub evidence_indeterminate_cases: usize,
    pub resolving_probe_selected_cases: usize,
    pub no_admissible_probe_cases: usize,
    pub no_resolving_admissible_probe_cases: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecisionEvaluation {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub summary: DecisionSummary,
    pub cases: Vec<CaseResult>,
    pub physical_qualification_granted: bool,
    pub authority_granted: bool,
}

fn validate_law(law: &ProbabilityLaw) -> Result<(), String> {
    law.validate()
}

fn validate_corpus(corpus: &DecisionCorpus) -> Result<(), String> {
    if corpus.schema != CORPUS_SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
        || corpus.cases.len() > MAX_CASES
    {
        return Err("unsupported decision-identifiability corpus or claim ceiling".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    for case in &corpus.cases {
        if case.case_id.trim().is_empty()
            || !case_ids.insert(case.case_id.as_str())
            || case.evidence.scope_id.trim().is_empty()
            || case.evidence.configuration_id.trim().is_empty()
            || !matches!(
                case.evidence.admissibility_status.as_str(),
                "admissible" | "indeterminate"
            )
            || (case.evidence.admissibility_status == "admissible"
                && case
                    .evidence
                    .admissibility_receipt_id
                    .as_deref()
                    .is_none_or(str::is_empty))
            || case.worlds.is_empty()
            || case.worlds.len() > MAX_WORLDS
            || case.probes.len() > MAX_PROBES
        {
            return Err(
                "case identity, evidence scope, admissibility or fixture size is invalid"
                    .to_owned(),
            );
        }
        validate_law(&case.evidence.observation_law)?;

        let mut world_ids = BTreeSet::new();
        for world in &case.worlds {
            if world.world_id.trim().is_empty()
                || !world_ids.insert(world.world_id.as_str())
                || world.decision_class.trim().is_empty()
                || world.parameter_signature.is_empty()
                || world
                    .parameter_signature
                    .iter()
                    .any(|(key, value)| key.trim().is_empty() || value.trim().is_empty())
            {
                return Err(
                    "world identity, parameter signature and decision class must be explicit"
                        .to_owned(),
                );
            }
            validate_law(&world.observation_law)?;
        }

        let expected_world_ids = world_ids;
        let mut probe_ids = BTreeSet::new();
        for probe in &case.probes {
            if probe.probe_id.trim().is_empty()
                || !probe_ids.insert(probe.probe_id.as_str())
                || !matches!(
                    probe.admissibility_status.as_str(),
                    "admissible" | "inadmissible" | "indeterminate"
                )
                || (probe.admissibility_status == "admissible"
                    && probe
                        .admissibility_receipt_id
                        .as_deref()
                        .is_none_or(str::is_empty))
                || [
                    probe.cost.reference_trials,
                    probe.cost.exposure_micro_units,
                    probe.cost.elapsed_millis,
                ]
                .iter()
                .any(|value| *value > MAX_SAFE_INTEGER)
            {
                return Err("probe identity, receipt, status or cost is invalid".to_owned());
            }
            let supplied_world_ids = probe
                .world_outcome_laws
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            if supplied_world_ids != expected_world_ids {
                return Err(
                    "each probe must declare an outcome law for every candidate world".to_owned(),
                );
            }
            for law in probe.world_outcome_laws.values() {
                validate_law(law)?;
            }
        }
    }
    Ok(())
}

fn probe_rank(probe: &Probe, compatible_worlds: &[&World]) -> Result<ProbeRank, String> {
    let mut outcome_decisions: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut outcome_world_counts: BTreeMap<String, usize> = BTreeMap::new();

    for world in compatible_worlds {
        let law = probe
            .world_outcome_laws
            .get(&world.world_id)
            .ok_or_else(|| "probe is missing a compatible world's outcome law".to_owned())?;
        for mass in &law.outcomes {
            if !mass.probability.is_zero() {
                outcome_decisions
                    .entry(mass.outcome.clone())
                    .or_default()
                    .insert(world.decision_class.clone());
                *outcome_world_counts
                    .entry(mass.outcome.clone())
                    .or_default() += 1;
            }
        }
    }

    let worst_case_decision_class_count = outcome_decisions
        .values()
        .map(BTreeSet::len)
        .max()
        .ok_or_else(|| "probe has no possible outcome under compatible worlds".to_owned())?;
    let worst_case_compatible_world_count =
        outcome_world_counts.values().copied().max().unwrap_or(0);

    Ok(ProbeRank {
        rank: 0,
        probe_id: probe.probe_id.clone(),
        worst_case_decision_class_count,
        worst_case_compatible_world_count,
        cost: probe.cost.clone(),
    })
}

fn select_probe(
    case: &DecisionCase,
    compatible_worlds: &[&World],
) -> Result<ProbeSelection, String> {
    let mut excluded_probe_ids = Vec::new();
    let mut ranked = Vec::new();

    for probe in &case.probes {
        let receipt_present = probe
            .admissibility_receipt_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
        if probe.admissibility_status != "admissible" || !receipt_present {
            excluded_probe_ids.push(probe.probe_id.clone());
            continue;
        }
        ranked.push(probe_rank(probe, compatible_worlds)?);
    }

    ranked.sort_by(|left, right| {
        left.worst_case_decision_class_count
            .cmp(&right.worst_case_decision_class_count)
            .then_with(|| left.cost.reference_trials.cmp(&right.cost.reference_trials))
            .then_with(|| {
                left.cost
                    .exposure_micro_units
                    .cmp(&right.cost.exposure_micro_units)
            })
            .then_with(|| left.cost.elapsed_millis.cmp(&right.cost.elapsed_millis))
            .then_with(|| left.probe_id.cmp(&right.probe_id))
    });
    for (index, rank) in ranked.iter_mut().enumerate() {
        rank.rank = index + 1;
    }

    let Some(best) = ranked.first() else {
        return Ok(ProbeSelection {
            status: "no_admissible_probe".to_owned(),
            selected_probe_id: None,
            selected_worst_case_decision_class_count: None,
            admissible_probe_count: 0,
            excluded_probe_ids,
            ranked_admissible_probes: ranked,
        });
    };

    if best.worst_case_decision_class_count > 1 {
        return Ok(ProbeSelection {
            status: "no_resolving_admissible_probe".to_owned(),
            selected_probe_id: None,
            selected_worst_case_decision_class_count: None,
            admissible_probe_count: ranked.len(),
            excluded_probe_ids,
            ranked_admissible_probes: ranked,
        });
    }

    Ok(ProbeSelection {
        status: "selected_resolving_probe".to_owned(),
        selected_probe_id: Some(best.probe_id.clone()),
        selected_worst_case_decision_class_count: Some(best.worst_case_decision_class_count),
        admissible_probe_count: ranked.len(),
        excluded_probe_ids,
        ranked_admissible_probes: ranked,
    })
}

fn evaluate_case(case: &DecisionCase) -> Result<CaseResult, String> {
    if case.evidence.admissibility_status != "admissible"
        || case
            .evidence
            .admissibility_receipt_id
            .as_deref()
            .is_none_or(str::is_empty)
    {
        return Ok(CaseResult {
            case_id: case.case_id.clone(),
            disposition: "EVIDENCE_INDETERMINATE".to_owned(),
            evidence_scope_id: case.evidence.scope_id.clone(),
            compatible_world_ids: Vec::new(),
            compatible_world_count: 0,
            decision_classes: Vec::new(),
            parameter_identified: None,
            probe_selection: ProbeSelection::not_evaluated(),
        });
    }

    let compatible_worlds = case
        .worlds
        .iter()
        .filter(|world| {
            world
                .observation_law
                .exactly_equals(&case.evidence.observation_law)
        })
        .collect::<Vec<_>>();
    let mut compatible_world_ids = compatible_worlds
        .iter()
        .map(|world| world.world_id.clone())
        .collect::<Vec<_>>();
    compatible_world_ids.sort();

    if compatible_worlds.is_empty() {
        return Ok(CaseResult {
            case_id: case.case_id.clone(),
            disposition: "IDENTIFIED_SET_EMPTY_MODEL_INCOMPATIBLE".to_owned(),
            evidence_scope_id: case.evidence.scope_id.clone(),
            compatible_world_ids,
            compatible_world_count: 0,
            decision_classes: Vec::new(),
            parameter_identified: None,
            probe_selection: ProbeSelection::not_evaluated(),
        });
    }

    let decision_classes = compatible_worlds
        .iter()
        .map(|world| world.decision_class.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let parameter_signatures = compatible_worlds
        .iter()
        .map(|world| world.parameter_signature.clone())
        .collect::<BTreeSet<_>>();
    let parameter_identified = parameter_signatures.len() == 1;

    let (disposition, probe_selection) = if decision_classes.len() > 1 {
        (
            "DECISION_NOT_IDENTIFIED",
            select_probe(case, &compatible_worlds)?,
        )
    } else if !parameter_identified {
        (
            "PARAMETERS_UNIDENTIFIED_DECISION_IDENTIFIED",
            ProbeSelection::not_required(),
        )
    } else {
        ("DECISION_IDENTIFIED", ProbeSelection::not_required())
    };

    Ok(CaseResult {
        case_id: case.case_id.clone(),
        disposition: disposition.to_owned(),
        evidence_scope_id: case.evidence.scope_id.clone(),
        compatible_world_count: compatible_world_ids.len(),
        compatible_world_ids,
        decision_classes,
        parameter_identified: Some(parameter_identified),
        probe_selection,
    })
}

/// Evaluate a finite exact-law corpus. A successful result proves only the
/// algebra and declared control behavior of this bounded synthetic benchmark.
pub fn evaluate(corpus: &DecisionCorpus) -> Result<DecisionEvaluation, String> {
    validate_corpus(corpus)?;
    let cases = corpus
        .cases
        .iter()
        .map(evaluate_case)
        .collect::<Result<Vec<_>, _>>()?;

    let mut summary = DecisionSummary {
        case_count: cases.len(),
        decision_identified_cases: 0,
        parameter_unidentified_decision_identified_cases: 0,
        decision_not_identified_cases: 0,
        model_incompatible_cases: 0,
        evidence_indeterminate_cases: 0,
        resolving_probe_selected_cases: 0,
        no_admissible_probe_cases: 0,
        no_resolving_admissible_probe_cases: 0,
    };
    for case in &cases {
        match case.disposition.as_str() {
            "DECISION_IDENTIFIED" => summary.decision_identified_cases += 1,
            "PARAMETERS_UNIDENTIFIED_DECISION_IDENTIFIED" => {
                summary.parameter_unidentified_decision_identified_cases += 1;
            }
            "DECISION_NOT_IDENTIFIED" => summary.decision_not_identified_cases += 1,
            "IDENTIFIED_SET_EMPTY_MODEL_INCOMPATIBLE" => {
                summary.model_incompatible_cases += 1;
            }
            "EVIDENCE_INDETERMINATE" => summary.evidence_indeterminate_cases += 1,
            _ => return Err("internal error: unrecognized decision disposition".to_owned()),
        }
        match case.probe_selection.status.as_str() {
            "selected_resolving_probe" => summary.resolving_probe_selected_cases += 1,
            "no_admissible_probe" => summary.no_admissible_probe_cases += 1,
            "no_resolving_admissible_probe" => {
                summary.no_resolving_admissible_probe_cases += 1;
            }
            _ => {}
        }
    }

    Ok(DecisionEvaluation {
        result: "pass".to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        summary,
        cases,
        physical_qualification_granted: false,
        authority_granted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> DecisionCorpus {
        let raw = include_str!("../fixtures/v0.1/decision-identifiability-cases.json");
        serde_json::from_str(raw).expect("frozen decision-identifiability corpus must parse")
    }

    fn result<'a>(evaluation: &'a DecisionEvaluation, case_id: &str) -> &'a CaseResult {
        evaluation
            .cases
            .iter()
            .find(|case| case.case_id == case_id)
            .expect("case must be present")
    }

    #[test]
    fn finite_fixture_corpus_validates_and_evaluates() {
        let evaluation = evaluate(&corpus()).expect("exact finite corpus must evaluate");
        assert_eq!(evaluation.result, "pass");
        assert_eq!(evaluation.cases.len(), 12);
        assert!(!evaluation.physical_qualification_granted);
        assert!(!evaluation.authority_granted);
    }

    #[test]
    fn parameter_ambiguity_does_not_force_decision_abstention() {
        let evaluation = evaluate(&corpus()).expect("corpus must evaluate");
        let case = result(&evaluation, "constant-thinning-same-decision");
        assert_eq!(
            case.disposition,
            "PARAMETERS_UNIDENTIFIED_DECISION_IDENTIFIED"
        );
        assert_eq!(case.compatible_world_count, 2);
        assert_eq!(case.decision_classes, vec!["WITHIN_DOMAIN_LIMIT"]);
        assert_eq!(case.parameter_identified, Some(false));
    }

    #[test]
    fn observationally_equivalent_opposite_decisions_remain_ambiguous() {
        let evaluation = evaluate(&corpus()).expect("corpus must evaluate");
        let case = result(&evaluation, "constant-thinning-opposite-decisions");
        assert_eq!(case.disposition, "DECISION_NOT_IDENTIFIED");
        assert_eq!(case.decision_classes, vec!["HOLD", "PROCEED"]);
        assert_eq!(case.probe_selection.status, "selected_resolving_probe");
        assert_eq!(
            case.probe_selection.selected_probe_id.as_deref(),
            Some("independent-reference-probe")
        );
    }

    #[test]
    fn overlapping_probe_support_is_ranked_but_never_selected() {
        let evaluation = evaluate(&corpus()).expect("corpus must evaluate");
        let case = result(&evaluation, "permitted-probe-does-not-resolve");
        assert_eq!(case.disposition, "DECISION_NOT_IDENTIFIED");
        assert_eq!(case.probe_selection.status, "no_resolving_admissible_probe");
        assert_eq!(case.probe_selection.selected_probe_id, None);
        assert_eq!(
            case.probe_selection
                .selected_worst_case_decision_class_count,
            None
        );
        assert_eq!(case.probe_selection.admissible_probe_count, 1);
        assert_eq!(
            case.probe_selection.ranked_admissible_probes[0].probe_id,
            "same-outcome-reference"
        );
        assert_eq!(
            result(&evaluation, "action-realization-confounding")
                .probe_selection
                .selected_probe_id,
            None
        );
        assert_eq!(evaluation.summary.no_resolving_admissible_probe_cases, 2);
    }

    #[test]
    fn unsafe_or_unqualified_probe_is_never_selected() {
        let evaluation = evaluate(&corpus()).expect("corpus must evaluate");
        let case = result(&evaluation, "inadmissible-discriminator");
        assert_eq!(case.disposition, "DECISION_NOT_IDENTIFIED");
        assert_eq!(case.probe_selection.status, "no_admissible_probe");
        assert_eq!(case.probe_selection.selected_probe_id, None);
        assert!(
            case.probe_selection
                .excluded_probe_ids
                .contains(&"unsafe-but-discriminating".to_owned())
        );
        assert!(!evaluation.authority_granted);
    }

    #[test]
    fn empty_compatible_world_set_is_not_normalized_to_nearest_world() {
        let evaluation = evaluate(&corpus()).expect("corpus must evaluate");
        let case = result(&evaluation, "out-of-domain-observation");
        assert_eq!(case.disposition, "IDENTIFIED_SET_EMPTY_MODEL_INCOMPATIBLE");
        assert!(case.compatible_world_ids.is_empty());
        assert_eq!(case.parameter_identified, None);
    }

    #[test]
    fn missing_admissibility_evidence_fails_closed() {
        let evaluation = evaluate(&corpus()).expect("corpus must evaluate");
        let case = result(&evaluation, "evidence-indeterminate");
        assert_eq!(case.disposition, "EVIDENCE_INDETERMINATE");
        assert_eq!(case.probe_selection.status, "not_evaluated");
    }

    #[test]
    fn exact_fraction_equality_ignores_fraction_encoding() {
        let left = ProbabilityLaw {
            outcomes: vec![
                ProbabilityMass {
                    outcome: "hit".to_owned(),
                    probability: Rational {
                        numerator: 1,
                        denominator: 2,
                    },
                },
                ProbabilityMass {
                    outcome: "miss".to_owned(),
                    probability: Rational {
                        numerator: 1,
                        denominator: 2,
                    },
                },
            ],
        };
        let right = ProbabilityLaw {
            outcomes: vec![
                ProbabilityMass {
                    outcome: "miss".to_owned(),
                    probability: Rational {
                        numerator: 2,
                        denominator: 4,
                    },
                },
                ProbabilityMass {
                    outcome: "hit".to_owned(),
                    probability: Rational {
                        numerator: 3,
                        denominator: 6,
                    },
                },
            ],
        };
        assert!(left.exactly_equals(&right));
    }

    #[test]
    fn world_probe_and_outcome_order_does_not_change_canonical_result() {
        let mut input = corpus();
        let before = evaluate(&input).expect("original corpus must evaluate");
        let case_id = "constant-thinning-opposite-decisions";
        let target = input
            .cases
            .iter_mut()
            .find(|case| case.case_id == case_id)
            .expect("permutation case must exist");
        target.worlds.reverse();
        target.probes.reverse();
        target.evidence.observation_law.outcomes.reverse();
        for world in &mut target.worlds {
            world.observation_law.outcomes.reverse();
        }
        for probe in &mut target.probes {
            for law in probe.world_outcome_laws.values_mut() {
                law.outcomes.reverse();
            }
        }

        let after = evaluate(&input).expect("permuted corpus must evaluate");
        let before_value = serde_json::to_value(result(&before, case_id))
            .expect("original case result must serialize");
        let after_value = serde_json::to_value(result(&after, case_id))
            .expect("permuted case result must serialize");
        assert_eq!(before_value, after_value);
    }

    #[test]
    fn malformed_probability_law_is_rejected() {
        let mut invalid = corpus();
        invalid.cases[0].evidence.observation_law.outcomes[0].probability = Rational {
            numerator: 1,
            denominator: 3,
        };
        assert!(evaluate(&invalid).is_err());
    }
}
