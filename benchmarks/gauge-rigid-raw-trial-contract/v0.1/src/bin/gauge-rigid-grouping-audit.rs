//! Evidence-backed GAUGE source mapping and leakage audit.
//!
//! This tool verifies exact source bytes and validates two independently reviewed
//! TSV manifests. It does not infer physical identities, inspect arbitrary JSON
//! schemas, or establish statistical exchangeability.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

const MAPPING_HEADER: &str = "raw_path\traw_sha256\tmetadata_path\tmetadata_sha256\tmapping_status\tevidence_path\tevidence_field\tevidence_value\treviewer\treview_id\tfamily\tsubtask\tmaterial";
const GROUPING_HEADER: &str = "raw_path\traw_sha256\tunit_disposition\texchangeability_unit_id\tgroup_evidence_path\tgroup_evidence_field\tgroup_evidence_value\treviewer\treview_id\tdependence_keys\tsplit";
const SPLITS: [&str; 4] = ["FIT", "CALIBRATION", "TEST", "UNASSIGNED"];
const DEPENDENCE_KINDS: [&str; 7] = [
    "article",
    "session",
    "setup",
    "fixture",
    "camera_calibration",
    "run_block",
    "operator_block",
];

#[derive(Clone, Debug)]
struct Args {
    revision: String,
    raw_root: PathBuf,
    metadata_root: PathBuf,
    mapping_tsv: PathBuf,
    grouping_tsv: PathBuf,
    output: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct Mapping {
    raw_path: String,
    raw_sha: String,
    metadata_path: Option<String>,
    metadata_sha: Option<String>,
    status: String,
    family: String,
    subtask: String,
    material: String,
}

#[derive(Clone, Debug)]
struct Grouping {
    raw_path: String,
    raw_sha: String,
    disposition: String,
    unit_id: Option<String>,
    dependence_keys: Vec<String>,
    split: String,
}

#[derive(Clone, Debug)]
struct Row {
    mapping: Mapping,
    grouping: Grouping,
}

#[derive(Default)]
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl UnionFind {
    fn with_len(len: usize) -> Self {
        Self {
            parent: (0..len).collect(),
            rank: vec![0; len],
        }
    }

    fn find(&mut self, value: usize) -> usize {
        if self.parent[value] != value {
            let root = self.find(self.parent[value]);
            self.parent[value] = root;
        }
        self.parent[value]
    }

    fn union(&mut self, left: usize, right: usize) {
        let mut a = self.find(left);
        let mut b = self.find(right);
        if a == b {
            return;
        }
        if self.rank[a] < self.rank[b] {
            std::mem::swap(&mut a, &mut b);
        }
        self.parent[b] = a;
        if self.rank[a] == self.rank[b] {
            self.rank[a] = self.rank[a].saturating_add(1);
        }
    }
}

fn usage() -> &'static str {
    "Usage: gauge-rigid-grouping-audit --dataset-revision <40-hex> --raw-root <data/rigid> --metadata-root <metadata/rigid> --mapping-tsv <mapping.tsv> --grouping-tsv <grouping.tsv> [--output <receipt.jsonl>]"
}

fn parse_args() -> Result<Option<Args>, String> {
    let mut revision = None;
    let mut raw_root = None;
    let mut metadata_root = None;
    let mut mapping_tsv = None;
    let mut grouping_tsv = None;
    let mut output = None;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("{}", usage());
                return Ok(None);
            }
            "--dataset-revision" => revision = Some(next_value(&mut args, &arg)?),
            "--raw-root" => raw_root = Some(PathBuf::from(next_value(&mut args, &arg)?)),
            "--metadata-root" => {
                metadata_root = Some(PathBuf::from(next_value(&mut args, &arg)?))
            }
            "--mapping-tsv" => mapping_tsv = Some(PathBuf::from(next_value(&mut args, &arg)?)),
            "--grouping-tsv" => grouping_tsv = Some(PathBuf::from(next_value(&mut args, &arg)?)),
            "--output" => output = Some(PathBuf::from(next_value(&mut args, &arg)?)),
            other => return Err(format!("unknown argument: {other}\n{}", usage())),
        }
    }

    let revision = revision.ok_or_else(|| format!("--dataset-revision is required\n{}", usage()))?;
    if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("--dataset-revision must be an immutable 40-character hexadecimal commit".to_owned());
    }

    Ok(Some(Args {
        revision,
        raw_root: raw_root.ok_or_else(|| format!("--raw-root is required\n{}", usage()))?,
        metadata_root: metadata_root
            .ok_or_else(|| format!("--metadata-root is required\n{}", usage()))?,
        mapping_tsv: mapping_tsv.ok_or_else(|| format!("--mapping-tsv is required\n{}", usage()))?,
        grouping_tsv: grouping_tsv
            .ok_or_else(|| format!("--grouping-tsv is required\n{}", usage()))?,
        output,
    }))
}

