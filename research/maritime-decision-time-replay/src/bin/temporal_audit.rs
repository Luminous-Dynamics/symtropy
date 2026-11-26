use maritime_decision_time_replay::temporal::{
    project_temporal_revision, select_decision_revisions, AdjudicationRevision, AdjudicationState,
    AdjudicationAsOf, TemporalCutoffs, TemporalEvidenceRevision, TemporalError,
};
use std::collections::BTreeMap;
use std::error::Error;

const FIXTURES: &str = include_str!("../../fixtures/temporal-cases.tsv");

#[derive(Clone)]
struct FixtureRow {
    cutoffs: TemporalCutoffs,
    expected_subject: String,
    record: TemporalEvidenceRevision,
    expected_selected: String,
    expected_audit: String,
    expected_error: String,
}

fn parse_i64(value: &str, field: &str) -> Result<i64, Box<dyn Error>> {
    value
        .parse::<i64>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_u32(value: &str, field: &str) -> Result<u32, Box<dyn Error>> {
    value
        .parse::<u32>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_bool(value: &str, field: &str) -> Result<bool, Box<dyn Error>> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("invalid {field}: {value}").into()),
    }
}

fn parse_adjudications(value: &str) -> Result<Vec<AdjudicationRevision>, Box<dyn Error>> {
    if value == "-" {
        return Ok(Vec::new());
    }

    value
        .split(';')
        .map(|item| {
            let fields: Vec<&str> = item.split('@').collect();
            if fields.len() != 3 {
                return Err(format!("invalid adjudication tuple: {item}").into());
            }
            let state = match fields[2] {
                "ACCEPTED" => AdjudicationState::Accepted,
                "REJECTED" => AdjudicationState::Rejected,
                "DISPUTED" => AdjudicationState::Disputed,
                "NOT_EVALUABLE" => AdjudicationState::NotEvaluable,
                other => return Err(format!("invalid adjudication state: {other}").into()),
            };
            Ok(AdjudicationRevision {
                revision: parse_u32(fields[0], "adjudication_revision")?,
                recorded_at: parse_i64(fields[1], "adjudication_recorded_at")?,
                state,
            })
        })
        .collect()
}

fn parse_row(line: &str, line_number: usize) -> Result<(String, FixtureRow), Box<dyn Error>> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() != 22 {
        return Err(format!(
            "fixture line {line_number}: expected 22 fields, got {}",
            fields.len()
        )
        .into());
    }

    Ok((
        fields[0].to_owned(),
        FixtureRow {
            cutoffs: TemporalCutoffs {
                physical_at: parse_i64(fields[1], "physical_at")?,
                decision_at: parse_i64(fields[2], "decision_at")?,
                audit_at: parse_i64(fields[3], "audit_at")?,
            },
            expected_subject: fields[4].to_owned(),
            record: TemporalEvidenceRevision {
                source_id: fields[5].to_owned(),
                event_id: fields[6].to_owned(),
                subject_id: fields[7].to_owned(),
                source_revision: parse_u32(fields[8], "source_revision")?,
                payload_digest: fields[9].to_owned(),
                dependency_cluster: fields[10].to_owned(),
                valid_from: parse_i64(fields[11], "valid_from")?,
                valid_until: parse_i64(fields[12], "valid_until")?,
                published_at: parse_i64(fields[13], "published_at")?,
                retrieved_at: parse_i64(fields[14], "retrieved_at")?,
                rights_clear: parse_bool(fields[15], "rights_clear")?,
                rights_valid_from: parse_i64(fields[16], "rights_valid_from")?,
                rights_valid_until: parse_i64(fields[17], "rights_valid_until")?,
                adjudications: parse_adjudications(fields[18])?,
            },
            expected_selected: fields[19].to_owned(),
            expected_audit: fields[20].to_owned(),
            expected_error: fields[21].to_owned(),
        },
    ))
}

fn error_code(error: &TemporalError) -> &'static str {
    match error {
        TemporalError::ConflictingSourceRevision { .. } => "CONFLICTING_SOURCE_REVISION",
        TemporalError::ConflictingAdjudicationRevision { .. } => {
            "CONFLICTING_ADJUDICATION_REVISION"
        }
        TemporalError::SubjectBindingMismatch { .. } => "SUBJECT_BINDING_MISMATCH",
    }
}

