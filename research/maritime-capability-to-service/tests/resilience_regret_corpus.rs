use std::collections::{BTreeMap, BTreeSet};

const MANIFEST: &str = include_str!(concat!(
    "../../../docs/research/maritime-capability-to-service/fixtures/",
    "common-cause-recovery-regret-v0.tsv"
));

#[derive(Debug)]
struct Scenario {
    id: String,
    family: String,
    dependencies: Vec<String>,
    lifecycle_event: String,
    cutoff: String,
    expected: String,
    claim_ceiling: String,
}

fn scenarios(input: &str) -> Vec<Scenario> {
    input
        .lines()
        .enumerate()
        .filter_map(|(line_index, raw)| {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 8, "manifest line {}", line_index + 1);
            assert_eq!(fields[0], "CASE");
            let dependencies = if fields[3] == "NONE" {
                Vec::new()
            } else {
                fields[3].split(';').map(str::to_owned).collect()
            };
            Some(Scenario {
                id: fields[1].to_owned(),
                family: fields[2].to_owned(),
                dependencies,
                lifecycle_event: fields[4].to_owned(),
                cutoff: fields[5].to_owned(),
                expected: fields[6].to_owned(),
                claim_ceiling: fields[7].to_owned(),
            })
        })
        .collect()
}

#[test]
fn twenty_synthetic_scenarios_are_unique_and_have_explicit_claim_ceilings() {
    assert!(MANIFEST.contains("no occurrence probabilities are assigned"));
    assert!(!MANIFEST.to_ascii_lowercase().contains("probability="));

    let rows = scenarios(MANIFEST);
    assert_eq!(rows.len(), 20);
    let mut ids = BTreeSet::new();
    let mut families = BTreeMap::<String, usize>::new();
    for row in &rows {
        assert!(ids.insert(&row.id), "duplicate scenario {}", row.id);
        assert_eq!(row.claim_ceiling, "SYNTHETIC_ONLY");
        assert_eq!(row.cutoff, "T0");
        assert!(!row.expected.is_empty());
        *families.entry(row.family.clone()).or_default() += 1;
    }
    assert!(families.len() >= 8, "scenario families are insufficiently diverse");
}

#[test]
fn common_cause_identity_is_preserved_across_distinct_services_and_scenarios() {
    let rows = scenarios(MANIFEST);
    let mill_users = rows
        .iter()
        .filter(|row| row.dependencies.iter().any(|dependency| dependency == "MATERIAL:MILL-01"))
        .count();
    let calibration_users = rows
        .iter()
        .filter(|row| row.dependencies.iter().any(|dependency| dependency == "LAB:CAL-LAB-01"))
        .count();
    let workforce_users = rows
        .iter()
        .filter(|row| row.dependencies.iter().any(|dependency| dependency == "SKILL:WELDER-POOL-01"))
        .count();

    assert!(mill_users >= 2, "shared mill root must recur across events");
    assert!(calibration_users >= 2, "shared calibration root must recur across services");
    assert!(workforce_users >= 1);

    let transitive = rows
        .iter()
        .find(|row| row.id == "CC19_DIFFERENT_PATH_IDS_SHARED_UPSTREAM")
        .unwrap();
    assert!(transitive.dependencies.contains(&"PATH:SUPPLIER-A".to_owned()));
    assert!(transitive.dependencies.contains(&"PATH:SUPPLIER-B".to_owned()));
    assert!(
        transitive.dependencies.contains(&"UTILITY:SUBSTATION-01".to_owned()),
        "distinct path IDs must not erase the shared upstream root"
    );
}

#[test]
fn lifecycle_and_recovery_edge_cases_keep_service_and_knowledge_bounds() {
    let rows = scenarios(MANIFEST);
    let find = |id: &str| rows.iter().find(|row| row.id == id).unwrap();

    let unknown = find("CC20_DEPENDENCY_INVENTORY_UNKNOWN_REMAINDER");
    assert_eq!(unknown.expected, "INDETERMINATE_NO_NUMERIC_RESULT");
    assert!(unknown.dependencies.contains(&"DEPENDENCY:UNKNOWN-REMAINDER".to_owned()));

    let partial = find("RC16_PARTIAL_RESTORE_THEN_SECOND_RESOURCE_BLOCK");
    assert_eq!(partial.expected, "PARTIAL_SERVICE_RETAIN_DENOMINATOR");

    let authority = find("LC14_AUTHORITY_REVOKED_AFTER_RESERVATION");
    assert_eq!(authority.expected, "NO_EXECUTION_AUTHORITY");
    assert_eq!(authority.lifecycle_event, "EXECUTION_AUTHORITY_REVOKED");

    let late = find("EV18_LATE_EVIDENCE_HISTORICAL_REPLAY");
    assert_eq!(late.expected, "PRESERVE_HISTORICAL_KNOWLEDGE_CUT");
    assert_eq!(late.lifecycle_event, "FIRST_AVAILABLE_AFTER_DECISION_CUTOFF");

    let after_deadline = find("LC15_REPLACEMENT_QUALIFIES_AFTER_DEADLINE");
    assert_eq!(after_deadline.expected, "NO_CURRENT_SERVICE_CREDIT");
}