fn next_value(args: &mut impl Iterator<Item = String>, name: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{name} requires a value"))
}

fn canonical_root(path: &Path) -> Result<PathBuf, String> {
    let root = fs::canonicalize(path)
        .map_err(|error| format!("cannot resolve root {}: {error}", path.display()))?;
    if !root.is_dir() || root.file_name().and_then(|value| value.to_str()) != Some("rigid") {
        return Err(format!(
            "root {} must be an existing directory whose final component is rigid",
            root.display()
        ));
    }
    Ok(root)
}

fn relative_path_is_safe(value: &str) -> bool {
    if value.is_empty() || value.contains('\\') || value.contains('\0') {
        return false;
    }
    let path = Path::new(value);
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn walk_json(root: &Path, directory: &Path, files: &mut Vec<String>) -> Result<(), String> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
        .map(|entry| entry.map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by(|left, right| {
        left.file_name()
            .to_string_lossy()
            .as_bytes()
            .cmp(right.file_name().to_string_lossy().as_bytes())
    });

    for entry in entries {
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
        if kind.is_symlink() {
            return Err(format!("symlink is prohibited inside source root: {}", path.display()));
        }
        if kind.is_dir() {
            walk_json(root, &path, files)?;
        } else if kind.is_file()
            && path.extension().and_then(|value| value.to_str()) == Some("json")
        {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .to_str()
                .ok_or_else(|| format!("non-UTF-8 source path: {}", path.display()))?
                .replace('\\', "/");
            files.push(relative);
        }
    }
    Ok(())
}

fn enumerate_json(root: &Path) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    walk_json(root, root, &mut files)?;
    files.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    Ok(files)
}

fn resolve_source(root: &Path, relative: &str) -> Result<PathBuf, String> {
    if !relative_path_is_safe(relative) {
        return Err(format!("unsafe root-relative path: {relative:?}"));
    }
    let path = root.join(relative);
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("cannot inspect source {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("source is not a regular file: {}", path.display()));
    }
    Ok(path)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut input =
        File::open(path).map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let mut child = Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot start sha256sum: {error}"))?;
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "sha256sum stdin unavailable".to_owned())?;
        io::copy(&mut input, &mut stdin).map_err(|error| format!("cannot hash source: {error}"))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("cannot wait for sha256sum: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "sha256sum failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stdout = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    let digest = stdout
        .split_whitespace()
        .next()
        .ok_or_else(|| "sha256sum returned no digest".to_owned())?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("sha256sum returned a malformed digest".to_owned());
    }
    Ok(digest.to_ascii_lowercase())
}

