use gauge_rigid_raw_trial_contract::validate_trial_json;
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

const REQUIRED_FAMILIES: [&str; 8] = [
    "bouncing ball",
    "newton's cradle",
    "nonsmooth contact",
    "pendulum",
    "slope contact",
    "slope slider",
    "turntable",
    "wall breaking",
];

#[derive(Debug)]
struct Arguments {
    root: PathBuf,
    dataset_revision: String,
    output: Option<PathBuf>,
}

#[derive(Debug)]
struct ScanIssue {
    path: String,
    reason: String,
}

#[derive(Debug)]
struct FileResult {
    path: String,
    family: String,
    material: Option<String>,
    subtask: Option<String>,
    record_label: String,
    byte_len: usize,
    sha256: Option<String>,
    disposition: &'static str,
    error: Option<String>,
    fps_hz: Option<f64>,
    translation_unit: Option<&'static str>,
    euler_order: Option<&'static str>,
    quaternion_component_order: Option<&'static str>,
    body_streams: Vec<(String, usize, f64)>,
    semantics: Option<String>,
}

fn usage() -> &'static str {
    "Usage: gauge-rigid-inventory --root PATH --dataset-revision REV [--output PATH]"
}

fn parse_arguments() -> Result<Option<Arguments>, String> {
    let mut args = env::args_os().skip(1);
    let mut root = None;
    let mut revision = None;
    let mut output = None;

    while let Some(flag) = args.next() {
        if flag == OsStr::new("--help") || flag == OsStr::new("-h") {
            println!("{}", usage());
            return Ok(None);
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {}", flag.to_string_lossy()))?;
        match flag.to_str() {
            Some("--root") => root = Some(PathBuf::from(value)),
            Some("--dataset-revision") => {
                revision = Some(
                    value
                        .into_string()
                        .map_err(|_| "dataset revision must be UTF-8".to_owned())?,
                );
            }
            Some("--output") => output = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown argument: {}", flag.to_string_lossy())),
        }
    }

    let root = root.ok_or_else(|| format!("--root is required\n{}", usage()))?;
    let dataset_revision =
        revision.ok_or_else(|| format!("--dataset-revision is required\n{}", usage()))?;
    if dataset_revision.trim().is_empty() {
        return Err("--dataset-revision must not be empty".to_owned());
    }

    Ok(Some(Arguments {
        root,
        dataset_revision,
        output,
    }))
}

fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{000c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            value if value <= '\u{001f}' => {
                use std::fmt::Write as FmtWrite;
                let _ = write!(output, "\\u{:04x}", value as u32);
            }
            value => output.push(value),
        }
    }
    output.push('"');
    output
}

fn json_optional_string(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

fn relative_utf8(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| "enumerated path escaped the selected root".to_owned())?;
    let mut pieces = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(value) => pieces.push(
                value
                    .to_str()
                    .ok_or_else(|| "non-UTF-8 path cannot be represented in JSONL".to_owned())?,
            ),
            _ => return Err("unexpected non-normal relative path component".to_owned()),
        }
    }
    Ok(pieces.join("/"))
}

fn scan_directory(
    root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
    issues: &mut Vec<ScanIssue>,
) {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            let path = relative_utf8(root, directory)
                .unwrap_or_else(|_| directory.display().to_string());
            issues.push(ScanIssue {
                path,
                reason: format!("cannot read directory: {error}"),
            });
            return;
        }
    };

    let mut entries = entries.collect::<Vec<_>>();
    entries.sort_by(|left, right| left.as_ref().ok().map(|e| e.file_name()).cmp(
        &right.as_ref().ok().map(|e| e.file_name())
    ));

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                issues.push(ScanIssue {
                    path: relative_utf8(root, directory)
                        .unwrap_or_else(|_| directory.display().to_string()),
                    reason: format!("cannot enumerate directory entry: {error}"),
                });
                continue;
            }
        };
        let path = entry.path();
        let relative = relative_utf8(root, &path)
            .unwrap_or_else(|_| path.display().to_string());
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                issues.push(ScanIssue {
                    path: relative,
                    reason: format!("cannot inspect entry: {error}"),
                });
                continue;
            }
        };

        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            issues.push(ScanIssue {
                path: relative,
                reason: "symlink encountered; inventory never follows symlinks".to_owned(),
            });
        } else if file_type.is_dir() {
            scan_directory(root, &path, files, issues);
        } else if file_type.is_file()
            && path.extension().and_then(OsStr::to_str) == Some("json")
        {
            files.push(path);
        }
    }
}

