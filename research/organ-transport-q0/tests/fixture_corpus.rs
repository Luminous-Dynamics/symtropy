//! Executable verifier for the pinned Q0 tube and small-network fixture corpora.
//!
//! A passing test establishes only analytic/software conformance for these fixtures.
//! It does not validate physical devices, tissue models, or biological outcomes.

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use symtropy_organ_transport_q0::{
    evaluate_tube, junction_residual_m3_per_s, FlowError, TubeSpec,
};

const TUBE_CORPUS: &str = include_str!("../tube-reference-cases.v0.json");
const NETWORK_CORPUS: &str = include_str!("../network-reference-cases.v0.json");
const TUBE_CASE_IDS: &[&str] = &[
    "q0-tube-baseline",
    "q0-tube-reverse-pressure",
    "q0-tube-zero-pressure",
    "q0-tube-nonpositive-radius",
    "q0-tube-nonfinite-pressure",
];
const NETWORK_CASE_IDS: &[&str] = &[
    "q0-network-series-equal-resistance",
    "q0-network-bifurcation-two-equal-outlets",
    "q0-network-merge-two-inlets",
    "q0-network-bifurcation-three-to-one-flow-split",
    "q0-network-two-level-branching",
    "q0-network-reversed-edge-orientation",
];
const CORPUS_MANIFEST: &str = include_str!("../corpus-manifest.v0.json");

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    value
        .get(key)
        .unwrap_or_else(|| panic!("required field {key:?} is absent from {value}"))
}

fn string_field<'a>(value: &'a Value, key: &str) -> &'a str {
    field(value, key)
        .as_str()
        .unwrap_or_else(|| panic!("field {key:?} must be a string: {value}"))
}

fn number(value: &Value) -> f64 {
    if let Some(value) = value.as_f64() {
        return value;
    }
    match value.as_str() {
        Some("Infinity") => f64::INFINITY,
        Some("-Infinity") => f64::NEG_INFINITY,
        Some("NaN") => f64::NAN,
        _ => panic!("expected a JSON number or a declared non-finite sentinel: {value}"),
    }
}

fn number_field(value: &Value, key: &str) -> f64 {
    number(field(value, key))
}

fn parse_tube_spec(inputs: &Value) -> TubeSpec {
    TubeSpec {
        length_m: number_field(inputs, "length_m"),
        radius_m: number_field(inputs, "radius_m"),
        dynamic_viscosity_pa_s: number_field(inputs, "dynamic_viscosity_pa_s"),
        pressure_drop_pa: number_field(inputs, "pressure_drop_pa"),
    }
}

fn error_label(error: FlowError) -> String {
    match error {
        FlowError::NonFiniteInput(field) => format!("NonFiniteInput({field})"),
        FlowError::NonPositiveInput(field) => format!("NonPositiveInput({field})"),
        FlowError::NumericalRange => "NumericalRange".to_owned(),
    }
}

fn assert_close(actual: f64, expected: f64, relative: f64, absolute: f64, context: &str) {
    assert!(actual.is_finite(), "{context}: actual value is not finite: {actual}");
    assert!(expected.is_finite(), "{context}: expected value is not finite: {expected}");
    assert!(
        relative.is_finite() && relative >= 0.0 && absolute.is_finite() && absolute >= 0.0,
        "{context}: tolerances must be finite and non-negative"
    );
    let allowed = absolute + relative * expected.abs();
    assert!(
        (actual - expected).abs() <= allowed,
        "{context}: actual {actual:.16e}, expected {expected:.16e}, allowed error {allowed:.3e}"
    );
}

fn parse_json_corpus(source: &str, expected_id: &str) -> Value {
    let root: Value = serde_json::from_str(source).expect("fixture corpus must be valid JSON");
    assert_eq!(string_field(&root, "schema_version"), "0.1.0");
    assert_eq!(string_field(&root, "corpus_id"), expected_id);
    assert_eq!(string_field(&root, "status"), "REFERENCE_FIXTURE_SPECIFICATION");
    assert!(
        field(&root, "purpose").as_str().is_some_and(|purpose| !purpose.trim().is_empty()),
        "fixture corpus must declare a purpose"
    );
    root
}