fn valid_sha(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn read_tsv(path: &Path, expected_header: &str, columns: usize) -> Result<Vec<Vec<String>>, String> {
    let file = File::open(path)
        .map_err(|error| format!("cannot open manifest {}: {error}", path.display()))?;
    let mut lines = BufReader::new(file).lines();
    let header = lines
        .next()
        .ok_or_else(|| format!("manifest {} is empty", path.display()))?
        .map_err(|error| error.to_string())?;
    if header != expected_header {
        return Err(format!(
            "manifest {} has an unexpected header; expected exactly:\n{}",
            path.display(),
            expected_header
        ));
    }

    let mut rows = Vec::new();
    for (index, line) in lines.enumerate() {
        let line_number = index + 2;
        let line = line.map_err(|error| format!("{}:{line_number}: {error}", path.display()))?;
        if line.is_empty() {
            return Err(format!("{}:{line_number}: blank rows are prohibited", path.display()));
        }
        let fields = line.split('\t').map(str::to_owned).collect::<Vec<_>>();
        if fields.len() != columns {
            return Err(format!(
                "{}:{line_number}: expected {columns} tab-separated fields, got {}",
                path.display(),
                fields.len()
            ));
        }
        if fields.iter().any(|field| field.trim() != field || field.contains('\r')) {
            return Err(format!(
                "{}:{line_number}: fields must not have leading/trailing whitespace or CR",
                path.display()
            ));
        }
        rows.push(fields);
    }
    Ok(rows)
}

fn parse_mappings(
    rows: Vec<Vec<String>>,
    raw_root: &Path,
    metadata_root: &Path,
) -> Result<Vec<Mapping>, String> {
    let mut mappings = Vec::with_capacity(rows.len());
    let mut seen = BTreeSet::new();
    let mut metadata_digests = HashMap::<String, String>::new();

    for (index, fields) in rows.into_iter().enumerate() {
        let line = index + 2;
        let raw_path = fields[0].clone();
        let raw_sha = fields[1].to_ascii_lowercase();
        let metadata_path = (!fields[2].is_empty()).then(|| fields[2].clone());
        let metadata_sha = (!fields[3].is_empty()).then(|| fields[3].to_ascii_lowercase());
        let status = fields[4].as_str();

        if !seen.insert(raw_path.clone()) {
            return Err(format!("mapping.tsv:{line}: duplicate raw_path {raw_path}"));
        }
        if !valid_sha(&raw_sha) {
            return Err(format!("mapping.tsv:{line}: malformed raw SHA-256"));
        }
        let raw_file = resolve_source(raw_root, &raw_path)?;
        if sha256_file(&raw_file)? != raw_sha {
            return Err(format!("mapping.tsv:{line}: raw digest mismatch for {raw_path}"));
        }

        match status {
            "EXPLICIT_SOURCE_MAPPING" | "REVIEWED_MAPPING" => {
                let metadata_path = metadata_path
                    .as_deref()
                    .ok_or_else(|| format!("mapping.tsv:{line}: mapped row lacks metadata_path"))?;
                let metadata_sha = metadata_sha
                    .as_deref()
                    .ok_or_else(|| format!("mapping.tsv:{line}: mapped row lacks metadata_sha256"))?;
                if !valid_sha(metadata_sha) {
                    return Err(format!("mapping.tsv:{line}: malformed metadata SHA-256"));
                }
                if fields[5].is_empty() || fields[6].is_empty() || fields[7].is_empty() {
                    return Err(format!(
                        "mapping.tsv:{line}: mapped row requires evidence_path, evidence_field and evidence_value"
                    ));
                }
                if fields[5] != metadata_path {
                    return Err(format!(
                        "mapping.tsv:{line}: evidence_path must identify the exact mapped metadata file"
                    ));
                }
                if status == "REVIEWED_MAPPING" && (fields[8].is_empty() || fields[9].is_empty()) {
                    return Err(format!(
                        "mapping.tsv:{line}: REVIEWED_MAPPING requires reviewer and review_id"
                    ));
                }
                let metadata_file = resolve_source(metadata_root, metadata_path)?;
                let actual = if let Some(cached) = metadata_digests.get(metadata_path) {
                    cached.clone()
                } else {
                    let digest = sha256_file(&metadata_file)?;
                    metadata_digests.insert(metadata_path.to_owned(), digest.clone());
                    digest
                };
                if actual != metadata_sha {
                    return Err(format!(
                        "mapping.tsv:{line}: metadata digest mismatch for {metadata_path}"
                    ));
                }
            }
            "AMBIGUOUS" | "UNMAPPED" => {
                if metadata_path.is_some() || metadata_sha.is_some() {
                    return Err(format!(
                        "mapping.tsv:{line}: {status} rows must not nominate a metadata file"
                    ));
                }
            }
            other => {
                return Err(format!("mapping.tsv:{line}: unknown mapping_status {other:?}"));
            }
        }

        if fields[10].is_empty() || fields[11].is_empty() || fields[12].is_empty() {
            return Err(format!(
                "mapping.tsv:{line}: family, subtask and material must be explicit; use UNKNOWN where the source is silent"
            ));
        }

        mappings.push(Mapping {
            raw_path,
            raw_sha,
            metadata_path,
            metadata_sha,
            status: status.to_owned(),
            family: fields[10].clone(),
            subtask: fields[11].clone(),
            material: fields[12].clone(),
        });
    }
    Ok(mappings)
}

fn parse_dependence_keys(value: &str, line: usize) -> Result<Vec<String>, String> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    let mut keys = BTreeSet::new();
    for item in value.split(';') {
        let (kind, identity) = item
            .split_once(':')
            .ok_or_else(|| format!("grouping.tsv:{line}: malformed dependence key {item:?}"))?;
        if !DEPENDENCE_KINDS.contains(&kind) || identity.is_empty() || identity.trim() != identity {
            return Err(format!(
                "grouping.tsv:{line}: invalid dependence key {item:?}; allowed kinds: {}",
                DEPENDENCE_KINDS.join(", ")
            ));
        }
        if !keys.insert(item.to_owned()) {
            return Err(format!("grouping.tsv:{line}: duplicate dependence key {item:?}"));
        }
    }
    Ok(keys.into_iter().collect())
}

