use engineering_scenario_compiler::strict_jcs_input::parse_strict_jcs_input;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const GENERATOR_ID: &str = "engineering-scenario-jcs-adversarial-corpus";
const GENERATOR_VERSION: &str = "0.1.0";
const PROFILE: &str = "engineering-scenario-ir-jcs-rfc8785-v1";
const SEED: u64 = 0x4a43_535f_434f_5250;
const GENERATED_CASES: usize = 256;

#[derive(Debug, Clone)]
struct ValidCase {
    case_id: String,
    input: Vec<u8>,
    equivalence_group: Option<String>,
}

#[derive(Debug, Clone)]
struct RejectCase {
    case_id: String,
    input: Vec<u8>,
    expected_rejection_class: String,
}

#[derive(Debug, Clone, Serialize)]
struct ValidManifestEntry {
    case_id: String,
    input_file: String,
    input_sha256: String,
    expected_canonical_file: String,
    expected_canonical_sha256: String,
    expected_canonical_bytes: usize,
    equivalence_group: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct RejectManifestEntry {
    case_id: String,
    input_file: String,
    input_sha256: String,
    expected_rejection_class: String,
}

#[derive(Debug, Clone, Serialize)]
struct CorpusManifest {
    schema_version: String,
    generator_id: String,
    generator_version: String,
    canonicalization_profile: String,
    deterministic_seed_hex: String,
    generated_case_count: usize,
    valid_case_count: usize,
    reject_case_count: usize,
    valid_cases: Vec<ValidManifestEntry>,
    reject_cases: Vec<RejectManifestEntry>,
    claim_ceiling: String,
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn add_valid(
    cases: &mut Vec<ValidCase>,
    case_id: &str,
    input: &[u8],
    equivalence_group: Option<&str>,
) {
    cases.push(ValidCase {
        case_id: case_id.to_owned(),
        input: input.to_vec(),
        equivalence_group: equivalence_group.map(str::to_owned),
    });
}

fn next_random(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn generated_value_cases() -> Result<Vec<ValidCase>, String> {
    let mut cases = Vec::with_capacity(GENERATED_CASES);
    let mut state = SEED;
    for index in 0..GENERATED_CASES {
        let a = next_random(&mut state) % 1_000_000_000_000;
        let b = next_random(&mut state) % 1_000_000_000_000;
        let c = next_random(&mut state) % 1_000_000_000_000;
        let flag = next_random(&mut state) & 1 == 1;
        let signed = if next_random(&mut state) & 1 == 1 {
            (next_random(&mut state) % 1_000_000) as i64
        } else {
            -((next_random(&mut state) % 1_000_000) as i64)
        };
        let depth = (next_random(&mut state) % 5) as usize + 1;
        let bulk_bytes = [64_usize, 512, 4_096, 16_384][index % 4];
        let bulk_payload = "p".repeat(bulk_bytes);
        let nested = (0..depth)
            .map(|level| {
                json!({
                    "level": level,
                    "token": format!("case-{index}-level-{level}-é-e\u{301}-𝄞"),
                    "number": next_random(&mut state) % 65_536
                })
            })
            .collect::<Vec<Value>>();
        let value = json!({
            "case": index,
            "payload_bytes": bulk_bytes,
            "bulk_payload": bulk_payload,
            "payload": {
                "values": [a, b, c, signed],
                "flags": [flag, !flag, true, false],
                "nested": nested,
                "unicode": ["é", "e\u{301}", "𝄞", "plain-ascii"],
                "optional": if flag { Value::Null } else { json!({"present": true}) }
            },
            "profile": PROFILE
        });
        let input = serde_json::to_vec(&value)
            .map_err(|error| format!("serialize generated valid case {index}: {error}"))?;
        cases.push(ValidCase {
            case_id: format!("generated-{index:04}"),
            input,
            equivalence_group: None,
        });
    }
    Ok(cases)
}

fn valid_cases() -> Result<Vec<ValidCase>, String> {
    let mut cases = Vec::new();
    add_valid(&mut cases, "empty-object", br#"{}"#, None);
    add_valid(
        &mut cases,
        "nested-values",
        br#"{"z":0,"a":[true,false,null,{"nested":"value"}]}"#,
        None,
    );
    add_valid(
        &mut cases,
        "object-key-order-forward",
        br#"{"z":1,"y":2,"a":3}"#,
        Some("object-order-equivalence"),
    );
    add_valid(
        &mut cases,
        "object-key-order-reverse",
        br#"{"a":3,"y":2,"z":1}"#,
        Some("object-order-equivalence"),
    );
    add_valid(
        &mut cases,
        "escaped-unicode-value",
        br#"{"glyph":"\uD834\uDD1E","text":"line\nbreak","letter":"\u00e9"}"#,
        Some("escaped-literal-equivalence"),
    );
    add_valid(
        &mut cases,
        "literal-unicode-value",
        "{\"letter\":\"é\",\"glyph\":\"𝄞\",\"text\":\"line\\nbreak\"}".as_bytes(),
        Some("escaped-literal-equivalence"),
    );
    add_valid(
        &mut cases,
        "utf16-property-order",
        "{\"keys\":{\"\\uE000\":\"bmp-private\",\"𝄞\":\"supplementary\",\"z\":\"last\",\"a\":\"first\"}}".as_bytes(),
        None,
    );
    add_valid(
        &mut cases,
        "unicode-normalization-distinct-keys",
        "{\"é\":\"composed\",\"e\\u0301\":\"decomposed\"}".as_bytes(),
        None,
    );
    add_valid(
        &mut cases,
        "binary64-boundaries",
        br#"{"values":[-0,5e-324,1.7976931348623157e308,0.000001,1e-7,1e+30]}"#,
        None,
    );
    add_valid(
        &mut cases,
        "safe-integer-boundaries",
        b"{\"maximum\":9007199254740991,\"minimum\":-9007199254740991}",
        None,
    );
    add_valid(
        &mut cases,
        "negative-zero-forms",
        br#"{"values":[-0,-0.0,0,0.0]}"#,
        None,
    );
    add_valid(
        &mut cases,
        "nested-arrays-and-objects",
        br#"{"outer":[{"items":[1,2,{"deep":{"enabled":true}}]},[],{"empty":null}]}"#,
        None,
    );

    cases.extend(generated_value_cases()?);
    Ok(cases)
}

fn reject_cases() -> Vec<RejectCase> {
    vec![
        RejectCase {
            case_id: "duplicate-root-key".to_owned(),
            input: br#"{"x":1,"x":2}"#.to_vec(),
            expected_rejection_class: "duplicate_member_name".to_owned(),
        },
        RejectCase {
            case_id: "duplicate-nested-key".to_owned(),
            input: br#"{"outer":{"x":1,"x":2}}"#.to_vec(),
            expected_rejection_class: "duplicate_member_name".to_owned(),
        },
        RejectCase {
            case_id: "duplicate-key-in-array-object".to_owned(),
            input: br#"{"items":[{"x":1,"x":2}]}"#.to_vec(),
            expected_rejection_class: "duplicate_member_name".to_owned(),
        },
        RejectCase {
            case_id: "escaped-equivalent-duplicate".to_owned(),
            input: br#"{"a":1,"\u0061":2}"#.to_vec(),
            expected_rejection_class: "duplicate_member_name".to_owned(),
        },
        RejectCase {
            case_id: "unicode-escaped-equivalent-duplicate".to_owned(),
            input: "{\"é\":1,\"\\u00e9\":2}".as_bytes().to_vec(),
            expected_rejection_class: "duplicate_member_name".to_owned(),
        },
        RejectCase {
            case_id: "invalid-utf8".to_owned(),
            input: vec![0xff, b'{', b'}'],
            expected_rejection_class: "invalid_utf8".to_owned(),
        },
        RejectCase {
            case_id: "unpaired-high-surrogate".to_owned(),
            input: br#"{"bad":"\uD800"}"#.to_vec(),
            expected_rejection_class: "unpaired_surrogate".to_owned(),
        },
        RejectCase {
            case_id: "unpaired-low-surrogate".to_owned(),
            input: br#"{"bad":"\uDC00"}"#.to_vec(),
            expected_rejection_class: "unpaired_surrogate".to_owned(),
        },
        RejectCase {
            case_id: "binary64-overflow".to_owned(),
            input: b"1e9999".to_vec(),
            expected_rejection_class: "non_finite_number".to_owned(),
        },
        RejectCase {
            case_id: "unsafe-positive-integer".to_owned(),
            input: b"9007199254740992".to_vec(),
            expected_rejection_class: "unsafe_integer".to_owned(),
        },
        RejectCase {
            case_id: "unsafe-negative-integer".to_owned(),
            input: b"-9007199254740992".to_vec(),
            expected_rejection_class: "unsafe_integer".to_owned(),
        },
        RejectCase {
            case_id: "unsafe-integer-over-u64".to_owned(),
            input: b"18446744073709551616".to_vec(),
            expected_rejection_class: "unsafe_integer".to_owned(),
        },
        RejectCase {
            case_id: "unsafe-integer-over-i64".to_owned(),
            input: b"-9223372036854775809".to_vec(),
            expected_rejection_class: "unsafe_integer".to_owned(),
        },
        RejectCase {
            case_id: "trailing-json-value".to_owned(),
            input: b"{} {}".to_vec(),
            expected_rejection_class: "trailing_content".to_owned(),
        },
        RejectCase {
            case_id: "malformed-token".to_owned(),
            input: br#"{"x":]}"#.to_vec(),
            expected_rejection_class: "malformed_json".to_owned(),
        },
        RejectCase {
            case_id: "unescaped-control-character".to_owned(),
            input: b"{\"x\":\"raw\nnewline\"}".to_vec(),
            expected_rejection_class: "malformed_json".to_owned(),
        },
    ]
}

fn validate_case_ids(valid: &[ValidCase], rejected: &[RejectCase]) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    for case_id in valid.iter().map(|case| &case.case_id).chain(rejected.iter().map(|case| &case.case_id)) {
        if !ids.insert(case_id.clone()) {
            return Err(format!("duplicate corpus case ID {case_id:?}"));
        }
    }
    Ok(())
}

fn verify_equivalence_groups(
    cases: &[ValidCase],
    canonical: &BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    let mut groups = BTreeMap::<String, Vec<u8>>::new();
    for case in cases {
        let Some(group) = &case.equivalence_group else {
            continue;
        };
        let bytes = canonical
            .get(&case.case_id)
            .ok_or_else(|| format!("missing canonical bytes for {}", case.case_id))?;
        if let Some(previous) = groups.get(group) {
            if previous != bytes {
                return Err(format!("metamorphic canonical bytes diverged in group {group}"));
            }
        } else {
            groups.insert(group.clone(), bytes.clone());
        }
    }
    Ok(())
}

fn generate(output_directory: &Path) -> Result<CorpusManifest, String> {
    if output_directory.exists() {
        let mut entries = fs::read_dir(output_directory)
            .map_err(|error| format!("read existing output directory: {error}"))?;
        if entries.next().is_some() {
            return Err(format!(
                "output directory {} must be empty",
                output_directory.display()
            ));
        }
    } else {
        fs::create_dir_all(output_directory)
            .map_err(|error| format!("create output directory: {error}"))?;
    }
    let valid_directory = output_directory.join("valid");
    let expected_directory = output_directory.join("expected");
    let reject_directory = output_directory.join("reject");
    fs::create_dir_all(&valid_directory)
        .map_err(|error| format!("create valid-input directory: {error}"))?;
    fs::create_dir_all(&expected_directory)
        .map_err(|error| format!("create expected-output directory: {error}"))?;
    fs::create_dir_all(&reject_directory)
        .map_err(|error| format!("create reject-input directory: {error}"))?;

    let valid = valid_cases()?;
    let rejected = reject_cases();
    validate_case_ids(&valid, &rejected)?;

    let mut valid_entries = Vec::with_capacity(valid.len());
    let mut canonical_by_id = BTreeMap::new();
    for case in &valid {
        let parsed = parse_strict_jcs_input(&case.input)
            .map_err(|error| format!("Rust strict parser rejected valid case {}: {error}", case.case_id))?;
        let canonical = serde_jcs::to_vec(&parsed)
            .map_err(|error| format!("Rust JCS canonicalization failed for {}: {error}", case.case_id))?;
        let input_relative = format!("valid/{}.json", case.case_id);
        let expected_relative = format!("expected/{}.jcs", case.case_id);
        fs::write(output_directory.join(&input_relative), &case.input)
            .map_err(|error| format!("write valid case {}: {error}", case.case_id))?;
        fs::write(output_directory.join(&expected_relative), &canonical)
            .map_err(|error| format!("write expected canonical bytes {}: {error}", case.case_id))?;
        canonical_by_id.insert(case.case_id.clone(), canonical.clone());
        valid_entries.push(ValidManifestEntry {
            case_id: case.case_id.clone(),
            input_file: input_relative,
            input_sha256: sha256(&case.input),
            expected_canonical_file: expected_relative,
            expected_canonical_sha256: sha256(&canonical),
            expected_canonical_bytes: canonical.len(),
            equivalence_group: case.equivalence_group.clone(),
        });
    }
    verify_equivalence_groups(&valid, &canonical_by_id)?;

    let mut reject_entries = Vec::with_capacity(rejected.len());
    for case in &rejected {
        if parse_strict_jcs_input(&case.input).is_ok() {
            return Err(format!(
                "Rust strict parser unexpectedly accepted reject case {}",
                case.case_id
            ));
        }
        let relative = format!("reject/{}.bin", case.case_id);
        fs::write(output_directory.join(&relative), &case.input)
            .map_err(|error| format!("write reject case {}: {error}", case.case_id))?;
        reject_entries.push(RejectManifestEntry {
            case_id: case.case_id.clone(),
            input_file: relative,
            input_sha256: sha256(&case.input),
            expected_rejection_class: case.expected_rejection_class.clone(),
        });
    }

    let manifest = CorpusManifest {
        schema_version: "engineering-scenario-jcs-corpus-v0.1".to_owned(),
        generator_id: GENERATOR_ID.to_owned(),
        generator_version: GENERATOR_VERSION.to_owned(),
        canonicalization_profile: PROFILE.to_owned(),
        deterministic_seed_hex: format!("{SEED:016x}"),
        generated_case_count: GENERATED_CASES,
        valid_case_count: valid_entries.len(),
        reject_case_count: reject_entries.len(),
        valid_cases: valid_entries,
        reject_cases: reject_entries,
        claim_ceiling: concat!(
            "byte identity and strict raw-input rejection behavior for this fixed generated corpus ",
            "and RFC 8785 profile only; no engineering-model or physical-correctness claim"
        )
        .to_owned(),
    };
    let bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| format!("serialize corpus manifest: {error}"))?;
    fs::write(output_directory.join("manifest.json"), &bytes)
        .map_err(|error| format!("write corpus manifest: {error}"))?;
    fs::write(
        output_directory.join("manifest.sha256"),
        format!("{}  manifest.json\n", sha256(&bytes)),
    )
    .map_err(|error| format!("write corpus manifest digest: {error}"))?;
    Ok(manifest)
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let output_directory = args
        .next()
        .ok_or_else(|| "usage: jcs-corpus-generator <empty-output-directory>".to_owned())?;
    if args.next().is_some() {
        return Err("expected exactly one output directory".to_owned());
    }
    let manifest = generate(&PathBuf::from(output_directory))?;
    println!("generator_id={}", manifest.generator_id);
    println!("profile={}", manifest.canonicalization_profile);
    println!("seed={}", manifest.deterministic_seed_hex);
    println!("generated_case_count={}", manifest.generated_case_count);
    println!("valid_case_count={}", manifest.valid_case_count);
    println!("reject_case_count={}", manifest.reject_case_count);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("JCS corpus generation failed: {error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_valid_and_reject_panels_are_strictly_classified() {
        let valid = valid_cases().unwrap();
        let rejected = reject_cases();
        assert_eq!(valid.len(), 12 + GENERATED_CASES);
        assert_eq!(rejected.len(), 16);
        validate_case_ids(&valid, &rejected).unwrap();
        for case in &valid {
            assert!(parse_strict_jcs_input(&case.input).is_ok(), "{}", case.case_id);
        }
        for case in &rejected {
            assert!(parse_strict_jcs_input(&case.input).is_err(), "{}", case.case_id);
        }
    }

    #[test]
    fn object_order_and_escaped_literal_pairs_have_identical_canonical_bytes() {
        let valid = valid_cases().unwrap();
        let mut grouped = BTreeMap::<String, Vec<Vec<u8>>>::new();
        for case in &valid {
            if let Some(group) = &case.equivalence_group {
                let parsed = parse_strict_jcs_input(&case.input).unwrap();
                let canonical = serde_jcs::to_vec(&parsed).unwrap();
                grouped.entry(group.clone()).or_default().push(canonical);
            }
        }
        assert_eq!(grouped.len(), 2);
        for (group, outputs) in grouped {
            assert_eq!(outputs.len(), 2, "{group}");
            assert_eq!(outputs[0], outputs[1], "{group}");
        }
    }
}