#[test]
fn fixture_bytes_match_the_pinned_sha256_manifest() {
    let manifest: Value =
        serde_json::from_str(CORPUS_MANIFEST).expect("corpus manifest must be valid JSON");
    assert_eq!(string_field(&manifest, "schema_version"), "0.1.0");
    assert_eq!(
        string_field(&manifest, "manifest_id"),
        "ORGAN-TRANSPORT-Q0-CORPUS-MANIFEST"
    );
    assert_eq!(
        string_field(&manifest, "status"),
        "PINNED_INPUTS_NOT_EXECUTION_RECEIPT"
    );

    let fixtures = field(&manifest, "fixtures")
        .as_array()
        .expect("manifest fixtures must be an array");
    assert_eq!(fixtures.len(), 2);
    let mut paths = HashSet::new();

    for fixture in fixtures {
        let path = string_field(fixture, "path");
        assert!(paths.insert(path.to_owned()), "duplicate manifest path {path}");
        let (source, expected_id, expected_case_ids): (&str, &str, &[&str]) = match path {
            "tube-reference-cases.v0.json" => {
                (TUBE_CORPUS, "ORGAN-TRANSPORT-Q0-TUBE", TUBE_CASE_IDS)
            }
            "network-reference-cases.v0.json" => {
                (NETWORK_CORPUS, "ORGAN-TRANSPORT-Q0-NETWORK", NETWORK_CASE_IDS)
            }
            other => panic!("unrecognized corpus path in manifest: {other}"),
        };
        let expected_cases = expected_case_ids.len();

        assert_eq!(string_field(fixture, "corpus_id"), expected_id);
        assert_eq!(string_field(fixture, "schema_version"), "0.1.0");
        assert_eq!(
            field(fixture, "byte_length").as_u64().expect("byte_length must be u64") as usize,
            source.len(),
            "{path}: byte length changed"
        );
        let digest = Sha256::digest(source.as_bytes());
        let digest_hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(
            digest_hex,
            string_field(fixture, "sha256"),
            "{path}: input digest changed; review and intentionally repin the corpus"
        );

        let pinned_case_ids = field(fixture, "case_ids")
            .as_array()
            .expect("case_ids must be an array");
        assert_eq!(pinned_case_ids.len(), expected_case_ids.len(), "{path}: manifest case IDs changed");
        for (index, pinned_id) in pinned_case_ids.iter().enumerate() {
            assert_eq!(
                pinned_id.as_str(),
                Some(expected_case_ids[index]),
                "{path}: manifest case ID/order differs at index {index}"
            );
        }

        let parsed = parse_json_corpus(source, expected_id);
        let parsed_cases = field(&parsed, "cases")
            .as_array()
            .expect("cases must be an array");
        assert_eq!(parsed_cases.len(), expected_cases, "{path}: case count changed");
        for (index, case) in parsed_cases.iter().enumerate() {
            assert_eq!(
                string_field(case, "id"),
                expected_case_ids[index],
                "{path}: fixture case ID/order differs at index {index}"
            );
        }
        assert_eq!(
            field(fixture, "case_count").as_u64().expect("case_count must be u64") as usize,
            expected_cases
        );
    }

    assert_eq!(
        paths,
        HashSet::from([
            "tube-reference-cases.v0.json".to_owned(),
            "network-reference-cases.v0.json".to_owned()
        ])
    );
}