fn parse_groupings(rows: Vec<Vec<String>>) -> Result<Vec<Grouping>, String> {
    let mut output = Vec::with_capacity(rows.len());
    let mut seen = BTreeSet::new();

    for (index, fields) in rows.into_iter().enumerate() {
        let line = index + 2;
        let raw_path = fields[0].clone();
        let raw_sha = fields[1].to_ascii_lowercase();
        if !seen.insert(raw_path.clone()) {
            return Err(format!("grouping.tsv:{line}: duplicate raw_path {raw_path}"));
        }
        if !valid_sha(&raw_sha) {
            return Err(format!("grouping.tsv:{line}: malformed raw SHA-256"));
        }

        let disposition = fields[2].as_str();
        let unit_id = (!fields[3].is_empty()).then(|| fields[3].clone());
        let dependence_keys = parse_dependence_keys(&fields[9], line)?;
        let split = fields[10].as_str();
        if !SPLITS.contains(&split) {
            return Err(format!("grouping.tsv:{line}: invalid split {split:?}"));
        }

        match disposition {
            "SOURCE_BACKED" => {
                if unit_id.is_none()
                    || fields[4].is_empty()
                    || fields[5].is_empty()
                    || fields[6].is_empty()
                    || fields[7].is_empty()
                    || fields[8].is_empty()
                    || split == "UNASSIGNED"
                {
                    return Err(format!(
                        "grouping.tsv:{line}: SOURCE_BACKED requires unit ID, exact group evidence, reviewer, review ID and an assigned split"
                    ));
                }
                if !fields[4].starts_with("metadata/") && !fields[4].starts_with("raw/") {
                    return Err(format!(
                        "grouping.tsv:{line}: group_evidence_path must start with metadata/ or raw/"
                    ));
                }
            }
            "UNKNOWN" => {
                if unit_id.is_some()
                    || !dependence_keys.is_empty()
                    || !fields[4].is_empty()
                    || !fields[5].is_empty()
                    || !fields[6].is_empty()
                    || !fields[7].is_empty()
                    || !fields[8].is_empty()
                    || split != "UNASSIGNED"
                {
                    return Err(format!(
                        "grouping.tsv:{line}: UNKNOWN rows must have no unit, dependence keys or evidence and must be UNASSIGNED"
                    ));
                }
            }
            other => return Err(format!("grouping.tsv:{line}: unknown unit_disposition {other:?}")),
        }

        output.push(Grouping {
            raw_path,
            raw_sha,
            disposition: disposition.to_owned(),
            unit_id,
            dependence_keys,
            split: split.to_owned(),
        });
    }
    Ok(output)
}

fn json_escape(value: &str) -> String {
    let mut result = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            value if value < ' ' => {
                result.push_str(&format!("\\u{:04x}", value as u32));
            }
            value => result.push(value),
        }
    }
    result.push('"');
    result
}

fn json_optional(value: Option<&str>) -> String {
    value.map(json_escape).unwrap_or_else(|| "null".to_owned())
}

