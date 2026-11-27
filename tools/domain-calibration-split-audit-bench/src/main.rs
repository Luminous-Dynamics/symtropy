use domain_calibration_split_audit_bench::{AuditReceipt, RECEIPT_SCHEMA, SampleRow, audit_rows};
use std::error::Error;
use std::fmt::Write as _;
use std::fs;

const HEADER: &str = "sample_id\ttarget_id\tsession_id\tsequence_id\tdomain_id\tpartition\tsplit_group_kind\tsplit_group_id\tframe_sha256\timage_sha256\ttranslation_kind\ttranslator_sha256\ttranslated_sha256\tlabel_audit_sha256\tlabel_audit_state";

fn json_string(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            other if other.is_control() => output.push('?'),
            other => output.push(other),
        }
    }
    output.push('"');
    output
}

fn json_array(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| json_string(value))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_receipt(receipt: &AuditReceipt, source_commit: &str, corpus_sha256: &str) -> String {
    let mut output = String::new();
    write!(
        output,
        "{{\"schema\":{},\"source_commit\":{},\"corpus_sha256\":{},\"row_count\":{},\"split_group_kind\":{},\"partition_counts\":{{\"TRAIN\":{},\"CALIBRATION\":{},\"TEST\":{}}},\"missing_partitions\":{},\"duplicate_sample_ids\":{},\"cross_partition_group_leaks\":{},\"cross_partition_frame_digest_leaks\":{},\"cross_partition_image_digest_leaks\":{},\"cross_partition_translated_digest_leaks\":{},\"translation_lineage_unqualified\":{},\"disposition\":{},\"dataset_source_bytes_authenticated\":false,\"translation_geometry_verified_here\":false,\"operational_exchangeability_proven\":false,\"flight_qualification\":false}}",
        json_string(RECEIPT_SCHEMA),
        json_string(source_commit),
        json_string(corpus_sha256),
        receipt.row_count,
        json_string(&receipt.split_group_kind),
        receipt.train_count,
        receipt.calibration_count,
        receipt.test_count,
        json_array(&receipt.missing_partitions),
        json_array(&receipt.duplicate_sample_ids),
        json_array(&receipt.cross_partition_group_leaks),
        json_array(&receipt.cross_partition_frame_digest_leaks),
        json_array(&receipt.cross_partition_image_digest_leaks),
        json_array(&receipt.cross_partition_translated_digest_leaks),
        json_array(&receipt.translation_lineage_unqualified),
        json_string(receipt.disposition)
    )
    .expect("writing to String cannot fail");
    output
}

fn parse_rows(contents: &str) -> Result<Vec<SampleRow>, Box<dyn Error>> {
    let mut lines = contents.lines();
    if lines.next() != Some(HEADER) {
        return Err("unexpected TSV header".into());
    }

    let mut rows = Vec::new();
    for (index, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 15 {
            return Err(format!(
                "line {}: expected 15 tab-separated fields, got {}",
                index + 2,
                fields.len()
            )
            .into());
        }
        rows.push(SampleRow {
            sample_id: fields[0].to_owned(),
            target_id: fields[1].to_owned(),
            session_id: fields[2].to_owned(),
            sequence_id: fields[3].to_owned(),
            domain_id: fields[4].to_owned(),
            partition: fields[5].to_owned(),
            split_group_kind: fields[6].to_owned(),
            split_group_id: fields[7].to_owned(),
            frame_sha256: fields[8].to_owned(),
            image_sha256: fields[9].to_owned(),
            translation_kind: fields[10].to_owned(),
            translator_sha256: fields[11].to_owned(),
            translated_sha256: fields[12].to_owned(),
            label_audit_sha256: fields[13].to_owned(),
            label_audit_state: fields[14].to_owned(),
        });
    }
    Ok(rows)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: domain-calibration-split-audit-bench <manifest.tsv> <source-commit> <corpus-sha256>"
                .into(),
        );
    }
    if args[2].trim().is_empty()
        || args[3].len() != 64
        || !args[3].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("invalid source commit or corpus SHA-256 argument".into());
    }

    let contents = fs::read_to_string(&args[1])?;
    let rows = parse_rows(&contents)?;
    let receipt = audit_rows(&rows)?;
    println!("{}", render_receipt(&receipt, &args[2], &args[3]));
    Ok(())
}
