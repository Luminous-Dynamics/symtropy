// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic premise-trace sweep. No human or real-world outcome data is imported.

use civdyn_normative_audit::{
    AuditReport, FindingStatus, OutcomeSnapshot, PremiseFinding, ProfileFinding, evaluate_profiles,
    reference_profiles, reference_snapshot,
};

const CSV_COLUMNS: usize = 8;

fn csv_escape(value: &str) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "normative audit CSV schema must remain fixed"
    );
    println!(
        "{}",
        fields
            .iter()
            .map(|field| csv_escape(field))
            .collect::<Vec<_>>()
            .join(",")
    );
}

fn status_name(status: FindingStatus) -> &'static str {
    status.name()
}

fn emit_profile(snapshot: &OutcomeSnapshot, profile: &ProfileFinding) {
    emit(vec![
        "profile".into(),
        snapshot.snapshot_id.clone(),
        profile.profile_id.clone(),
        String::new(),
        status_name(profile.status).into(),
        String::new(),
        format!(
            "{} declared premises evaluated; no cross-profile ranking emitted",
            profile.premises.len()
        ),
        "synthetic_fixture".into(),
    ]);
    for premise in &profile.premises {
        emit_premise(snapshot, &profile.profile_id, premise);
    }
}

fn emit_premise(snapshot: &OutcomeSnapshot, profile_id: &str, premise: &PremiseFinding) {
    emit(vec![
        "premise".into(),
        snapshot.snapshot_id.clone(),
        profile_id.into(),
        premise.premise_id.clone(),
        status_name(premise.status).into(),
        premise.evidence_refs.join("|"),
        premise.explanation.clone(),
        "synthetic_fixture".into(),
    ]);
}

fn main() {
    println!(
        "record_type,snapshot_id,profile_id,premise_id,status,evidence_refs,explanation,source_type"
    );

    let snapshot = reference_snapshot();
    let report: AuditReport = evaluate_profiles(&snapshot, &reference_profiles())
        .expect("synthetic premise profiles and outcome snapshot must validate");
    for profile in &report.profiles {
        emit_profile(&snapshot, profile);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_profiles_do_not_collapse_to_a_universal_score() {
        let snapshot = reference_snapshot();
        let report = evaluate_profiles(&snapshot, &reference_profiles()).unwrap();
        assert_eq!(report.profiles.len(), 4);
        assert!(
            report
                .profiles
                .iter()
                .any(|profile| profile.status == FindingStatus::Supported)
        );
        assert!(
            report
                .profiles
                .iter()
                .any(|profile| profile.status == FindingStatus::Contradicted)
        );
    }

    #[test]
    fn emitted_report_rows_have_a_fixed_schema_width() {
        assert_eq!(CSV_COLUMNS, 8);
    }
}