fn audit(args: &Args) -> Result<(Vec<Row>, String), String> {
    let raw_root = canonical_root(&args.raw_root)?;
    let metadata_root = canonical_root(&args.metadata_root)?;
    let discovered_raw = enumerate_json(&raw_root)?;
    let discovered_metadata = enumerate_json(&metadata_root)?;

    let mapping_rows = read_tsv(&args.mapping_tsv, MAPPING_HEADER, 13)?;
    let grouping_rows = read_tsv(&args.grouping_tsv, GROUPING_HEADER, 11)?;
    let mappings = parse_mappings(mapping_rows, &raw_root, &metadata_root)?;
    let groupings = parse_groupings(grouping_rows)?;

    let mapping_paths = mappings.iter().map(|row| row.raw_path.clone()).collect::<BTreeSet<_>>();
    let grouping_paths = groupings.iter().map(|row| row.raw_path.clone()).collect::<BTreeSet<_>>();
    let raw_paths = discovered_raw.iter().cloned().collect::<BTreeSet<_>>();
    if mapping_paths != raw_paths {
        let missing = raw_paths.difference(&mapping_paths).cloned().collect::<Vec<_>>();
        let extra = mapping_paths.difference(&raw_paths).cloned().collect::<Vec<_>>();
        return Err(format!("mapping coverage mismatch; missing={missing:?}; unexpected={extra:?}"));
    }
    if grouping_paths != raw_paths {
        let missing = raw_paths.difference(&grouping_paths).cloned().collect::<Vec<_>>();
        let extra = grouping_paths.difference(&raw_paths).cloned().collect::<Vec<_>>();
        return Err(format!("grouping coverage mismatch; missing={missing:?}; unexpected={extra:?}"));
    }

    let mapping_by_path = mappings
        .into_iter()
        .map(|row| (row.raw_path.clone(), row))
        .collect::<BTreeMap<_, _>>();
    let grouping_by_path = groupings
        .into_iter()
        .map(|row| (row.raw_path.clone(), row))
        .collect::<BTreeMap<_, _>>();

    let mut referenced_metadata = BTreeSet::new();
    let mut rows = Vec::with_capacity(discovered_raw.len());
    for raw_path in &discovered_raw {
        let mapping = mapping_by_path
            .get(raw_path)
            .ok_or_else(|| format!("internal mapping lookup failed for {raw_path}"))?
            .clone();
        let grouping = grouping_by_path
            .get(raw_path)
            .ok_or_else(|| format!("internal grouping lookup failed for {raw_path}"))?
            .clone();
        if mapping.raw_sha != grouping.raw_sha {
            return Err(format!("raw digest differs between manifests for {raw_path}"));
        }
        if let Some(metadata_path) = &mapping.metadata_path {
            referenced_metadata.insert(metadata_path.clone());
        }
        if mapping.status == "AMBIGUOUS" || mapping.status == "UNMAPPED" {
            if grouping.disposition != "UNKNOWN" {
                return Err(format!(
                    "{raw_path}: ambiguous/unmapped metadata cannot be assigned a source-backed physical unit"
                ));
            }
        }
        rows.push(Row { mapping, grouping });
    }

    let all_metadata = discovered_metadata.into_iter().collect::<BTreeSet<_>>();
    if referenced_metadata != all_metadata {
        let missing = all_metadata.difference(&referenced_metadata).cloned().collect::<Vec<_>>();
        let extra = referenced_metadata.difference(&all_metadata).cloned().collect::<Vec<_>>();
        return Err(format!(
            "metadata reference coverage mismatch; unreferenced_metadata={missing:?}; unexpected_metadata={extra:?}"
        ));
    }

    let mapping_digest = sha256_file(&args.mapping_tsv)?;
    let grouping_digest = sha256_file(&args.grouping_tsv)?;
    Ok((rows, format!("{mapping_digest}:{grouping_digest}")))
}

fn grouping_components(rows: &[Row]) -> (UnionFind, BTreeMap<usize, Vec<usize>>) {
    let mut union = UnionFind::with_len(rows.len());
    let mut first_by_key = HashMap::<String, usize>::new();

    for (index, row) in rows.iter().enumerate() {
        if row.grouping.disposition != "SOURCE_BACKED" {
            continue;
        }
        let keys = std::iter::once(format!(
            "unit:{}",
            row.grouping.unit_id.as_deref().unwrap_or_default()
        ))
        .chain(row.grouping.dependence_keys.iter().cloned().map(|key| format!("dep:{key}")))
        .chain(std::iter::once(format!("rawsha:{}", row.mapping.raw_sha)));
        for key in keys {
            if let Some(previous) = first_by_key.insert(key, index) {
                union.union(previous, index);
            }
        }
    }

    let mut components = BTreeMap::<usize, Vec<usize>>::new();
    for index in 0..rows.len() {
        if rows[index].grouping.disposition == "SOURCE_BACKED" {
            let root = union.find(index);
            components.entry(root).or_default().push(index);
        }
    }
    (union, components)
}

