// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Structural validator only: this register does not establish compliance or safety.
use std::{collections::BTreeSet, env, fs, path::Path, process::ExitCode};

const HEADER: [&str; 20] = [
    "record_id", "document_id", "title", "document_type", "lifecycle_status",
    "edition_or_revision", "lifecycle_date", "frequency_scope", "distance_scope",
    "evidence_domain", "modality_scope", "airborne_applicability", "jurisdiction",
    "source_url", "source_accessed_on", "applicability_rationale", "clause_locator",
    "reviewer", "scope_discrepancy", "claim_ceiling",
];

#[derive(Debug, PartialEq, Eq)]
pub struct Record {
    pub record_id: String,
    pub document_id: String,
    pub lifecycle_status: String,
    pub airborne_applicability: String,
}

fn valid_date(s: &str) -> bool {
    let p: Vec<_> = s.split('-').collect();
    if p.len() != 3 || p[0].len() != 4 || p[1].len() != 2 || p[2].len() != 2 { return false; }
    let (Ok(y), Ok(m), Ok(d)) = (p[0].parse::<u32>(), p[1].parse::<u32>(), p[2].parse::<u32>()) else { return false; };
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let max = match m { 1|3|5|7|8|10|12 => 31, 4|6|9|11 => 30, 2 if leap => 29, 2 => 28, _ => return false };
    (1..=max).contains(&d)
}
fn allowed(value: &str, set: &[&str]) -> bool { set.contains(&value) }

