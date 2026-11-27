#![deny(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

pub const RECEIPT_SCHEMA: &str = "luminous.spacecraft-domain-split-audit.receipt.v0.1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleRow {
    pub sample_id: String,
    pub target_id: String,
    pub session_id: String,
    pub sequence_id: String,
    pub domain_id: String,
    pub partition: String,
    pub split_group_kind: String,
    pub split_group_id: String,
    pub frame_sha256: String,
    pub image_sha256: String,
    pub translation_kind: String,
    pub translator_sha256: String,
    pub translated_sha256: String,
    pub label_audit_sha256: String,
    pub label_audit_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditReceipt {
    pub disposition: &'static str,
    pub row_count: usize,
    pub split_group_kind: String,
    pub train_count: usize,
    pub calibration_count: usize,
    pub test_count: usize,
    pub missing_partitions: Vec<String>,
    pub duplicate_sample_ids: Vec<String>,
    pub cross_partition_group_leaks: Vec<String>,
    pub cross_partition_frame_digest_leaks: Vec<String>,
    pub cross_partition_image_digest_leaks: Vec<String>,
    pub translation_lineage_unqualified: Vec<String>,
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty() && value != "NONE"
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn partition_index(partition: &str) -> Option<usize> {
    match partition {
        "TRAIN" => Some(0),
        "CALIBRATION" => Some(1),
        "TEST" => Some(2),
        _ => None,
    }
}

fn split_group_value(row: &SampleRow) -> Option<&str> {
    match row.split_group_kind.as_str() {
        "TARGET" => Some(&row.target_id),
        "SESSION" => Some(&row.session_id),
        "SEQUENCE" => Some(&row.sequence_id),
        _ => None,
    }
}

fn add_partition(
    values: &mut BTreeMap<String, BTreeSet<String>>,
    identity: &str,
    partition: &str,
) {
    values
        .entry(identity.to_owned())
        .or_default()
        .insert(partition.to_owned());
}

fn cross_partition_leaks(values: BTreeMap<String, BTreeSet<String>>) -> Vec<String> {
    values
        .into_iter()
        .filter_map(|(identity, partitions)| {
            (partitions.len() > 1).then(|| {
                format!("{identity}:{}", partitions.into_iter().collect::<Vec<_>>().join("|"))
            })
        })
        .collect()
}

pub fn audit_rows(rows: &[SampleRow]) -> Result<AuditReceipt, String> {
    if rows.is_empty() {
        return Err("manifest has no samples".to_owned());
    }

    let split_group_kind = rows[0].split_group_kind.clone();
    if !["TARGET", "SESSION", "SEQUENCE"].contains(&split_group_kind.as_str()) {
        return Err(format!("unsupported split_group_kind: {split_group_kind}"));
    }

    let mut partition_counts = [0_usize; 3];
    let mut sample_counts = BTreeMap::<String, usize>::new();
    let mut group_partitions = BTreeMap::<String, BTreeSet<String>>::new();
    let mut frame_partitions = BTreeMap::<String, BTreeSet<String>>::new();
    let mut image_partitions = BTreeMap::<String, BTreeSet<String>>::new();
    let mut unqualified_translation = BTreeSet::<String>::new();

    for (index, row) in rows.iter().enumerate() {
        for (name, value) in [
            ("sample_id", row.sample_id.as_str()),
            ("target_id", row.target_id.as_str()),
            ("session_id", row.session_id.as_str()),
            ("sequence_id", row.sequence_id.as_str()),
            ("domain_id", row.domain_id.as_str()),
            ("split_group_id", row.split_group_id.as_str()),
        ] {
            if !valid_text(value) {
                return Err(format!("row {} has invalid {name}", index + 2));
            }
        }
        if row.split_group_kind != split_group_kind {
            return Err(format!(
                "row {} changes split_group_kind from {split_group_kind} to {}",
                index + 2,
                row.split_group_kind
            ));
        }
        if split_group_value(row) != Some(row.split_group_id.as_str()) {
            return Err(format!(
                "row {} split_group_id does not equal its declared {} identity",
                index + 2,
                row.split_group_kind
            ));
        }

        let Some(partition) = partition_index(&row.partition) else {
            return Err(format!("row {} has unsupported partition {}", index + 2, row.partition));
        };
        partition_counts[partition] += 1;
        *sample_counts.entry(row.sample_id.clone()).or_default() += 1;

        for (name, digest) in [
            ("frame_sha256", row.frame_sha256.as_str()),
            ("image_sha256", row.image_sha256.as_str()),
        ] {
            if !valid_sha256(digest) {
                return Err(format!("row {} has invalid {name}", index + 2));
            }
        }

        add_partition(&mut group_partitions, &row.split_group_id, &row.partition);
        add_partition(&mut frame_partitions, &row.frame_sha256, &row.partition);
        add_partition(&mut image_partitions, &row.image_sha256, &row.partition);

        match row.translation_kind.as_str() {
            "NONE" => {
                if row.translator_sha256 != "NONE"
                    || row.translated_sha256 != "NONE"
                    || row.label_audit_sha256 != "NONE"
                    || row.label_audit_state != "NONE"
                {
                    return Err(format!(
                        "row {} has translation metadata without IMAGE_TRANSLATED",
                        index + 2
                    ));
                }
            }
            "IMAGE_TRANSLATED" => {
                if !valid_sha256(&row.translator_sha256)
                    || !valid_sha256(&row.translated_sha256)
                    || !valid_sha256(&row.label_audit_sha256)
                {
                    return Err(format!("row {} has incomplete translation lineage", index + 2));
                }
                if row.label_audit_state != "PASS" {
                    unqualified_translation.insert(row.sample_id.clone());
                }
            }
            other => {
                return Err(format!("row {} has unsupported translation_kind {other}", index + 2));
            }
        }
    }

    let missing_partitions = ["TRAIN", "CALIBRATION", "TEST"]
        .iter()
        .enumerate()
        .filter_map(|(index, name)| (partition_counts[index] == 0).then(|| (*name).to_owned()))
        .collect::<Vec<_>>();

    let duplicate_sample_ids = sample_counts
        .into_iter()
        .filter_map(|(sample, count)| (count > 1).then_some(sample))
        .collect::<Vec<_>>();

    let group_leaks = cross_partition_leaks(group_partitions);
    let frame_leaks = cross_partition_leaks(frame_partitions);
    let image_leaks = cross_partition_leaks(image_partitions);
    let translation_issues = unqualified_translation.into_iter().collect::<Vec<_>>();

    let split_leakage = !duplicate_sample_ids.is_empty()
        || !group_leaks.is_empty()
        || !frame_leaks.is_empty()
        || !image_leaks.is_empty();

    let disposition = if !missing_partitions.is_empty() {
        "INCOMPLETE_PARTITION_SET"
    } else if split_leakage {
        "SPLIT_LEAKAGE"
    } else if !translation_issues.is_empty() {
        "BLOCKED_TRANSLATION_LINEAGE"
    } else {
        "PASS_SPLIT_INTEGRITY"
    };

    Ok(AuditReceipt {
        disposition,
        row_count: rows.len(),
        split_group_kind,
        train_count: partition_counts[0],
        calibration_count: partition_counts[1],
        test_count: partition_counts[2],
        missing_partitions,
        duplicate_sample_ids,
        cross_partition_group_leaks: group_leaks,
        cross_partition_frame_digest_leaks: frame_leaks,
        cross_partition_image_digest_leaks: image_leaks,
        translation_lineage_unqualified: translation_issues,
    })
}

#[cfg(test)]
mod tests {
    use super::{SampleRow, audit_rows};

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn row(sample: &str, partition: &str, sequence: &str, frame: char, image: char) -> SampleRow {
        SampleRow {
            sample_id: sample.to_owned(),
            target_id: "target-1".to_owned(),
            session_id: format!("session-{sequence}"),
            sequence_id: sequence.to_owned(),
            domain_id: "lightbox".to_owned(),
            partition: partition.to_owned(),
            split_group_kind: "SEQUENCE".to_owned(),
            split_group_id: sequence.to_owned(),
            frame_sha256: digest(frame),
            image_sha256: digest(image),
            translation_kind: "NONE".to_owned(),
            translator_sha256: "NONE".to_owned(),
            translated_sha256: "NONE".to_owned(),
            label_audit_sha256: "NONE".to_owned(),
            label_audit_state: "NONE".to_owned(),
        }
    }

    fn clean_rows() -> Vec<SampleRow> {
        vec![
            row("train-1", "TRAIN", "seq-train", 'a', 'b'),
            row("cal-1", "CALIBRATION", "seq-cal", 'c', 'd'),
            row("test-1", "TEST", "seq-test", 'e', 'f'),
        ]
    }

    #[test]
    fn clean_group_disjoint_manifest_passes() {
        let receipt = audit_rows(&clean_rows()).expect("valid manifest");
        assert_eq!(receipt.disposition, "PASS_SPLIT_INTEGRITY");
        assert_eq!(receipt.row_count, 3);
        assert!(receipt.missing_partitions.is_empty());
    }

    #[test]
    fn duplicate_sample_identity_fails() {
        let mut rows = clean_rows();
        rows.push(rows[0].clone());
        let receipt = audit_rows(&rows).expect("well-formed manifest");
        assert_eq!(receipt.disposition, "SPLIT_LEAKAGE");
        assert_eq!(receipt.duplicate_sample_ids, ["train-1"]);
    }

    #[test]
    fn same_group_across_partitions_fails() {
        let mut rows = clean_rows();
        rows[2].sequence_id = "seq-cal".to_owned();
        rows[2].split_group_id = "seq-cal".to_owned();
        let receipt = audit_rows(&rows).expect("well-formed manifest");
        assert_eq!(receipt.disposition, "SPLIT_LEAKAGE");
        assert_eq!(
            receipt.cross_partition_group_leaks,
            ["seq-cal:CALIBRATION|TEST"]
        );
    }

    #[test]
    fn frame_digest_reuse_across_partitions_fails() {
        let mut rows = clean_rows();
        rows[2].frame_sha256 = rows[0].frame_sha256.clone();
        let receipt = audit_rows(&rows).expect("well-formed manifest");
        assert_eq!(receipt.disposition, "SPLIT_LEAKAGE");
        assert_eq!(receipt.cross_partition_frame_digest_leaks.len(), 1);
    }

    #[test]
    fn image_digest_reuse_across_partitions_fails() {
        let mut rows = clean_rows();
        rows[2].image_sha256 = rows[0].image_sha256.clone();
        let receipt = audit_rows(&rows).expect("well-formed manifest");
        assert_eq!(receipt.disposition, "SPLIT_LEAKAGE");
        assert_eq!(receipt.cross_partition_image_digest_leaks.len(), 1);
    }

    #[test]
    fn translated_row_requires_digests_and_passed_audit() {
        let mut rows = clean_rows();
        rows[1].translation_kind = "IMAGE_TRANSLATED".to_owned();
        rows[1].translator_sha256 = digest('1');
        rows[1].translated_sha256 = digest('2');
        rows[1].label_audit_sha256 = digest('3');
        rows[1].label_audit_state = "PASS".to_owned();
        let receipt = audit_rows(&rows).expect("complete lineage");
        assert_eq!(receipt.disposition, "PASS_SPLIT_INTEGRITY");
        assert!(receipt.translation_lineage_unqualified.is_empty());
    }

    #[test]
    fn translated_row_without_passed_audit_is_blocked() {
        let mut rows = clean_rows();
        rows[1].translation_kind = "IMAGE_TRANSLATED".to_owned();
        rows[1].translator_sha256 = digest('1');
        rows[1].translated_sha256 = digest('2');
        rows[1].label_audit_sha256 = digest('3');
        rows[1].label_audit_state = "PENDING".to_owned();
        let receipt = audit_rows(&rows).expect("metadata is structurally complete");
        assert_eq!(receipt.disposition, "BLOCKED_TRANSLATION_LINEAGE");
        assert_eq!(receipt.translation_lineage_unqualified, ["cal-1"]);
    }

    #[test]
    fn mixed_split_group_kind_is_invalid() {
        let mut rows = clean_rows();
        rows[1].split_group_kind = "TARGET".to_owned();
        rows[1].split_group_id = rows[1].target_id.clone();
        assert!(audit_rows(&rows).is_err());
    }

    #[test]
    fn split_group_id_must_match_declared_identity() {
        let mut rows = clean_rows();
        rows[1].split_group_id = "wrong".to_owned();
        assert!(audit_rows(&rows).is_err());
    }

    #[test]
    fn missing_partition_is_incomplete_not_a_pass() {
        let mut rows = clean_rows();
        rows.pop();
        let receipt = audit_rows(&rows).expect("well-formed manifest");
        assert_eq!(receipt.disposition, "INCOMPLETE_PARTITION_SET");
        assert_eq!(receipt.missing_partitions, ["TEST"]);
    }

    #[test]
    fn translation_metadata_without_translation_kind_is_invalid() {
        let mut rows = clean_rows();
        rows[1].translator_sha256 = digest('1');
        assert!(audit_rows(&rows).is_err());
    }

    #[test]
    fn empty_manifest_is_invalid() {
        assert!(audit_rows(&[]).is_err());
    }
}
