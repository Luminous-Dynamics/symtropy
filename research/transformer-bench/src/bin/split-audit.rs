//! Audits data-split integrity for transformer surrogate benchmarks.
//! This checks declared identifiers and partitions; it does not prove that identifiers are truthful,
//! source artifacts are authentic, or measurements are independent in the physical world.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

const HEADER: [&str; 9] = [
    "experiment_id",
    "observation_id",
    "geometry_family",
    "material_id",
    "material_lot_id",
    "manufacturer_id",
    "measurement_campaign_id",
    "split_policy",
    "partition",
];

#[derive(Clone, Debug)]
struct Row {
    experiment_id: String,
    observation_id: String,
    geometry_family: String,
    material_id: String,
    material_lot_id: String,
    manufacturer_id: String,
    measurement_campaign_id: String,
    split_policy: String,
    partition: String,
}

#[derive(Clone, Debug)]
struct Summary {
    experiment_id: String,
    split_policy: String,
    train_rows: usize,
    validation_rows: usize,
    test_rows: usize,
    train_groups: usize,
    test_groups: usize,
    interpretation: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("split-audit: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err(format!(
            "usage: {} <split-ledger.csv> <audit-report.csv>",
            args.first().map(String::as_str).unwrap_or("split-audit")
        ));
    }
    let rows = read_rows(&args[1])?;
    let summaries = audit(&rows)?;
    write_report(&args[2], &summaries)?;
    let held_out = summaries.iter().filter(|summary| summary.split_policy != "RANDOM_ROW").count();
    eprintln!(
        "split audit passed: experiments={}, observations={}, held-out-group evaluations={}; identifier truth and source provenance are not independently authenticated",
        summaries.len(),
        rows.len(),
        held_out
    );
    Ok(())
}

fn nonempty(value: &str, context: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{context}: empty value is not allowed"))
    } else {
        Ok(value.to_string())
    }
}

fn valid_policy(value: &str) -> bool {
    matches!(
        value,
        "RANDOM_ROW"
            | "HELD_OUT_GEOMETRY"
            | "HELD_OUT_MANUFACTURER"
            | "HELD_OUT_MATERIAL_LOT"
            | "HELD_OUT_CAMPAIGN"
    )
}

fn valid_partition(value: &str) -> bool {
    matches!(value, "TRAIN" | "VALIDATION" | "TEST")
}

fn read_rows(path: &str) -> Result<Vec<Row>, String> {
    let content =
        fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let mut lines = content.lines();
    let header = lines
        .next()
        .ok_or_else(|| format!("{path}: missing CSV header"))?
        .trim_end_matches('\r');
    let header_columns: Vec<&str> = header.split(',').collect();
    if header_columns.as_slice() != HEADER.as_slice() {
        return Err(format!(
            "{path}: expected header [{}], got [{}]",
            HEADER.join(","),
            header_columns.join(",")
        ));
    }

    let mut rows = Vec::new();
    for (index, raw_line) in lines.enumerate() {
        let line_number = index + 2;
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            return Err(format!("{path}:{line_number}: blank rows are not allowed"));
        }
        if line.contains('"') {
            return Err(format!(
                "{path}:{line_number}: quoted CSV fields are not supported"
            ));
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != HEADER.len() {
            return Err(format!(
                "{path}:{line_number}: expected {} fields, got {}",
                HEADER.len(),
                fields.len()
            ));
        }
        let row = Row {
            experiment_id: nonempty(fields[0], "experiment_id")?,
            observation_id: nonempty(fields[1], "observation_id")?,
            geometry_family: nonempty(fields[2], "geometry_family")?,
            material_id: nonempty(fields[3], "material_id")?,
            material_lot_id: nonempty(fields[4], "material_lot_id")?,
            manufacturer_id: nonempty(fields[5], "manufacturer_id")?,
            measurement_campaign_id: nonempty(fields[6], "measurement_campaign_id")?,
            split_policy: nonempty(fields[7], "split_policy")?,
            partition: nonempty(fields[8], "partition")?,
        };
        if !valid_policy(&row.split_policy) {
            return Err(format!(
                "{path}:{line_number}: unsupported split_policy {:?}",
                row.split_policy
            ));
        }
        if !valid_partition(&row.partition) {
            return Err(format!(
                "{path}:{line_number}: partition must be TRAIN, VALIDATION, or TEST"
            ));
        }
        rows.push(row);
    }
    if rows.is_empty() {
        return Err(format!("{path}: at least one observation is required"));
    }
    Ok(rows)
}

fn held_out_dimension(policy: &str) -> Option<&'static str> {
    match policy {
        "HELD_OUT_GEOMETRY" => Some("geometry_family"),
        "HELD_OUT_MANUFACTURER" => Some("manufacturer_id"),
        "HELD_OUT_MATERIAL_LOT" => Some("material_lot_id"),
        "HELD_OUT_CAMPAIGN" => Some("measurement_campaign_id"),
        "RANDOM_ROW" => None,
        _ => None,
    }
}