fn sorted_json_files(root: &Path) -> (Vec<PathBuf>, Vec<ScanIssue>) {
    let mut files = Vec::new();
    let mut issues = Vec::new();
    scan_directory(root, root, &mut files, &mut issues);
    files.sort_by(|left, right| {
        let left = relative_utf8(root, left).unwrap_or_default();
        let right = relative_utf8(root, right).unwrap_or_default();
        left.as_bytes().cmp(right.as_bytes())
    });
    issues.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    (files, issues)
}

fn sha256_bytes(bytes: &[u8]) -> Result<String, String> {
    let mut child = Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("sha256sum is unavailable: {error}"))?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| "sha256sum stdin pipe was not created".to_owned())?;
        stdin
            .write_all(bytes)
            .map_err(|error| format!("cannot provide raw bytes to sha256sum: {error}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|error| format!("cannot collect sha256sum result: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "sha256sum failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let digest = String::from_utf8(output.stdout)
        .map_err(|_| "sha256sum emitted non-UTF-8 output".to_owned())?
        .split_whitespace()
        .next()
        .ok_or_else(|| "sha256sum emitted an empty digest".to_owned())?
        .to_owned();
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("sha256sum emitted a malformed SHA-256 digest".to_owned());
    }
    Ok(digest.to_ascii_lowercase())
}

fn path_labels(relative_path: &str) -> (String, Option<String>, Option<String>, String) {
    let components = relative_path.split('/').collect::<Vec<_>>();
    let family = components.first().copied().unwrap_or("UNKNOWN").to_owned();
    let file_name = components.last().copied().unwrap_or("");
    let record_label = file_name
        .strip_suffix(".json")
        .unwrap_or(file_name)
        .to_owned();

    let json_index = components.iter().position(|component| *component == "json");
    let Some(json_index) = json_index else {
        return (family, None, None, record_label);
    };

    let directories_after_json = components
        .iter()
        .skip(json_index + 1)
        .take(components.len().saturating_sub(json_index + 2))
        .copied()
        .collect::<Vec<_>>();
    let mut material = None;
    let mut subtask = None;
    let mut rest = directories_after_json.as_slice();

    if let Some(first) = rest.first().copied() {
        if ["wood", "plastic", "metal"].contains(&first) {
            material = Some(first.to_owned());
            rest = &rest[1..];
        }
    }
    if !rest.is_empty() {
        subtask = Some(rest.join("/"));
    }

    (family, material, subtask, record_label)
}

fn semantics_json(report: &gauge_rigid_raw_trial_contract::TrialReport) -> String {
    let s = report.semantics;
    format!(
        "{{\"fps\":\"{:?}\",\"translation_unit\":\"{:?}\",\"euler_order\":\"{:?}\",\"quaternion_component_storage_order\":\"{:?}\",\"quaternion_algebra\":\"{:?}\",\"transform_direction\":\"{:?}\",\"coordinate_handedness\":\"{:?}\",\"frame_identifiers\":\"{:?}\",\"euler_angle_unit\":\"{:?}\",\"absolute_clock_origin\":\"{:?}\",\"per_sample_measurement_uncertainty\":\"{:?}\"}}",
        s.fps,
        s.translation_unit,
        s.euler_order,
        s.quaternion_component_storage_order,
        s.quaternion_algebra,
        s.transform_direction,
        s.coordinate_handedness,
        s.frame_identifiers,
        s.euler_angle_unit,
        s.absolute_clock_origin,
        s.per_sample_measurement_uncertainty,
    )
}