fn output_jsonl(args: &Args, rows: &[Row], digest_pair: &str) -> Result<String, String> {
    let (mut union, components) = grouping_components(rows);
    let mut out = String::new();
    out.push_str(&format!(
        "{{\"record_type\":\"header\",\"schema\":\"gauge-rigid-grouping-audit-v0.1\",\"dataset_revision\":{},\"mapping_tsv_sha256\":{},\"grouping_tsv_sha256\":{},\"claim_ceiling\":\"SOURCE_GROUPING_RECEIPT_ONLY_NOT_EXCHANGEABILITY_PROOF\"}}\n",
        json_escape(&args.revision),
        json_escape(digest_pair.split(':').next().unwrap_or_default()),
        json_escape(digest_pair.split(':').nth(1).unwrap_or_default()),
    ));

    let mut mapping_counts = BTreeMap::<String, usize>::new();
    let mut unknown_rows = 0usize;
    let mut split_counts = BTreeMap::<String, usize>::new();
    let mut domain_units = BTreeMap::<(String, String, String), BTreeMap<String, BTreeSet<usize>>>::new();
    let mut split_by_component = BTreeMap::<usize, BTreeSet<String>>::new();

    for row in rows {
        *mapping_counts.entry(row.mapping.status.clone()).or_default() += 1;
        if row.grouping.disposition == "UNKNOWN" {
            unknown_rows += 1;
        } else {
            *split_counts.entry(row.grouping.split.clone()).or_default() += 1;
        }
    }

    for (root, indices) in &components {
        let mut splits = BTreeSet::new();
        for index in indices {
            splits.insert(rows[*index].grouping.split.clone());
            let key = (
                rows[*index].mapping.family.clone(),
                rows[*index].mapping.subtask.clone(),
                rows[*index].mapping.material.clone(),
            );
            domain_units
                .entry(key)
                .or_default()
                .entry(rows[*index].grouping.split.clone())
                .or_default()
                .insert(*root);
        }
        split_by_component.insert(*root, splits);
    }

    let mut leakage_components = 0usize;
    for splits in split_by_component.values() {
        if splits.len() > 1 {
            leakage_components += 1;
        }
    }

    for (index, row) in rows.iter().enumerate() {
        let component = if row.grouping.disposition == "SOURCE_BACKED" {
            Some(union.find(index))
        } else {
            None
        };
        out.push_str(&format!(
            "{{\"record_type\":\"row\",\"raw_path\":{},\"raw_sha256\":{},\"metadata_path\":{},\"metadata_sha256\":{},\"mapping_status\":{},\"unit_disposition\":{},\"exchangeability_unit_id\":{},\"dependence_keys\":[{}],\"split\":{},\"dependence_component\":{}}}\n",
            json_escape(&row.mapping.raw_path),
            json_escape(&row.mapping.raw_sha),
            json_optional(row.mapping.metadata_path.as_deref()),
            json_optional(row.mapping.metadata_sha.as_deref()),
            json_escape(&row.mapping.status),
            json_escape(&row.grouping.disposition),
            json_optional(row.grouping.unit_id.as_deref()),
            row.grouping.dependence_keys.iter().map(|key| json_escape(key)).collect::<Vec<_>>().join(","),
            json_escape(&row.grouping.split),
            component.map(|value| value.to_string()).unwrap_or_else(|| "null".to_owned()),
        ));
    }

    for ((family, subtask, material), counts) in domain_units {
        let calibration_n = counts.get("CALIBRATION").map(BTreeSet::len).unwrap_or(0);
        let fit_n = counts.get("FIT").map(BTreeSet::len).unwrap_or(0);
        let test_n = counts.get("TEST").map(BTreeSet::len).unwrap_or(0);
        out.push_str(&format!(
            "{{\"record_type\":\"domain_summary\",\"family\":{},\"subtask\":{},\"material\":{},\"fit_units\":{},\"calibration_units\":{},\"test_units\":{},\"nominal_95_rank_status\":{}}}\n",
            json_escape(&family),
            json_escape(&subtask),
            json_escape(&material),
            fit_n,
            calibration_n,
            test_n,
            json_escape(if calibration_n >= 19 {
                "FINITE_RANK_POSSIBLE_NOT_COVERAGE_PROOF"
            } else {
                "UNBOUNDED_REQUIRED_BELOW_19_INDEPENDENT_UNITS"
            }),
        ));
    }

    out.push_str(&format!(
        "{{\"record_type\":\"summary\",\"raw_file_count\":{},\"source_backed_rows\":{},\"unknown_group_rows\":{},\"source_backed_dependence_components\":{},\"split_fit_rows\":{},\"split_calibration_rows\":{},\"split_test_rows\":{},\"mapping_status_counts\":{{{}}},\"cross_split_dependence_components\":{},\"split_leakage_status\":{},\"physical_trial_count\":\"NOT_ASSERTED\",\"exchangeability_status\":\"NOT_PROVEN\"}}\n",
        rows.len(),
        rows.len().saturating_sub(unknown_rows),
        unknown_rows,
        components.len(),
        split_counts.get("FIT").copied().unwrap_or(0),
        split_counts.get("CALIBRATION").copied().unwrap_or(0),
        split_counts.get("TEST").copied().unwrap_or(0),
        mapping_counts.iter().map(|(key,value)| format!("{}:{}",json_escape(key),value)).collect::<Vec<_>>().join(","),
        leakage_components,
        json_escape(if leakage_components == 0 { "PASS_NO_DECLARED_DEPENDENCE_LEAKAGE" } else { "FAIL_CROSS_SPLIT_DEPENDENCE" }),
    ));
    Ok(out)
}