#[test]
fn pinned_tube_fixture_corpus_executes_against_the_reference_kernel() {
    let root = parse_json_corpus(TUBE_CORPUS, "ORGAN-TRANSPORT-Q0-TUBE");
    let cases = field(&root, "cases")
        .as_array()
        .expect("cases must be an array");
    assert_eq!(cases.len(), TUBE_CASE_IDS.len(), "the pinned tube corpus changed unexpectedly");

    let mut case_ids = HashSet::new();
    for (index, case) in cases.iter().enumerate() {
        let id = string_field(case, "id");
        assert_eq!(id, TUBE_CASE_IDS[index], "tube fixture order/identity changed at index {index}");
        assert!(case_ids.insert(id.to_owned()), "duplicate tube fixture ID {id}");
        let expected = field(case, "expected");
        let expected_status = string_field(expected, "status");
        let inputs = field(case, "inputs");
        let spec = parse_tube_spec(inputs);

        match expected_status {
            "valid" => {
                let result = evaluate_tube(spec)
                    .unwrap_or_else(|error| panic!("{id}: unexpected reference error {error:?}"));
                let tolerance = field(case, "tolerance");
                let relative = number_field(tolerance, "relative");
                let absolute = number_field(tolerance, "absolute");
                assert_close(
                    result.hydraulic_resistance_pa_s_per_m3,
                    number_field(expected, "hydraulic_resistance_pa_s_per_m3"),
                    relative,
                    absolute,
                    &format!("{id}: resistance"),
                );
                assert_close(
                    result.volume_flow_m3_per_s,
                    number_field(expected, "volume_flow_m3_per_s"),
                    relative,
                    absolute,
                    &format!("{id}: flow"),
                );
            }
            "reject" => {
                let actual = evaluate_tube(spec)
                    .expect_err(&format!("{id}: negative control unexpectedly accepted"));
                assert_eq!(
                    error_label(actual),
                    string_field(expected, "error"),
                    "{id}: wrong fail-closed error"
                );
            }
            other => panic!("{id}: unsupported expected status {other:?}"),
        }
    }
    assert_eq!(case_ids.len(), TUBE_CASE_IDS.len());
}