fn inspect_file(root: &Path, path: &Path) -> Result<FileResult, String> {
    let relative = relative_utf8(root, path)?;
    let (family, material, subtask, record_label) = path_labels(&relative);
    let mut file = File::open(path).map_err(|error| format!("cannot open file: {error}"))?;
    let mut raw_bytes = Vec::new();
    file.read_to_end(&mut raw_bytes)
        .map_err(|error| format!("cannot read raw file: {error}"))?;
    let sha256 = sha256_bytes(&raw_bytes)?;

    let text = std::str::from_utf8(&raw_bytes)
        .map_err(|error| format!("raw JSON is not valid UTF-8: {error}"))?;
    match validate_trial_json(text) {
        Ok(report) => {
            let mut streams = report
                .bodies
                .iter()
                .map(|body| {
                    (
                        body.body_id.clone(),
                        body.sample_count,
                        body.maximum_quaternion_norm_deviation,
                    )
                })
                .collect::<Vec<_>>();
            streams.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
            Ok(FileResult {
                path: relative,
                family,
                material,
                subtask,
                record_label,
                byte_len: raw_bytes.len(),
                sha256: Some(sha256),
                disposition: "VALID_OBSERVED_PROFILE",
                error: None,
                fps_hz: Some(report.fps_hz),
                translation_unit: Some(report.translation_unit),
                euler_order: Some(report.euler_order),
                quaternion_component_order: Some(report.quaternion_component_order),
                body_streams: streams,
                semantics: Some(semantics_json(&report)),
            })
        }
        Err(error) => Ok(FileResult {
            path: relative,
            family,
            material,
            subtask,
            record_label,
            byte_len: raw_bytes.len(),
            sha256: Some(sha256),
            disposition: "INVALID_OR_UNSUPPORTED_PROFILE",
            error: Some(error.to_string()),
            fps_hz: None,
            translation_unit: None,
            euler_order: None,
            quaternion_component_order: None,
            body_streams: Vec::new(),
            semantics: None,
        }),
    }
}

