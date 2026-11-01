//! Machine-checkable conformance of observed case metadata to a published benchmark protocol.
//!
//! A protocol deviation is recorded, never silently normalized or treated as equivalent.
//! Missing observations remain unknown. Meeting a required minimum while missing a desirable
//! target is distinguished from a mandatory deviation. This module does not authenticate source
//! metadata, determine physical equivalence, or validate solver predictions.

use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolConstraint {
    ExactText {
        expected: String,
        case_sensitive: bool,
    },
    NumericRange {
        minimum: f64,
        maximum: f64,
        absolute_tolerance: f64,
        unit: String,
    },
    MinimumDesirableCount {
        minimum: usize,
        desirable: usize,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolRequirement {
    pub key: String,
    pub description: String,
    pub constraint: ProtocolConstraint,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObservedProtocolValue {
    Text(String),
    Number(f64),
    Count(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolObservation {
    pub key: String,
    /// None means not observed or not supplied. It never means zero or a pass.
    pub value: Option<ObservedProtocolValue>,
    pub source_reference: String,
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolFindingStatus {
    Conforms,
    Deviates,
    MeetsMinimumBelowDesirable,
    Unknown,
}

impl ProtocolFindingStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Conforms => "conforms",
            Self::Deviates => "deviates",
            Self::MeetsMinimumBelowDesirable => "meets_minimum_below_desirable",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolFinding {
    pub key: String,
    pub description: String,
    pub expected: String,
    pub observed: String,
    pub status: ProtocolFindingStatus,
    pub source_reference: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolAuditReport {
    pub protocol_id: String,
    pub task_id: String,
    pub observation_set_id: String,
    pub overall_status: &'static str,
    pub evidence_level: &'static str,
    pub findings: Vec<ProtocolFinding>,
}

pub fn audit_protocol(
    protocol_id: &str,
    task_id: &str,
    observation_set_id: &str,
    requirements: &[ProtocolRequirement],
    observations: &[ProtocolObservation],
) -> Result<ProtocolAuditReport, String> {
    if protocol_id.trim().is_empty()
        || task_id.trim().is_empty()
        || observation_set_id.trim().is_empty()
    {
        return Err("protocol, task, and observation-set identifiers must be non-empty".to_owned());
    }
    if requirements.is_empty() {
        return Err("at least one protocol requirement is required".to_owned());
    }

    let mut requirement_by_key = BTreeMap::new();
    for requirement in requirements {
        validate_key(&requirement.key, "requirement")?;
        if requirement.description.trim().is_empty() {
            return Err(format!("requirement {} has an empty description", requirement.key));
        }
        validate_constraint(&requirement.key, &requirement.constraint)?;
        if requirement_by_key.insert(requirement.key.as_str(), requirement).is_some() {
            return Err(format!("duplicate protocol requirement {}", requirement.key));
        }
    }

    let mut observation_by_key = BTreeMap::new();
    for observation in observations {
        validate_key(&observation.key, "observation")?;
        if !requirement_by_key.contains_key(observation.key.as_str()) {
            return Err(format!("observation {} has no declared protocol requirement", observation.key));
        }
        if observation.value.is_some() && observation.source_reference.trim().is_empty() {
            return Err(format!(
                "observed value {} must have a non-empty source reference",
                observation.key
            ));
        }
        if let Some(ObservedProtocolValue::Number(value)) = observation.value {
            if !value.is_finite() {
                return Err(format!("observed numeric value {} is non-finite", observation.key));
            }
        }
        if observation_by_key.insert(observation.key.as_str(), observation).is_some() {
            return Err(format!("duplicate protocol observation {}", observation.key));
        }
    }

    let mut findings = Vec::with_capacity(requirements.len());
    let mut deviations = 0;
    let mut unknowns = 0;
    let mut desirable_gaps = 0;

    for requirement in requirements {
        let observation = observation_by_key.get(requirement.key.as_str()).copied();
        let finding = evaluate_requirement(requirement, observation)?;
        match finding.status {
            ProtocolFindingStatus::Deviates => deviations += 1,
            ProtocolFindingStatus::Unknown => unknowns += 1,
            ProtocolFindingStatus::MeetsMinimumBelowDesirable => desirable_gaps += 1,
            ProtocolFindingStatus::Conforms => {}
        }
        findings.push(finding);
    }

    let overall_status = match (deviations > 0, unknowns > 0, desirable_gaps > 0) {
        (true, true, _) => "deviations_detected_and_incomplete",
        (true, false, _) => "deviations_detected",
        (false, true, _) => "incomplete",
        (false, false, true) => "minimum_met_desirable_target_not_met",
        (false, false, false) => "conforms_to_declared_protocol",
    };

    Ok(ProtocolAuditReport {
        protocol_id: protocol_id.to_owned(),
        task_id: task_id.to_owned(),
        observation_set_id: observation_set_id.to_owned(),
        overall_status,
        evidence_level: "caller_supplied_metadata_not_source_authenticated_not_physical_validation",
        findings,
    })
}

fn validate_key(key: &str, kind: &str) -> Result<(), String> {
    if key.trim().is_empty()
        || key.contains(',')
        || key.contains('\n')
        || key.contains('\r')
        || key.contains('\t')
    {
        return Err(format!("{kind} key must be non-empty and CSV-safe"));
    }
    Ok(())
}

fn validate_constraint(key: &str, constraint: &ProtocolConstraint) -> Result<(), String> {
    match constraint {
        ProtocolConstraint::ExactText { expected, .. } if expected.trim().is_empty() => {
            Err(format!("requirement {key} has an empty expected text value"))
        }
        ProtocolConstraint::NumericRange {
            minimum,
            maximum,
            absolute_tolerance,
            ..
        } if !minimum.is_finite()
            || !maximum.is_finite()
            || !absolute_tolerance.is_finite()
            || minimum > maximum
            || *absolute_tolerance < 0.0 =>
        {
            Err(format!("requirement {key} has an invalid numeric range or tolerance"))
        }
        ProtocolConstraint::MinimumDesirableCount { minimum, desirable }
            if *minimum == 0 || minimum > desirable =>
        {
            Err(format!("requirement {key} has an invalid minimum/desirable count pair"))
        }
        _ => Ok(()),
    }
}

fn evaluate_requirement(
    requirement: &ProtocolRequirement,
    observation: Option<&ProtocolObservation>,
) -> Result<ProtocolFinding, String> {
    let source_reference = observation
        .map(|item| item.source_reference.clone())
        .unwrap_or_default();
    let note = observation.map(|item| item.note.clone()).unwrap_or_else(|| {
        "no observation supplied; status remains unknown".to_owned()
    });
    let Some(value) = observation.and_then(|item| item.value.as_ref()) else {
        return Ok(ProtocolFinding {
            key: requirement.key.clone(),
            description: requirement.description.clone(),
            expected: expected_display(&requirement.constraint),
            observed: "unknown".to_owned(),
            status: ProtocolFindingStatus::Unknown,
            source_reference,
            note,
        });
    };

    let (expected, observed, status) = match (&requirement.constraint, value) {
        (
            ProtocolConstraint::ExactText {
                expected,
                case_sensitive,
            },
            ObservedProtocolValue::Text(observed),
        ) => {
            let matches = if *case_sensitive {
                observed == expected
            } else {
                observed.eq_ignore_ascii_case(expected)
            };
            (
                expected.clone(),
                observed.clone(),
                if matches {
                    ProtocolFindingStatus::Conforms
                } else {
                    ProtocolFindingStatus::Deviates
                },
            )
        }
        (
            ProtocolConstraint::NumericRange {
                minimum,
                maximum,
                absolute_tolerance,
                unit,
            },
            ObservedProtocolValue::Number(observed),
        ) => {
            let lower = minimum - absolute_tolerance;
            let upper = maximum + absolute_tolerance;
            (
                format!("[{minimum}, {maximum}] {unit} ± tolerance {absolute_tolerance}"),
                format!("{observed} {unit}"),
                if *observed >= lower && *observed <= upper {
                    ProtocolFindingStatus::Conforms
                } else {
                    ProtocolFindingStatus::Deviates
                },
            )
        }
        (
            ProtocolConstraint::MinimumDesirableCount {
                minimum,
                desirable,
            },
            ObservedProtocolValue::Count(observed),
        ) => {
            let status = if observed < minimum {
                ProtocolFindingStatus::Deviates
            } else if observed < desirable {
                ProtocolFindingStatus::MeetsMinimumBelowDesirable
            } else {
                ProtocolFindingStatus::Conforms
            };
            (
                format!("minimum={minimum}; desirable={desirable}"),
                observed.to_string(),
                status,
            )
        }
        _ => {
            return Err(format!(
                "observation type for {} does not match its declared constraint",
                requirement.key
            ));
        }
    };

    Ok(ProtocolFinding {
        key: requirement.key.clone(),
        description: requirement.description.clone(),
        expected,
        observed,
        status,
        source_reference,
        note,
    })
}

fn expected_display(constraint: &ProtocolConstraint) -> String {
    match constraint {
        ProtocolConstraint::ExactText { expected, .. } => expected.clone(),
        ProtocolConstraint::NumericRange {
            minimum,
            maximum,
            absolute_tolerance,
            unit,
        } => format!("[{minimum}, {maximum}] {unit} ± tolerance {absolute_tolerance}"),
        ProtocolConstraint::MinimumDesirableCount {
            minimum,
            desirable,
        } => format!("minimum={minimum}; desirable={desirable}"),
    }
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub const PROTOCOL_CONFORMANCE_CSV_HEADER: &str =
    "protocol_id,task_id,observation_set_id,key,description,expected,observed,status,source_reference,note,evidence_level";

pub fn report_to_csv(report: &ProtocolAuditReport) -> String {
    let mut output = format!("{PROTOCOL_CONFORMANCE_CSV_HEADER}\n");
    for finding in &report.findings {
        let fields = [
            &report.protocol_id,
            &report.task_id,
            &report.observation_set_id,
            &finding.key,
            &finding.description,
            &finding.expected,
            &finding.observed,
            &finding.status.as_str().to_owned(),
            &finding.source_reference,
            &finding.note,
            &report.evidence_level.to_owned(),
        ];
        output.push_str(
            &fields
                .iter()
                .map(|field| csv_escape(field))
                .collect::<Vec<_>>()
                .join(","),
        );
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn requirement(key: &str, constraint: ProtocolConstraint) -> ProtocolRequirement {
        ProtocolRequirement {
            key: key.to_owned(),
            description: format!("requirement {key}"),
            constraint,
        }
    }

    fn observation(
        key: &str,
        value: ObservedProtocolValue,
    ) -> ProtocolObservation {
        ProtocolObservation {
            key: key.to_owned(),
            value: Some(value),
            source_reference: "pinned-metadata-fixture".to_owned(),
            note: "synthetic known-answer observation".to_owned(),
        }
    }

    #[test]
    fn exact_text_and_numeric_ranges_are_audited_without_coercion() {
        let requirements = vec![
            requirement(
                "sidewall",
                ProtocolConstraint::ExactText {
                    expected: "periodic".to_owned(),
                    case_sensitive: false,
                },
            ),
            requirement(
                "mach",
                ProtocolConstraint::NumericRange {
                    minimum: 0.73,
                    maximum: 0.73,
                    absolute_tolerance: 1.0e-9,
                    unit: "1".to_owned(),
                },
            ),
        ];
        let observations = vec![
            observation("sidewall", ObservedProtocolValue::Text("PERIODIC".to_owned())),
            observation("mach", ObservedProtocolValue::Number(0.73)),
        ];
        let report = audit_protocol("p", "t", "o", &requirements, &observations)
            .expect("valid observations");
        assert_eq!(report.overall_status, "conforms_to_declared_protocol");
        assert!(report
            .findings
            .iter()
            .all(|finding| finding.status == ProtocolFindingStatus::Conforms));
    }

    #[test]
    fn symmetric_boundary_condition_is_reported_as_deviation_from_periodic_requirement() {
        let requirements = vec![requirement(
            "sidewall",
            ProtocolConstraint::ExactText {
                expected: "periodic".to_owned(),
                case_sensitive: true,
            },
        )];
        let observations = vec![observation(
            "sidewall",
            ObservedProtocolValue::Text("symmetric".to_owned()),
        )];
        let report = audit_protocol("p", "t", "o", &requirements, &observations)
            .expect("valid metadata");
        assert_eq!(report.findings[0].status, ProtocolFindingStatus::Deviates);
        assert_eq!(report.findings[0].observed, "symmetric");
        assert_eq!(report.findings[0].expected, "periodic");
        assert_eq!(report.overall_status, "deviations_detected");
    }

    #[test]
    fn minimum_grid_count_is_distinct_from_desirable_target() {
        let requirements = vec![requirement(
            "grid_count",
            ProtocolConstraint::MinimumDesirableCount {
                minimum: 4,
                desirable: 6,
            },
        )];
        let observations = vec![observation("grid_count", ObservedProtocolValue::Count(4))];
        let report = audit_protocol("p", "t", "o", &requirements, &observations)
            .expect("valid grid count");
        assert_eq!(
            report.findings[0].status,
            ProtocolFindingStatus::MeetsMinimumBelowDesirable
        );
        assert_eq!(
            report.overall_status,
            "minimum_met_desirable_target_not_met"
        );
    }

    #[test]
    fn missing_value_remains_unknown_not_zero_or_pass() {
        let requirements = vec![requirement(
            "mesh_identity",
            ProtocolConstraint::ExactText {
                expected: "independently_pinned".to_owned(),
                case_sensitive: true,
            },
        )];
        let report = audit_protocol("p", "t", "o", &requirements, &[])
            .expect("missing observation is represented as unknown");
        assert_eq!(report.findings[0].status, ProtocolFindingStatus::Unknown);
        assert_eq!(report.findings[0].observed, "unknown");
        assert_eq!(report.overall_status, "incomplete");
    }

    #[test]
    fn count_below_minimum_is_a_mandatory_deviation() {
        let requirements = vec![requirement(
            "grid_count",
            ProtocolConstraint::MinimumDesirableCount {
                minimum: 4,
                desirable: 6,
            },
        )];
        let observations = vec![observation("grid_count", ObservedProtocolValue::Count(3))];
        let report = audit_protocol("p", "t", "o", &requirements, &observations)
            .expect("count is typed");
        assert_eq!(report.findings[0].status, ProtocolFindingStatus::Deviates);
    }

    #[test]
    fn rejects_duplicate_keys_wrong_types_nonfinite_and_bad_tolerance() {
        let exact = requirement(
            "boundary",
            ProtocolConstraint::ExactText {
                expected: "periodic".to_owned(),
                case_sensitive: true,
            },
        );
        let duplicate = vec![exact.clone(), exact.clone()];
        assert!(audit_protocol("p", "t", "o", &duplicate, &[]).is_err());

        let wrong_type = vec![observation("boundary", ObservedProtocolValue::Count(1))];
        assert!(audit_protocol("p", "t", "o", &[exact], &wrong_type).is_err());

        let nonfinite = vec![observation("mach", ObservedProtocolValue::Number(f64::NAN))];
        let numeric = requirement(
            "mach",
            ProtocolConstraint::NumericRange {
                minimum: 0.73,
                maximum: 0.73,
                absolute_tolerance: 0.0,
                unit: "1".to_owned(),
            },
        );
        assert!(audit_protocol("p", "t", "o", &[numeric.clone()], &nonfinite).is_err());

        let invalid_tolerance = requirement(
            "mach",
            ProtocolConstraint::NumericRange {
                minimum: 0.73,
                maximum: 0.73,
                absolute_tolerance: -1.0,
                unit: "1".to_owned(),
            },
        );
        assert!(audit_protocol("p", "t", "o", &[invalid_tolerance], &[]).is_err());
    }

    #[test]
    fn rejects_unclassified_observations_and_empty_source_lineage() {
        let requirements = vec![requirement(
            "mach",
            ProtocolConstraint::NumericRange {
                minimum: 0.73,
                maximum: 0.73,
                absolute_tolerance: 1.0e-9,
                unit: "1".to_owned(),
            },
        )];
        let mut item = observation("mach", ObservedProtocolValue::Number(0.73));
        item.source_reference.clear();
        assert!(audit_protocol("p", "t", "o", &requirements, &[item]).is_err());
        let extra = vec![observation("unlisted", ObservedProtocolValue::Text("x".to_owned()))];
        assert!(audit_protocol("p", "t", "o", &requirements, &extra).is_err());
    }

    #[test]
    fn output_is_deterministic_and_escapes_csv() {
        let requirements = vec![requirement(
            "boundary",
            ProtocolConstraint::ExactText {
                expected: "periodic".to_owned(),
                case_sensitive: true,
            },
        )];
        let observations = vec![observation(
            "boundary",
            ObservedProtocolValue::Text("symmetric".to_owned()),
        )];
        let report = audit_protocol("p", "t", "o", &requirements, &observations)
            .expect("valid report");
        let first = report_to_csv(&report);
        assert_eq!(first, report_to_csv(&report));
        assert!(first.contains("\"deviates\""));
        assert_eq!(first.lines().count(), 2);
    }
}