fn run(args: Args) -> Result<bool, String> {
    let (rows, digests) = audit(&args)?;
    let receipt = output_jsonl(&args, &rows, &digests)?;
    if let Some(path) = &args.output {
        let mut file = File::create(path)
            .map_err(|error| format!("cannot create output {}: {error}", path.display()))?;
        file.write_all(receipt.as_bytes())
            .map_err(|error| format!("cannot write output {}: {error}", path.display()))?;
    } else {
        let stdout = io::stdout();
        let mut writer = io::BufWriter::new(stdout.lock());
        writer
            .write_all(receipt.as_bytes())
            .map_err(|error| format!("cannot write stdout: {error}"))?;
        writer.flush().map_err(|error| error.to_string())?;
    }
    let (mut union, components) = grouping_components(&rows);
    let mut splits_by_component = BTreeMap::<usize, BTreeSet<String>>::new();
    for (root, indices) in components {
        for index in indices {
            splits_by_component
                .entry(union.find(root))
                .or_default()
                .insert(rows[index].grouping.split.clone());
        }
    }
    Ok(splits_by_component.values().all(|splits| splits.len() <= 1))
}

fn main() {
    let args = match parse_args() {
        Ok(Some(args)) => args,
        Ok(None) => return,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    match run(args) {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_find_connects_transitive_shared_dependencies() {
        let mut union = UnionFind::with_len(4);
        union.union(0, 1);
        union.union(2, 3);
        union.union(1, 2);
        assert_eq!(union.find(0), union.find(3));
    }

    #[test]
    fn path_validation_rejects_absolute_parent_and_backslash_paths() {
        assert!(relative_path_is_safe("slope slider/json/1.json"));
        assert!(!relative_path_is_safe("../outside.json"));
        assert!(!relative_path_is_safe("/absolute/file.json"));
        assert!(!relative_path_is_safe("nested\\file.json"));
        assert!(!relative_path_is_safe("a/../../b.json"));
    }

    #[test]
    fn dependence_keys_are_namespaced_and_strict() {
        assert_eq!(
            parse_dependence_keys("article:A-1;session:S-2", 2).unwrap(),
            vec!["article:A-1".to_owned(), "session:S-2".to_owned()]
        );
        assert!(parse_dependence_keys("article:", 2).is_err());
        assert!(parse_dependence_keys("unknown:X", 2).is_err());
        assert!(parse_dependence_keys("article:A;article:A", 2).is_err());
    }

    #[test]
    fn json_escape_handles_quotes_slashes_and_controls() {
        assert_eq!(
            json_escape("a\"b\\c\n"),
            "\"a\\\"b\\\\c\\n\"".to_owned()
        );
    }

    #[test]
    fn split_vocabulary_is_closed() {
        assert!(SPLITS.contains(&"CALIBRATION"));
        assert!(!SPLITS.contains(&"VALIDATION"));
    }
}