pub fn validate_register(input: &str) -> Result<Vec<Record>, Vec<String>> {
    let normalized = input.replace("\r\n", "\n");
    if normalized.contains('\r') {
        return Err(vec!["bare carriage return is not a valid line ending".into()]);
    }
    let mut errors = Vec::new();
    let mut lines = normalized.split_terminator('\n');
    let Some(header) = lines.next() else { return Err(vec!["register is empty".into()]); };
    if header.split('\t').collect::<Vec<_>>().as_slice() != HEADER.as_slice() {
        return Err(vec![format!("header mismatch: expected exact {}-column v0 header", HEADER.len())]);
    }
    let (mut record_ids, mut document_ids) = (BTreeSet::new(), BTreeSet::new());
    let mut records = Vec::new();

    for (offset, line) in lines.enumerate() {
        let n = offset + 2;
        if line.is_empty() { continue; }
        let f: Vec<_> = line.split('\t').collect();
        if f.len() != HEADER.len() {
            errors.push(format!("line {n}: expected {} fields, found {}", HEADER.len(), f.len())); continue;
        }
        let mut ok = true;
        for (i, value) in f.iter().enumerate() {
            if value.trim().is_empty() || value.trim() != *value {
                errors.push(format!("line {n}: {} must be non-empty and have no edge whitespace", HEADER[i])); ok = false;
            }
        }
        if !ok { continue; }

        let (id, doc, doc_type, lifecycle, lifecycle_date) = (f[0], f[1], f[3], f[4], f[6]);
        let (frequency, evidence, modality, applicability) = (f[7], f[9], f[10], f[11]);
        let (url, accessed, rationale, clause, reviewer, discrepancy, ceiling) =
            (f[13], f[14], f[15], f[16], f[17], f[18], f[19]);

        if !record_ids.insert(id.to_string()) { errors.push(format!("line {n}: duplicate record_id {id}")); ok = false; }
        if !document_ids.insert(doc.to_string()) { errors.push(format!("line {n}: duplicate document_id {doc}")); ok = false; }
        if !url.starts_with("https://") || url.chars().any(char::is_whitespace) {
            errors.push(format!("line {n}: source_url must be HTTPS and contain no whitespace")); ok = false;
        }
        if !valid_date(accessed) { errors.push(format!("line {n}: source_accessed_on must be YYYY-MM-DD")); ok = false; }
        if lifecycle_date != "NOT_PUBLISHED" && lifecycle_date != "UNKNOWN" && !valid_date(lifecycle_date) {
            errors.push(format!("line {n}: lifecycle_date must be YYYY-MM-DD, NOT_PUBLISHED or UNKNOWN")); ok = false;
        }

        let enums: [(&str, &[&str], &str); 6] = [
            (doc_type, &["STANDARD","PAS","TECHNICAL_REPORT","ACTIVE_PAR","REGULATION","FREQUENCY_PLAN","AUTHORITY_GUIDANCE"], "document_type"),
            (lifecycle, &["PUBLISHED","ACTIVE_STANDARD","ACTIVE_PAR","IN_DEVELOPMENT","WITHDRAWN","UNKNOWN"], "lifecycle_status"),
            (evidence, &["EMF_EXPOSURE","RF_EMISSIONS_EMC","RADIATIVE_WPT_STANDARDIZATION","ELECTRICAL_SAFETY","RADIO_SPECTRUM_ALLOCATION","EQUIPMENT_AUTHORIZATION","CIVIL_AVIATION","OTHER"], "evidence_domain"),
            (modality, &["INDUCTIVE_WPT","CAPACITIVE_WPT","RADIATIVE_WPT","RADIO_BEAM_WPT","MULTI_MODAL","UNKNOWN"], "modality_scope"),
            (applicability, &["UNASSESSED","UNKNOWN","IN_SCOPE","CONDITIONAL","OUT_OF_SCOPE"], "airborne_applicability"),
            (discrepancy, &["NONE","SOURCE_INCONSISTENCY"], "scope_discrepancy"),
        ];
        for (value, set, name) in enums {
            if !allowed(value, set) { errors.push(format!("line {n}: unsupported {name} value {value:?}")); ok = false; }
        }

        if doc_type == "ACTIVE_PAR" {
            if lifecycle != "ACTIVE_PAR" || lifecycle_date != "NOT_PUBLISHED" {
                errors.push(format!("line {n}: ACTIVE_PAR must have ACTIVE_PAR lifecycle and NOT_PUBLISHED date")); ok = false;
            }
        } else if lifecycle == "ACTIVE_PAR" {
            errors.push(format!("line {n}: ACTIVE_PAR lifecycle requires ACTIVE_PAR document_type")); ok = false;
        }

        if matches!(applicability, "UNASSESSED"|"UNKNOWN") {
            if clause != "NOT_REVIEWED" || reviewer != "NOT_REVIEWED" {
                errors.push(format!("line {n}: UNASSESSED/UNKNOWN requires NOT_REVIEWED clause and reviewer")); ok = false;
            }
        } else if matches!(applicability, "IN_SCOPE"|"CONDITIONAL"|"OUT_OF_SCOPE")
            && (clause == "NOT_REVIEWED" || reviewer == "NOT_REVIEWED") {
            errors.push(format!("line {n}: assessed applicability requires clause locator and named reviewer")); ok = false;
        }

        let has_title = frequency.contains("TITLE=");
        let has_body = frequency.contains("BODY=");
        if discrepancy == "SOURCE_INCONSISTENCY" && !(has_title && has_body) {
            errors.push(format!("line {n}: SOURCE_INCONSISTENCY requires TITLE= and BODY= claims")); ok = false;
        } else if discrepancy == "NONE" && has_title && has_body {
            errors.push(format!("line {n}: conflicting TITLE=/BODY= claims must be flagged SOURCE_INCONSISTENCY")); ok = false;
        }
        if rationale.is_empty() || ceiling.is_empty() {
            errors.push(format!("line {n}: rationale and claim_ceiling are mandatory")); ok = false;
        }
        if ok {
            records.push(Record { record_id: id.into(), document_id: doc.into(),
                lifecycle_status: lifecycle.into(), airborne_applicability: applicability.into() });
        }
    }
    if records.is_empty() && errors.is_empty() { errors.push("register has no data records".into()); }
    if errors.is_empty() { Ok(records) } else { Err(errors) }
}