fn group_value<'a>(row: &'a Row, dimension: &str) -> &'a str {
    match dimension {
        "geometry_family" => &row.geometry_family,
        "manufacturer_id" => &row.manufacturer_id,
        "material_lot_id" => &row.material_lot_id,
        "measurement_campaign_id" => &row.measurement_campaign_id,
        _ => unreachable!("dimension comes from held_out_dimension"),
    }
}

fn audit(rows: &[Row]) -> Result<Vec<Summary>, String> {
    let mut experiments: BTreeMap<(String, String), Vec<&Row>> = BTreeMap::new();
    let mut seen_observations: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();

    for row in rows {
        let key = (row.experiment_id.clone(), row.split_policy.clone());
        let ids = seen_observations.entry(key.clone()).or_default();
        if !ids.insert(row.observation_id.clone()) {
            return Err(format!(
                "experiment {:?}, policy {}: duplicate observation_id {:?}; one observation must not appear more than once in a split ledger",
                row.experiment_id, row.split_policy, row.observation_id
            ));
        }
        experiments.entry(key).or_default().push(row);
    }

    let mut summaries = Vec::new();
    for ((experiment_id, split_policy), group_rows) in experiments {
        let mut train = Vec::new();
        let mut validation = Vec::new();
        let mut test = Vec::new();
        for row in &group_rows {
            match row.partition.as_str() {
                "TRAIN" => train.push(*row),
                "VALIDATION" => validation.push(*row),
                "TEST" => test.push(*row),
                _ => unreachable!("partition validated during parsing"),
            }
        }
        if train.is_empty() || validation.is_empty() || test.is_empty() {
            return Err(format!(
                "experiment {:?}, policy {}: TRAIN, VALIDATION, and TEST must all be non-empty",
                experiment_id, split_policy
            ));
        }

        let dimension = held_out_dimension(&split_policy);
        let (train_groups, test_groups, interpretation) = if let Some(dimension) = dimension {
            let training_groups: BTreeSet<&str> = train
                .iter()
                .chain(validation.iter())
                .map(|row| group_value(row, dimension))
                .collect();
            let testing_groups: BTreeSet<&str> =
                test.iter().map(|row| group_value(row, dimension)).collect();
            if let Some(leaked) = testing_groups.intersection(&training_groups).next() {
                return Err(format!(
                    "experiment {:?}, policy {}: held-out {} {:?} appears in TRAIN or VALIDATION; test independence is invalid",
                    experiment_id, split_policy, dimension, leaked
                ));
            }
            (
                training_groups.len(),
                testing_groups.len(),
                format!("HELD_OUT_GROUP_DISJOINT:{dimension}"),
            )
        } else {
            let groups: BTreeSet<&str> = group_rows
                .iter()
                .map(|row| row.geometry_family.as_str())
                .collect();
            (
                groups.len(),
                groups.len(),
                "RANDOM_ROW_BASELINE_ONLY_NOT_TRANSFER_EVIDENCE".to_string(),
            )
        };

        summaries.push(Summary {
            experiment_id,
            split_policy,
            train_rows: train.len(),
            validation_rows: validation.len(),
            test_rows: test.len(),
            train_groups,
            test_groups,
            interpretation,
        });
    }
    Ok(summaries)
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn write_report(path: &str, summaries: &[Summary]) -> Result<(), String> {
    let mut output = String::from(
        "experiment_id,split_policy,train_rows,validation_rows,test_rows,train_groups,test_groups,disjointness,interpretation\n",
    );
    for row in summaries {
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            csv_escape(&row.experiment_id),
            row.split_policy,
            row.train_rows,
            row.validation_rows,
            row.test_rows,
            row.train_groups,
            row.test_groups,
            if row.split_policy == "RANDOM_ROW" { "ROW_PARTITION_ONLY" } else { "PASS" },
            csv_escape(&row.interpretation),
        ));
    }
    fs::write(path, output).map_err(|error| format!("cannot write {path}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER_LINE: &str = "experiment_id,observation_id,geometry_family,material_id,material_lot_id,manufacturer_id,measurement_campaign_id,split_policy,partition";

    fn row(observation_id: &str, geometry: &str, manufacturer: &str, lot: &str, campaign: &str, policy: &str, partition: &str) -> Row {
        Row {
            experiment_id: "exp-1".to_string(),
            observation_id: observation_id.to_string(),
            geometry_family: geometry.to_string(),
            material_id: "GOES-M5".to_string(),
            material_lot_id: lot.to_string(),
            manufacturer_id: manufacturer.to_string(),
            measurement_campaign_id: campaign.to_string(),
            split_policy: policy.to_string(),
            partition: partition.to_string(),
        }
    }

    fn valid_geometry_split() -> Vec<Row> {
        vec![
            row("train-1", "G1", "M1", "L1", "C1", "HELD_OUT_GEOMETRY", "TRAIN"),
            row("train-2", "G2", "M1", "L2", "C2", "HELD_OUT_GEOMETRY", "TRAIN"),
            row("val-1", "G1", "M1", "L1", "C3", "HELD_OUT_GEOMETRY", "VALIDATION"),
            row("test-1", "G3", "M2", "L3", "C4", "HELD_OUT_GEOMETRY", "TEST"),
        ]
    }

    #[test]
    fn accepts_geometry_family_holdout_when_test_family_is_absent_from_train_and_validation() {
        let summaries = audit(&valid_geometry_split()).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].train_rows, 2);
        assert_eq!(summaries[0].validation_rows, 1);
        assert_eq!(summaries[0].test_rows, 1);
        assert_eq!(summaries[0].train_groups, 2);
        assert_eq!(summaries[0].test_groups, 1);
        assert_eq!(summaries[0].interpretation, "HELD_OUT_GROUP_DISJOINT:geometry_family");
    }

    #[test]
    fn rejects_geometry_leakage_even_when_row_counts_are_valid() {
        let mut rows = valid_geometry_split();
        rows[3].geometry_family = "G1".to_string();
        let error = audit(&rows).unwrap_err();
        assert!(error.contains("held-out geometry_family"));
        assert!(error.contains("appears in TRAIN or VALIDATION"));
    }

    #[test]
    fn rejects_each_non_geometry_holdout_dimension_when_it_leaks() {
        for (policy, dimension) in [
            ("HELD_OUT_MANUFACTURER", "manufacturer_id"),
            ("HELD_OUT_MATERIAL_LOT", "material_lot_id"),
            ("HELD_OUT_CAMPAIGN", "measurement_campaign_id"),
        ] {
            let mut rows = vec![
                row("train-1", "G1", "M1", "L1", "C1", policy, "TRAIN"),
                row("val-1", "G2", "M2", "L2", "C2", policy, "VALIDATION"),
                row("test-1", "G3", "M3", "L3", "C3", policy, "TEST"),
            ];
            match policy {
                "HELD_OUT_MANUFACTURER" => rows[2].manufacturer_id = "M1".to_string(),
                "HELD_OUT_MATERIAL_LOT" => rows[2].material_lot_id = "L2".to_string(),
                "HELD_OUT_CAMPAIGN" => rows[2].measurement_campaign_id = "C1".to_string(),
                _ => unreachable!(),
            }
            let error = audit(&rows).unwrap_err();
            assert!(error.contains(dimension), "{policy}: {error}");
        }
    }

    #[test]
    fn random_row_split_is_labeled_as_baseline_not_transfer_evidence() {
        let rows = vec![
            row("train-1", "G1", "M1", "L1", "C1", "RANDOM_ROW", "TRAIN"),
            row("val-1", "G1", "M1", "L1", "C1", "RANDOM_ROW", "VALIDATION"),
            row("test-1", "G1", "M1", "L1", "C1", "RANDOM_ROW", "TEST"),
        ];
        let summary = audit(&rows).unwrap();
        assert_eq!(summary[0].interpretation, "RANDOM_ROW_BASELINE_ONLY_NOT_TRANSFER_EVIDENCE");
    }

    #[test]
    fn rejects_duplicate_observation_id_across_partitions() {
        let rows = vec![
            row("same-id", "G1", "M1", "L1", "C1", "RANDOM_ROW", "TRAIN"),
            row("same-id", "G2", "M2", "L2", "C2", "RANDOM_ROW", "TEST"),
            row("val-id", "G3", "M3", "L3", "C3", "RANDOM_ROW", "VALIDATION"),
        ];
        assert!(audit(&rows).unwrap_err().contains("duplicate observation_id"));
    }

    #[test]
    fn rejects_missing_partition() {
        let rows = vec![
            row("train-1", "G1", "M1", "L1", "C1", "RANDOM_ROW", "TRAIN"),
            row("test-1", "G2", "M2", "L2", "C2", "RANDOM_ROW", "TEST"),
        ];
        assert!(audit(&rows).unwrap_err().contains("must all be non-empty"));
    }

    #[test]
    fn parser_rejects_unknown_policy_and_quoted_fields() {
        let path = env::temp_dir().join("transformer-split-audit-test.csv");
        let path = path.to_string_lossy().into_owned();
        fs::write(
            &path,
            format!("{HEADER_LINE}\nexp-1,o1,G1,GOES,L1,M1,C1,UNDECLARED,TRAIN\n"),
        )
        .unwrap();
        assert!(read_rows(&path).unwrap_err().contains("unsupported split_policy"));
        fs::write(
            &path,
            format!("{HEADER_LINE}\nexp-1,\"o1\",G1,GOES,L1,M1,C1,RANDOM_ROW,TRAIN\n"),
        )
        .unwrap();
        assert!(read_rows(&path).unwrap_err().contains("quoted CSV fields"));
        let _ = fs::remove_file(&path);
    }
}
