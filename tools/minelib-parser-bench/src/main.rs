use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    env, fs,
    path::{Component, Path},
    process::ExitCode,
};

const VERSION: &str = "0.1.0";
const EXPECTED_FIXTURE_BLOCKS: usize = 5;
const EXPECTED_FIXTURE_EDGES: usize = 3;
type Check<T> = Result<T, String>;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AttributeKind {
    UnsignedInteger,
    SignedInteger,
    FixedDecimal,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AttributeColumn {
    name: String,
    kind: AttributeKind,
    scale: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExpectedCounts {
    block_count: usize,
    precedence_rows: usize,
    precedence_edges: usize,
    objective_sum_scaled: i64,
    expected_block_ids: Vec<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    suite_id: String,
    suite_version: String,
    evidence_class: String,
    claim_ceiling: String,
    format_specification: String,
    format_version: String,
    block_file: String,
    precedence_file: String,
    attribute_columns: Vec<AttributeColumn>,
    benchmark_objective_attribute: String,
    benchmark_objective_scale: u32,
    expected: ExpectedCounts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedBlock {
    id: usize,
    coordinates: [i64; 3],
    attributes: BTreeMap<String, i64>,
}

#[derive(Debug, Clone)]
struct ParseSummary {
    blocks: Vec<ParsedBlock>,
    precedence: BTreeMap<usize, Vec<usize>>,
    precedence_edges: usize,
    objective_sum_scaled: i64,
}

#[derive(Debug, Clone, Serialize)]
struct Receipt {
    result: String,
    suite_id: String,
    suite_version: String,
    parser_version: String,
    source_commit: String,
    manifest_sha256: String,
    block_file_sha256: String,
    precedence_file_sha256: String,
    block_count: usize,
    precedence_rows: usize,
    precedence_edges: usize,
    expected_block_ids: Vec<usize>,
    benchmark_objective_attribute: String,
    benchmark_objective_scale: u32,
    objective_sum_scaled: i64,
    claim_ceiling: String,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_lower_hex(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value == value.to_ascii_lowercase()
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_manifest(manifest: &Manifest) -> Check<()> {
    if manifest.suite_id != "MINE-REFERENCE-PARSER-001"
        || manifest.suite_version != VERSION
        || manifest.evidence_class != "authored_synthetic_fixture"
        || !nonempty(&manifest.claim_ceiling)
        || !nonempty(&manifest.format_version)
        || !(manifest.format_specification.starts_with("https://")
            && !manifest.format_specification.chars().any(char::is_whitespace))
        || manifest.attribute_columns.is_empty()
        || manifest.expected.block_count != EXPECTED_FIXTURE_BLOCKS
        || manifest.expected.precedence_edges != EXPECTED_FIXTURE_EDGES
    {
        return Err("manifest identity, evidence class, source reference, or known-answer contract is invalid".to_owned());
    }
    for file in [&manifest.block_file, &manifest.precedence_file] {
        let path = Path::new(file);
        if path.as_os_str().is_empty()
            || path.is_absolute()
            || path.components().any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err("fixture paths must be non-empty relative paths without traversal".to_owned());
        }
    }

    let mut names = BTreeSet::new();
    for column in &manifest.attribute_columns {
        if !nonempty(&column.name) || !names.insert(column.name.clone()) {
            return Err("attribute column names must be non-empty and unique".to_owned());
        }
        match column.kind {
            AttributeKind::FixedDecimal => {
                if !column.scale.is_some_and(|scale| scale <= 9) {
                    return Err(format!("{}: fixed-decimal scale must be between 0 and 9", column.name));
                }
            }
            AttributeKind::UnsignedInteger | AttributeKind::SignedInteger => {
                if column.scale.is_some() {
                    return Err(format!("{}: integer columns must not declare a decimal scale", column.name));
                }
            }
        }
    }
    let objective_column = manifest
        .attribute_columns
        .iter()
        .find(|column| column.name == manifest.benchmark_objective_attribute)
        .ok_or_else(|| "benchmark objective attribute is absent from block schema".to_owned())?;
    if objective_column.kind == AttributeKind::FixedDecimal {
        if objective_column.scale != Some(manifest.benchmark_objective_scale) {
            return Err("objective scale differs from declared attribute scale".to_owned());
        }
    } else if manifest.benchmark_objective_scale != 0 {
        return Err("integer objective attribute must use scale zero".to_owned());
    }
    Ok(())
}

fn tokens(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('%') {
        return Vec::new();
    }
    trimmed
        .replace(':', " ")
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

fn parse_fixed_decimal(raw: &str, scale: u32) -> Check<i64> {
    if raw.is_empty() || raw.starts_with('+') || raw.contains(['e', 'E']) {
        return Err(format!("invalid fixed-decimal literal: {raw}"));
    }
    let (negative, unsigned) = match raw.strip_prefix('-') {
        Some(value) => (true, value),
        None => (false, raw),
    };
    if unsigned.is_empty() {
        return Err(format!("invalid fixed-decimal literal: {raw}"));
    }
    let mut parts = unsigned.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || (unsigned.contains('.') && fraction.is_empty())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > scale as usize
    {
        return Err(format!("invalid fixed-decimal literal at scale {scale}: {raw}"));
    }

    let factor = 10_i128
        .checked_pow(scale)
        .ok_or_else(|| "fixed-decimal scale overflow".to_owned())?;
    let whole_value = whole
        .parse::<i128>()
        .map_err(|_| format!("fixed-decimal whole part out of range: {raw}"))?;
    let mut fractional_value = if fraction.is_empty() {
        0_i128
    } else {
        fraction
            .parse::<i128>()
            .map_err(|_| format!("fixed-decimal fractional part out of range: {raw}"))?
    };
    for _ in fraction.len()..scale as usize {
        fractional_value = fractional_value
            .checked_mul(10)
            .ok_or_else(|| format!("fixed-decimal literal out of range: {raw}"))?;
    }
    let magnitude = whole_value
        .checked_mul(factor)
        .and_then(|value| value.checked_add(fractional_value))
        .ok_or_else(|| format!("fixed-decimal literal out of range: {raw}"))?;
    let signed_value = if negative { -magnitude } else { magnitude };
    i64::try_from(signed_value).map_err(|_| format!("fixed-decimal literal out of i64 range: {raw}"))
}

fn parse_attribute(raw: &str, column: &AttributeColumn) -> Check<i64> {
    match column.kind {
        AttributeKind::UnsignedInteger => {
            if raw.starts_with('-') || raw.starts_with('+') {
                return Err(format!("{} expects an unsigned integer, got {raw}", column.name));
            }
            let value = raw
                .parse::<u64>()
                .map_err(|_| format!("{} expects an unsigned integer, got {raw}", column.name))?;
            i64::try_from(value).map_err(|_| format!("{} value exceeds supported i64 range", column.name))
        }
        AttributeKind::SignedInteger => raw
            .parse::<i64>()
            .map_err(|_| format!("{} expects a signed integer, got {raw}", column.name)),
        AttributeKind::FixedDecimal => {
            parse_fixed_decimal(raw, column.scale.expect("manifest validates fixed-decimal scales"))
        }
    }
}

fn parse_block_model(input: &str, schema: &[AttributeColumn]) -> Check<Vec<ParsedBlock>> {
    let mut blocks = Vec::new();
    let mut ids = BTreeSet::new();
    let expected_columns = 4 + schema.len();
    for (line_index, line) in input.lines().enumerate() {
        let row = tokens(line);
        if row.is_empty() {
            continue;
        }
        if row.len() != expected_columns {
            return Err(format!(
                "block line {}: expected {expected_columns} columns, got {}",
                line_index + 1,
                row.len()
            ));
        }
        let id = row[0]
            .parse::<usize>()
            .map_err(|_| format!("block line {}: invalid block ID", line_index + 1))?;
        if !ids.insert(id) {
            return Err(format!("block line {}: duplicate block ID {id}", line_index + 1));
        }
        let x = row[1]
            .parse::<i64>()
            .map_err(|_| format!("block line {}: invalid x coordinate", line_index + 1))?;
        let y = row[2]
            .parse::<i64>()
            .map_err(|_| format!("block line {}: invalid y coordinate", line_index + 1))?;
        let z = row[3]
            .parse::<i64>()
            .map_err(|_| format!("block line {}: invalid z coordinate", line_index + 1))?;

        let mut attributes = BTreeMap::new();
        for (column, raw) in schema.iter().zip(row[4..].iter()) {
            let value = parse_attribute(raw, column)
                .map_err(|error| format!("block line {}: {error}", line_index + 1))?;
            attributes.insert(column.name.clone(), value);
        }
        blocks.push(ParsedBlock {
            id,
            coordinates: [x, y, z],
            attributes,
        });
    }
    if blocks.is_empty() {
        return Err("block-model file has no data rows".to_owned());
    }
    blocks.sort_by_key(|block| block.id);
    for (expected_id, block) in blocks.iter().enumerate() {
        if block.id != expected_id {
            return Err(format!(
                "block IDs must be contiguous and zero-based in this conformance profile: expected {expected_id}, got {}",
                block.id
            ));
        }
    }
    Ok(blocks)
}

fn parse_precedence(input: &str, block_ids: &BTreeSet<usize>) -> Check<(BTreeMap<usize, Vec<usize>>, usize)> {
    let mut rows = BTreeMap::new();
    let mut edge_count = 0_usize;
    for (line_index, line) in input.lines().enumerate() {
        let row = tokens(line);
        if row.is_empty() {
            continue;
        }
        if row.len() < 2 {
            return Err(format!("precedence line {}: expected block ID and predecessor count", line_index + 1));
        }
        let block_id = row[0]
            .parse::<usize>()
            .map_err(|_| format!("precedence line {}: invalid block ID", line_index + 1))?;
        if !block_ids.contains(&block_id) {
            return Err(format!("precedence line {}: unknown block ID {block_id}", line_index + 1));
        }
        let predecessor_count = row[1]
            .parse::<usize>()
            .map_err(|_| format!("precedence line {}: invalid predecessor count", line_index + 1))?;
        if row.len() != predecessor_count + 2 {
            return Err(format!(
                "precedence line {}: declares {predecessor_count} predecessors but provides {}",
                line_index + 1,
                row.len().saturating_sub(2)
            ));
        }
        let mut predecessors = Vec::with_capacity(predecessor_count);
        let mut unique = BTreeSet::new();
        for raw in &row[2..] {
            let predecessor = raw
                .parse::<usize>()
                .map_err(|_| format!("precedence line {}: invalid predecessor ID", line_index + 1))?;
            if !block_ids.contains(&predecessor) {
                return Err(format!(
                    "precedence line {}: dangling predecessor ID {predecessor}",
                    line_index + 1
                ));
            }
            if predecessor == block_id {
                return Err(format!("precedence line {}: block cannot precede itself", line_index + 1));
            }
            if !unique.insert(predecessor) {
                return Err(format!(
                    "precedence line {}: duplicate predecessor ID {predecessor}",
                    line_index + 1
                ));
            }
            predecessors.push(predecessor);
        }
        if rows.insert(block_id, predecessors).is_some() {
            return Err(format!("precedence line {}: duplicate row for block {block_id}", line_index + 1));
        }
        edge_count = edge_count
            .checked_add(predecessor_count)
            .ok_or_else(|| "precedence edge count overflow".to_owned())?;
    }
    if rows.keys().copied().collect::<BTreeSet<_>>() != *block_ids {
        return Err("precedence rows must cover every block exactly once".to_owned());
    }

    let mut indegree: BTreeMap<usize, usize> =
        block_ids.iter().map(|id| (*id, 0_usize)).collect();
    let mut successors: BTreeMap<usize, Vec<usize>> =
        block_ids.iter().map(|id| (*id, Vec::new())).collect();
    for (block, predecessors) in &rows {
        *indegree.get_mut(block).expect("row IDs were checked") = predecessors.len();
        for predecessor in predecessors {
            successors
                .get_mut(predecessor)
                .expect("predecessor IDs were checked")
                .push(*block);
        }
    }
    let mut ready: VecDeque<usize> = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
        .collect();
    let mut visited = 0_usize;
    while let Some(id) = ready.pop_front() {
        visited += 1;
        for successor in successors.get(&id).expect("all block IDs have a successor list") {
            let degree = indegree.get_mut(successor).expect("successor IDs are known");
            *degree -= 1;
            if *degree == 0 {
                ready.push_back(*successor);
            }
        }
    }
    if visited != block_ids.len() {
        return Err("precedence graph contains a cycle".to_owned());
    }
    Ok((rows, edge_count))
}

fn analyze(manifest: &Manifest, block_input: &str, precedence_input: &str) -> Check<ParseSummary> {
    validate_manifest(manifest)?;
    let blocks = parse_block_model(block_input, &manifest.attribute_columns)?;
    let block_ids: BTreeSet<usize> = blocks.iter().map(|block| block.id).collect();
    let (precedence, precedence_edges) = parse_precedence(precedence_input, &block_ids)?;
    let objective_sum_scaled = blocks.iter().try_fold(0_i64, |sum, block| {
        let value = block
            .attributes
            .get(&manifest.benchmark_objective_attribute)
            .ok_or_else(|| "objective attribute missing after schema parse".to_owned())?;
        sum.checked_add(*value)
            .ok_or_else(|| "objective attribute sum overflows i64".to_owned())
    })?;

    if blocks.len() != manifest.expected.block_count
        || precedence.len() != manifest.expected.precedence_rows
        || precedence_edges != manifest.expected.precedence_edges
        || objective_sum_scaled != manifest.expected.objective_sum_scaled
        || blocks.iter().map(|block| block.id).collect::<Vec<_>>()
            != manifest.expected.expected_block_ids
    {
        return Err("parsed values do not match frozen known-answer expectations".to_owned());
    }
    Ok(ParseSummary {
        blocks,
        precedence,
        precedence_edges,
        objective_sum_scaled,
    })
}

fn safe_join(base: &Path, relative: &str) -> Check<std::path::PathBuf> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path.components().any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("input path must be a local relative path without traversal".to_owned());
    }
    Ok(base.join(path))
}

fn source_commit_from_args(args: &[String]) -> Check<String> {
    if args.len() != 4 || args[2] != "--source-commit" {
        return Err(
            "usage: minelib-parser-bench <manifest.json> --source-commit <40-hex-sha>".to_owned(),
        );
    }
    let source_commit = args[3].to_ascii_lowercase();
    if !is_lower_hex(&source_commit, 40) {
        return Err("source commit must be a 40-character hexadecimal SHA".to_owned());
    }
    Ok(source_commit)
}

fn run() -> Check<ExitCode> {
    let args: Vec<String> = env::args().collect();
    let source_commit = source_commit_from_args(&args)?;
    let manifest_path = Path::new(&args[1]);
    let manifest_bytes = fs::read(manifest_path)
        .map_err(|error| format!("cannot read manifest {}: {error}", manifest_path.display()))?;
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("invalid manifest JSON: {error}"))?;
    validate_manifest(&manifest)?;
    let base = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let block_path = safe_join(base, &manifest.block_file)?;
    let precedence_path = safe_join(base, &manifest.precedence_file)?;
    let block_bytes = fs::read(&block_path)
        .map_err(|error| format!("cannot read block file {}: {error}", block_path.display()))?;
    let precedence_bytes = fs::read(&precedence_path)
        .map_err(|error| format!("cannot read precedence file {}: {error}", precedence_path.display()))?;
    let block_input = std::str::from_utf8(&block_bytes).map_err(|_| "block file must be UTF-8/ASCII".to_owned())?;
    let precedence_input = std::str::from_utf8(&precedence_bytes).map_err(|_| "precedence file must be UTF-8/ASCII".to_owned())?;
    let summary = analyze(&manifest, block_input, precedence_input)?;

    let receipt = Receipt {
        result: "validated".to_owned(),
        suite_id: manifest.suite_id,
        suite_version: manifest.suite_version,
        parser_version: VERSION.to_owned(),
        source_commit,
        manifest_sha256: sha256(&manifest_bytes),
        block_file_sha256: sha256(&block_bytes),
        precedence_file_sha256: sha256(&precedence_bytes),
        block_count: summary.blocks.len(),
        precedence_rows: summary.precedence.len(),
        precedence_edges: summary.precedence_edges,
        expected_block_ids: summary.blocks.iter().map(|block| block.id).collect(),
        benchmark_objective_attribute: manifest.benchmark_objective_attribute,
        benchmark_objective_scale: manifest.benchmark_objective_scale,
        objective_sum_scaled: summary.objective_sum_scaled,
        claim_ceiling: manifest.claim_ceiling,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize receipt: {error}"))?
    );
    Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        serde_json::from_str(include_str!("../../../docs/tech/minelib-parser-fixtures-v0.1.synthetic.json"))
            .expect("frozen synthetic manifest must deserialize")
    }

    fn block_fixture() -> &'static str {
        include_str!("../../../docs/tech/minelib-parser-fixtures-v0.1.synthetic.block")
    }

    fn precedence_fixture() -> &'static str {
        include_str!("../../../docs/tech/minelib-parser-fixtures-v0.1.synthetic.prec")
    }

    #[test]
    fn fixture_matches_block_edge_and_objective_known_answers() {
        let manifest = manifest();
        let summary = analyze(&manifest, block_fixture(), precedence_fixture())
            .expect("known-answer fixture should parse");
        assert_eq!(EXPECTED_FIXTURE_BLOCKS, summary.blocks.len());
        assert_eq!(EXPECTED_FIXTURE_EDGES, summary.precedence_edges);
        assert_eq!(25_000_050, summary.objective_sum_scaled);
        assert_eq!(vec![0, 1, 2, 3, 4], summary.blocks.iter().map(|block| block.id).collect::<Vec<_>>());
        assert_eq!(1_250, summary.blocks[0].attributes["grade_gpt"]);
        assert_eq!(150_000_00, summary.blocks[0].attributes["block_value_usd"]);
    }

    #[test]
    fn duplicate_block_ids_fail_closed() {
        let mut blocks = block_fixture().to_owned();
        blocks = blocks.replace(
            "4 1 0 1 500 0.800 20000.00",
            "3 1 0 1 500 0.800 20000.00",
        );
        assert!(analyze(&manifest(), &blocks, precedence_fixture()).is_err());
    }

    #[test]
    fn malformed_or_ambiguous_decimal_fails_closed() {
        let blocks = block_fixture().replace("1.250", "1.2500");
        assert!(analyze(&manifest(), &blocks, precedence_fixture()).is_err());
        assert!(parse_fixed_decimal("1e3", 2).is_err());
        assert!(parse_fixed_decimal("1.2.3", 2).is_err());
    }

    #[test]
    fn decimal_parser_uses_exact_scaled_integer_arithmetic() {
        assert_eq!(1250, parse_fixed_decimal("1.25", 3).expect("valid decimal"));
        assert_eq!(-500, parse_fixed_decimal("-5.0", 2).expect("valid decimal"));
        assert_eq!(12, parse_fixed_decimal("0.12", 2).expect("valid decimal"));
        assert!(parse_fixed_decimal("0.123", 2).is_err());
    }

    #[test]
    fn wrong_column_count_fails_closed() {
        let blocks = block_fixture().replace("0 0 0 0 1000 1.250 150000.00", "0 0 0 0 1000 1.250");
        assert!(analyze(&manifest(), &blocks, precedence_fixture()).is_err());
    }

    #[test]
    fn dangling_predecessor_fails_closed() {
        let precedences = precedence_fixture().replace("4 1 3", "4 1 99");
        assert!(analyze(&manifest(), block_fixture(), &precedences).is_err());
    }

    #[test]
    fn duplicate_precedence_rows_fail_closed() {
        let precedences = precedence_fixture().replace("4 1 3", "3 1 0");
        assert!(analyze(&manifest(), block_fixture(), &precedences).is_err());
    }

    #[test]
    fn precedence_cycles_fail_closed() {
        let precedences = precedence_fixture().replace("0 0", "0 1 1");
        assert!(analyze(&manifest(), block_fixture(), &precedences).is_err());
    }

    #[test]
    fn duplicate_predecessor_and_self_edge_fail_closed() {
        let duplicate = precedence_fixture().replace("2 1 1", "2 2 1 1");
        assert!(analyze(&manifest(), block_fixture(), &duplicate).is_err());
        let self_edge = precedence_fixture().replace("3 0", "3 1 3");
        assert!(analyze(&manifest(), block_fixture(), &self_edge).is_err());
    }

    #[test]
    fn manifest_attribute_schema_must_be_unambiguous() {
        let mut manifest = manifest();
        manifest.attribute_columns[1].scale = None;
        assert!(validate_manifest(&manifest).is_err());
        let mut manifest = manifest();
        manifest.attribute_columns[2].name = "grade_gpt".to_owned();
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn unknown_manifest_fields_fail_closed() {
        let raw = include_str!("../../../docs/tech/minelib-parser-fixtures-v0.1.synthetic.json")
            .replacen(
                "\"suite_id\":",
                "\"unreviewed_parser_mode\":true,\"suite_id\":",
                1,
            );
        assert!(serde_json::from_str::<Manifest>(&raw).is_err());
    }

    #[test]
    fn fixture_paths_cannot_escape_manifest_directory() {
        let mut manifest = manifest();
        manifest.block_file = "../other.block".to_owned();
        assert!(validate_manifest(&manifest).is_err());
    }
}