fn run(path: &Path) -> Result<(), String> {
    let text = fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let records = validate_register(&text).map_err(|e| e.join("\n"))?;
    let active_par = records.iter().filter(|r| r.lifecycle_status == "ACTIVE_PAR").count();
    let unresolved = records.iter().filter(|r| matches!(r.airborne_applicability.as_str(), "UNASSESSED"|"UNKNOWN")).count();
    println!("CONFORMS: {} records; {active_par} ACTIVE_PAR; {unresolved} airborne applicability unresolved", records.len());
    println!("CLAIM CEILING: metadata consistency only; no compliance or aircraft applicability determination");
    Ok(())
}
fn main() -> ExitCode {
    let path = env::args().nth(1).unwrap_or_else(|| "research/wpt-standards-v0/standards.tsv".into());
    match run(Path::new(&path)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => { eprintln!("NOT_CONFORMS: {e}"); ExitCode::FAILURE }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = include_str!("../../../research/wpt-standards-v0/standards.tsv");

    #[test] fn fixture_is_valid_and_aircraft_scope_is_unassessed() {
        let r = validate_register(FIXTURE).unwrap();
        assert_eq!(r.len(), 9);
        assert!(r.iter().all(|x| matches!(x.airborne_applicability.as_str(), "UNASSESSED"|"UNKNOWN")));
        assert_eq!(r.iter().filter(|x| x.lifecycle_status == "ACTIVE_PAR").count(), 1);
        assert!(r.iter().any(|x| x.document_id == "South Africa National Radio Frequency Plan 2026 (GG 55048; Notice 4044)"));
    }
    #[test] fn published_standard_cannot_be_relabelled_active_par() {
        let bad = FIXTURE.replace("\tSTANDARD\tACTIVE_STANDARD\t", "\tACTIVE_PAR\tACTIVE_PAR\t");
        assert!(validate_register(&bad).is_err());
    }
    #[test] fn source_scope_conflict_cannot_be_silently_erased() {
        let bad = FIXTURE.replace("SOURCE_INCONSISTENCY", "NONE");
        assert!(validate_register(&bad).is_err());
    }
    #[test] fn duplicate_ids_fail_closed() {
        let mut l = FIXTURE.lines(); let h = l.next().unwrap(); let a = l.next().unwrap(); let b = l.next().unwrap();
        let bad = format!("{h}\n{a}\n{b}\n{a}\n");
        assert!(validate_register(&bad).unwrap_err().iter().any(|e| e.contains("duplicate record_id")));
    }
    #[test] fn http_sources_are_rejected() {
        let bad = FIXTURE.replacen("https://standards.ieee.org", "http://standards.ieee.org", 1);
        assert!(validate_register(&bad).is_err());
    }
    #[test] fn assessed_applicability_requires_clause_and_reviewer() {
        let bad = FIXTURE.replacen("\tUNASSESSED\t", "\tIN_SCOPE\t", 1);
        assert!(validate_register(&bad).unwrap_err().iter().any(|e| e.contains("clause locator and named reviewer")));
    }
    #[test] fn unknown_applicability_requires_unreviewed_placeholders() {
        assert!(validate_register(FIXTURE).is_ok());
        let bad_review = FIXTURE.replace(
            "NOT_REVIEWED\tNOT_REVIEWED\tNONE\tAuthority overview metadata only.",
            "SOME_CLAUSE\tNOT_REVIEWED\tNONE\tAuthority overview metadata only.",
        );
        assert!(validate_register(&bad_review).is_err());
    }
    #[test] fn malformed_dates_are_rejected() {
        let bad = FIXTURE.replacen("2026-11-08", "2026-02-30", 1);
        assert!(validate_register(&bad).is_err());
    }
    #[test] fn windows_crlf_fixture_is_normalized_but_bare_cr_is_rejected() {
        let normalized = FIXTURE.replace("\r\n", "\n");
        let crlf = normalized.replace('\n', "\r\n");
        assert!(validate_register(&crlf).is_ok());
        let bare_cr = normalized.replace('\n', "\r");
        assert!(validate_register(&bare_cr).is_err());
    }
    #[test] fn row_width_and_header_are_exact() {
        let bad = FIXTURE.replacen("\tUNASSESSED\t", "\tUNASSESSED\textra\t", 1);
        assert!(validate_register(&bad).unwrap_err().iter().any(|e| e.contains("expected 20 fields")));
        assert!(validate_register(&FIXTURE.replacen("record_id", "record_identifier", 1)).is_err());
    }
}