fn file_record_json(record: &FileResult) -> String {
    let streams = record
        .body_streams
        .iter()
        .map(|(body_id, sample_count, norm_deviation)| {
            format!(
                "{{\"body_id\":{},\"sample_count\":{},\"maximum_quaternion_norm_deviation\":{:.17}}}",
                json_string(body_id),
                sample_count,
                norm_deviation,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"record_type\":\"file\",\"path\":{},\"family_path_label\":{},\"material_path_label\":{},\"subtask_path_label\":{},\"record_label\":{},\"byte_length\":{},\"sha256\":{},\"disposition\":{},\"error\":{},\"fps_hz\":{},\"translation_unit\":{},\"euler_order\":{},\"quaternion_component_order\":{},\"body_streams\":[{}],\"semantics\":{},\"metadata_mapping\":\"NOT_ASSESSED\",\"physical_trial_or_session_grouping\":\"UNKNOWN\"}}",
        json_string(&record.path),
        json_string(&record.family),
        json_optional_string(record.material.as_deref()),
        json_optional_string(record.subtask.as_deref()),
        json_string(&record.record_label),
        record.byte_len,
        json_optional_string(record.sha256.as_deref()),
        json_string(record.disposition),
        json_optional_string(record.error.as_deref()),
        record
            .fps_hz
            .map(|value| format!("{value:.17}"))
            .unwrap_or_else(|| "null".to_owned()),
        json_optional_string(record.translation_unit),
        json_optional_string(record.euler_order),
        json_optional_string(record.quaternion_component_order),
        streams,
        record.semantics.clone().unwrap_or_else(|| "null".to_owned()),
    )
}

fn issue_record_json(issue: &ScanIssue) -> String {
    format!(
        "{{\"record_type\":\"scan_issue\",\"path\":{},\"reason\":{}}}",
        json_string(&issue.path),
        json_string(&issue.reason)
    )
}

fn header_json(revision: &str, root: &Path) -> String {
    format!(
        "{{\"record_type\":\"header\",\"dataset_revision\":{},\"root\":{},\"inventory_schema\":\"gauge-rigid-source-inventory-v0.1\",\"digest\":\"SHA-256 of exact raw file bytes\",\"parser\":\"gauge-rigid-raw-trial-contract/v0.1\",\"file_count_is_not_physical_trial_count\":true,\"trial_grouping\":\"UNKNOWN_UNLESS_SEPARATELY_SOURCED\"}}",
        json_string(revision),
        json_string(&root.display().to_string()),
    )
}

fn summary_json(
    records: &[FileResult],
    issues: &[ScanIssue],
    missing_families: &[String],
) -> String {
    let mut family_counts = BTreeMap::<String, (usize, usize, usize)>::new();
    for family in REQUIRED_FAMILIES {
        family_counts.insert(family.to_owned(), (0, 0, 0));
    }

    let mut valid_count = 0usize;
    let mut invalid_count = 0usize;
    let mut unknown_family_count = 0usize;
    for record in records {
        let entry = family_counts
            .entry(record.family.clone())
            .or_insert((0, 0, 0));
        entry.0 += 1;
        if record.disposition == "VALID_OBSERVED_PROFILE" {
            valid_count += 1;
            entry.1 += 1;
        } else {
            invalid_count += 1;
            entry.2 += 1;
        }
        if !REQUIRED_FAMILIES.contains(&record.family.as_str()) {
            unknown_family_count += 1;
        }
    }

    let families = family_counts
        .iter()
        .map(|(name, (total, valid, invalid))| {
            format!(
                "{{\"family_path_label\":{},\"json_file_count\":{},\"valid_count\":{},\"invalid_count\":{}}}",
                json_string(name),
                total,
                valid,
                invalid,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let missing = missing_families
        .iter()
        .map(|name| json_string(name))
        .collect::<Vec<_>>()
        .join(",");
    let passed = invalid_count == 0 && issues.is_empty() && missing_families.is_empty();

    format!(
        "{{\"record_type\":\"summary\",\"json_file_count\":{},\"valid_count\":{},\"invalid_or_unsupported_count\":{},\"unknown_family_file_count\":{},\"scan_issue_count\":{},\"missing_required_families\":[{}],\"required_family_coverage\":[{}],\"structure_only_disposition\":\"{}\",\"file_count_is_not_physical_trial_count\":true,\"physical_trial_count\":\"NOT_COMPUTED\",\"session_article_grouping\":\"UNKNOWN\"}}",
        records.len(),
        valid_count,
        invalid_count,
        unknown_family_count,
        issues.len(),
        missing,
        families,
        if passed { "PASS_STRUCTURE_ONLY" } else { "FAIL_INCOMPLETE_OR_INVALID" },
    )
}

fn write_manifest<W: Write>(
    writer: &mut W,
    revision: &str,
    root: &Path,
    records: &[FileResult],
    issues: &[ScanIssue],
) -> io::Result<()> {
    writer.write_all(header_json(revision, root).as_bytes())?;
    writer.write_all(b"\n")?;
    for record in records {
        writer.write_all(file_record_json(record).as_bytes())?;
        writer.write_all(b"\n")?;
    }
    for issue in issues {
        writer.write_all(issue_record_json(issue).as_bytes())?;
        writer.write_all(b"\n")?;
    }

    let seen = records
        .iter()
        .map(|record| record.family.as_str())
        .collect::<Vec<_>>();
    let missing = REQUIRED_FAMILIES
        .iter()
        .filter(|family| !seen.contains(family))
        .map(|family| (*family).to_owned())
        .collect::<Vec<_>>();
    writer.write_all(summary_json(records, issues, &missing).as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()
}

fn run(arguments: Arguments) -> Result<bool, String> {
    let root = fs::canonicalize(&arguments.root)
        .map_err(|error| format!("cannot resolve root directory: {error}"))?;
    if !root.is_dir() {
        return Err("--root must identify a directory".to_owned());
    }

    let (paths, mut issues) = sorted_json_files(&root);
    let mut records = Vec::with_capacity(paths.len());
    let mut hash_or_read_failure = false;

    for path in paths {
        let relative = relative_utf8(&root, &path)
            .unwrap_or_else(|_| path.display().to_string());
        match inspect_file(&root, &path) {
            Ok(record) => records.push(record),
            Err(error) => {
                hash_or_read_failure = true;
                let (family, material, subtask, record_label) = path_labels(&relative);
                let byte_len = fs::metadata(&path)
                    .ok()
                    .and_then(|metadata| usize::try_from(metadata.len()).ok())
                    .unwrap_or(0);
                records.push(FileResult {
                    path: relative,
                    family,
                    material,
                    subtask,
                    record_label,
                    byte_len,
                    sha256: None,
                    disposition: "NOT_EVALUABLE_HASH_OR_READ_FAILURE",
                    error: Some(error),
                    fps_hz: None,
                    translation_unit: None,
                    euler_order: None,
                    quaternion_component_order: None,
                    body_streams: Vec::new(),
                    semantics: None,
                });
            }
        }
    }

    let seen = records
        .iter()
        .map(|record| record.family.as_str())
        .collect::<Vec<_>>();
    let missing = REQUIRED_FAMILIES
        .iter()
        .filter(|family| !seen.contains(family))
        .map(|family| (*family).to_owned())
        .collect::<Vec<_>>();

    if let Some(output_path) = arguments.output {
        let file = File::create(&output_path)
            .map_err(|error| format!("cannot create manifest {}: {error}", output_path.display()))?;
        let mut writer = BufWriter::new(file);
        write_manifest(
            &mut writer,
            &arguments.dataset_revision,
            &root,
            &records,
            &issues,
        )
        .map_err(|error| format!("cannot write manifest: {error}"))?;
    } else {
        let stdout = io::stdout();
        let mut writer = BufWriter::new(stdout.lock());
        write_manifest(
            &mut writer,
            &arguments.dataset_revision,
            &root,
            &records,
            &issues,
        )
        .map_err(|error| format!("cannot write manifest: {error}"))?;
    }

    let passed = !hash_or_read_failure
        && issues.is_empty()
        && missing.is_empty()
        && !records.is_empty()
        && records
            .iter()
            .all(|record| record.disposition == "VALID_OBSERVED_PROFILE");
    if !passed {
        issues.clear();
    }
    Ok(passed)
}

fn main() {
    let arguments = match parse_arguments() {
        Ok(Some(arguments)) => arguments,
        Ok(None) => return,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    match run(arguments) {
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
    fn json_string_escapes_quotes_control_characters_and_unicode_safely() {
        let input = "trail \"quote\" \\\n\t\u{0001} / café";
        let escaped = json_string(input);
        assert_eq!(escaped, "\"trail \\\"quote\\\" \\\\\\n\\t\\u0001 / café\"");
    }

    #[test]
    fn path_labels_preserve_family_material_and_subtask_segments() {
        let (family, material, subtask, record) = path_labels(
            "slope slider/json/metal/task-2/record 1.json",
        );
        assert_eq!(family, "slope slider");
        assert_eq!(material.as_deref(), Some("metal"));
        assert_eq!(subtask.as_deref(), Some("task-2"));
        assert_eq!(record, "record 1");
    }

    #[test]
    fn path_labels_do_not_invent_material_for_direct_record_layout() {
        let (family, material, subtask, record) =
            path_labels("newton's cradle/json/1.json");
        assert_eq!(family, "newton's cradle");
        assert_eq!(material, None);
        assert_eq!(subtask, None);
        assert_eq!(record, "1");
    }

    #[test]
    fn sha256_hashes_exact_raw_bytes() {
        assert_eq!(
            sha256_bytes(b"abc").unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_ne!(sha256_bytes(b"abc").unwrap(), sha256_bytes(b"abc\n").unwrap());
    }

    #[test]
    fn required_families_are_eight_distinct_names() {
        let mut unique = REQUIRED_FAMILIES.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), 8);
    }
}