#[test]
fn pinned_small_network_corpus_checks_edge_flows_and_junction_conservation() {
    let root = parse_json_corpus(NETWORK_CORPUS, "ORGAN-TRANSPORT-Q0-NETWORK");
    let cases = field(&root, "cases")
        .as_array()
        .expect("cases must be an array");
    assert_eq!(cases.len(), NETWORK_CASE_IDS.len(), "the pinned network corpus changed unexpectedly");

    let tolerance = field(&root, "tolerance");
    let relative = number_field(tolerance, "relative");
    let absolute = number_field(tolerance, "absolute");
    let mut case_ids = HashSet::new();

    for (case_index, case) in cases.iter().enumerate() {
        let case_id = string_field(case, "id");
        assert_eq!(
            case_id, NETWORK_CASE_IDS[case_index],
            "network fixture order/identity changed at index {case_index}"
        );
        assert!(
            case_ids.insert(case_id.to_owned()),
            "duplicate network fixture ID {case_id}"
        );

        let mut pressure_by_node: HashMap<String, f64> = HashMap::new();
        let mut kind_by_node: HashMap<String, String> = HashMap::new();
        for node in field(case, "nodes")
            .as_array()
            .expect("nodes must be an array")
        {
            let node_id = string_field(node, "id").to_owned();
            let pressure = number_field(node, "pressure_pa");
            assert!(pressure.is_finite(), "{case_id}/{node_id}: non-finite pressure");
            let kind = string_field(node, "kind").to_owned();
            assert!(
                ["source", "junction", "sink"].contains(&kind.as_str()),
                "{case_id}/{node_id}: unsupported node kind {kind:?}"
            );
            assert!(
                pressure_by_node.insert(node_id.clone(), pressure).is_none(),
                "{case_id}: duplicate node ID {node_id}"
            );
            kind_by_node.insert(node_id, kind);
        }

        let mut edge_ids = HashSet::new();
        let mut signed_incident_flows: HashMap<String, Vec<f64>> = HashMap::new();
        let mut absolute_incident_flow: HashMap<String, f64> = HashMap::new();
        let edges = field(case, "edges")
            .as_array()
            .expect("edges must be an array");
        assert!(!edges.is_empty(), "{case_id}: network cannot have zero edges");

        for edge in edges {
            let edge_id = string_field(edge, "id");
            assert!(
                edge_ids.insert(edge_id.to_owned()),
                "{case_id}: duplicate edge ID {edge_id}"
            );
            let from = string_field(edge, "from").to_owned();
            let to = string_field(edge, "to").to_owned();
            assert_ne!(from, to, "{case_id}/{edge_id}: self-loop is not part of Q0");
            let from_pressure = *pressure_by_node
                .get(&from)
                .unwrap_or_else(|| panic!("{case_id}/{edge_id}: unknown from-node {from}"));
            let to_pressure = *pressure_by_node
                .get(&to)
                .unwrap_or_else(|| panic!("{case_id}/{edge_id}: unknown to-node {to}"));
            let spec = TubeSpec {
                length_m: number_field(edge, "length_m"),
                radius_m: number_field(edge, "radius_m"),
                dynamic_viscosity_pa_s: number_field(edge, "dynamic_viscosity_pa_s"),
                pressure_drop_pa: from_pressure - to_pressure,
            };
            let result = evaluate_tube(spec).unwrap_or_else(|error| {
                panic!("{case_id}/{edge_id}: analytical edge evaluation failed: {error:?}")
            });
            let expected_flow = number_field(edge, "expected_flow_m3_per_s");
            assert_close(
                result.volume_flow_m3_per_s,
                expected_flow,
                relative,
                absolute,
                &format!("{case_id}/{edge_id}: edge flow"),
            );

            // Positive flow is from -> to. Positive signed incidence denotes flow
            // into a node, so the from-node receives -Q and the to-node receives +Q.
            signed_incident_flows
                .entry(from.clone())
                .or_default()
                .push(-result.volume_flow_m3_per_s);
            signed_incident_flows
                .entry(to.clone())
                .or_default()
                .push(result.volume_flow_m3_per_s);
            *absolute_incident_flow.entry(from).or_default() +=
                result.volume_flow_m3_per_s.abs();
            *absolute_incident_flow.entry(to).or_default() +=
                result.volume_flow_m3_per_s.abs();
        }

        let junctions = field(case, "junctions")
            .as_array()
            .expect("junctions must be an array");
        let declared_junctions: HashSet<String> = junctions
            .iter()
            .map(|junction| string_field(junction, "node_id").to_owned())
            .collect();
        let actual_junctions: HashSet<String> = kind_by_node
            .iter()
            .filter_map(|(node_id, kind)| (kind == "junction").then_some(node_id.clone()))
            .collect();
        assert_eq!(
            junctions.len(), declared_junctions.len(),
            "{case_id}: duplicate junction declaration"
        );
        assert_eq!(
            declared_junctions, actual_junctions,
            "{case_id}: every internal junction must be checked exactly once"
        );

        for junction in junctions {
            let node_id = string_field(junction, "node_id");
            let incident = signed_incident_flows
                .get(node_id)
                .unwrap_or_else(|| panic!("{case_id}/{node_id}: junction has no incident edges"));
            let actual_residual = junction_residual_m3_per_s(incident)
                .unwrap_or_else(|error| panic!("{case_id}/{node_id}: residual error {error:?}"));
            let expected_residual = number_field(junction, "expected_residual_m3_per_s");
            let flow_scale = absolute_incident_flow.get(node_id).copied().unwrap_or(0.0);
            let allowed = absolute + relative * flow_scale.max(expected_residual.abs());
            assert!(
                (actual_residual - expected_residual).abs() <= allowed,
                "{case_id}/{node_id}: mass-balance residual {actual_residual:.16e}, \
                 expected {expected_residual:.16e}, allowed {allowed:.3e}"
            );
        }
    }

    assert_eq!(case_ids.len(), NETWORK_CASE_IDS.len());
}

#[test]
fn fixture_claim_ceiling_does_not_promote_network_math_to_biological_evidence() {
    let root = parse_json_corpus(NETWORK_CORPUS, "ORGAN-TRANSPORT-Q0-NETWORK");
    let claims = field(field(&root, "model"), "unsupported_claims")
        .as_array()
        .expect("unsupported_claims must be an array");
    for required in [
        "endothelial barrier function",
        "oxygen transport",
        "cell viability",
        "organ function",
        "transplant safety",
    ] {
        assert!(
            claims.iter().any(|claim| claim.as_str() == Some(required)),
            "Q0 must explicitly disclaim {required:?}"
        );
    }
}