fn state_label(state: AdjudicationAsOf) -> String {
    match state {
        AdjudicationAsOf::NotYetAdjudicated => "NOT_YET".to_owned(),
        AdjudicationAsOf::Recorded(AdjudicationState::Accepted) => "ACCEPTED".to_owned(),
        AdjudicationAsOf::Recorded(AdjudicationState::Rejected) => "REJECTED".to_owned(),
        AdjudicationAsOf::Recorded(AdjudicationState::Disputed) => "DISPUTED".to_owned(),
        AdjudicationAsOf::Recorded(AdjudicationState::NotEvaluable) => "NOT_EVALUABLE".to_owned(),
    }
}

fn audit_group(rows: &[FixtureRow]) -> Result<(String, String, String), Box<dyn Error>> {
    let first = rows.first().ok_or("empty temporal case group")?;
    let records: Vec<TemporalEvidenceRevision> =
        rows.iter().map(|row| row.record.clone()).collect();
    let selected = match select_decision_revisions(
        &records,
        &first.expected_subject,
        first.cutoffs,
    ) {
        Ok(value) => value,
        Err(error) => return Ok(("-".to_owned(), "-".to_owned(), error_code(&error).to_owned())),
    };

    let mut selected_values = Vec::new();
    let mut audit_values = Vec::new();
    for record in selected {
        let projection = match project_temporal_revision(record, first.cutoffs) {
            Ok(value) => value,
            Err(error) => {
                return Ok((
                    "-".to_owned(),
                    "-".to_owned(),
                    error_code(&error).to_owned(),
                ))
            }
        };
        let key = format!(
            "{}:{}:{}",
            record.source_id, record.event_id, record.source_revision
        );
        selected_values.push(key.clone());
        audit_values.push(format!(
            "{}={}@{}",
            key,
            state_label(projection.adjudication_as_of),
            projection.adjudication_revision_as_of.unwrap_or(0)
        ));
    }

    // Avoid an artificial "@0" for NOT_YET: revision zero is not an adjudication.
    for value in &mut audit_values {
        if value.ends_with("=NOT_YET@0") {
            *value = value.trim_end_matches("@0").to_owned();
        }
    }
    Ok((
        if selected_values.is_empty() {
            "-".to_owned()
        } else {
            selected_values.join(",")
        },
        if audit_values.is_empty() {
            "-".to_owned()
        } else {
            audit_values.join(",")
        },
        "-".to_owned(),
    ))
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut groups: BTreeMap<String, Vec<FixtureRow>> = BTreeMap::new();
    for (index, raw_line) in FIXTURES.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("case_id\t") {
            continue;
        }
        let (case_id, row) = parse_row(line, index + 1)?;
        groups.entry(case_id).or_default().push(row);
    }

    if groups.is_empty() {
        return Err("empty temporal fixture corpus".into());
    }

    println!("MDTR_TEMPORAL_RECEIPT\t0.1.0");
    println!("case_id\tselected_revisions\taudit_projection\terror");
    let mut passed = 0_usize;

    for (case_id, rows) in &groups {
        let expected = &rows[0];
        if rows.iter().any(|row| {
            row.cutoffs != expected.cutoffs
                || row.expected_subject != expected.expected_subject
                || row.expected_selected != expected.expected_selected
                || row.expected_audit != expected.expected_audit
                || row.expected_error != expected.expected_error
        }) {
            return Err(format!("{case_id}: group-level expected fields/cutoffs differ").into());
        }

        let (selected, audit, error) = audit_group(rows)?;
        if selected != expected.expected_selected
            || audit != expected.expected_audit
            || error != expected.expected_error
        {
            return Err(format!(
                "{case_id}: expected selected/audit/error {}/{}/{}, got {}/{}/{}",
                expected.expected_selected,
                expected.expected_audit,
                expected.expected_error,
                selected,
                audit,
                error
            )
            .into());
        }

        println!("{case_id}\t{selected}\t{audit}\t{error}");
        passed += 1;
    }

    println!("SUMMARY\tPASS\t{passed}/{passed}");
    Ok(())
}
