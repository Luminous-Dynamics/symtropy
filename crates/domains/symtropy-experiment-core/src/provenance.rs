// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Content-addressed execution receipts for scientific experiment runs.
//!
//! A receipt records what inputs/model/output and execution settings were used. It is a
//! reproducibility and integrity aid, not a signature, trusted timestamp, validation claim, or
//! proof that a model is scientifically correct.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Receipt schema 0.3.0 binds digests to RFC 8785 JCS and adds build-input digests.
pub const RUN_RECEIPT_SCHEMA_VERSION: &str = "0.3.0";
pub const RECEIPT_CANONICALIZATION_ALGORITHM: &str = "RFC8785-JCS";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunReceiptPayload {
    pub receipt_schema_version: String,
    pub digest_algorithm: String,
    /// Canonical JSON byte contract used for model, output, and receipt-payload digests.
    pub canonicalization_algorithm: String,
    pub execution_mode: String,
    pub package_version: String,
    /// Build-time revision supplied through SYMTROPY_SOURCE_REVISION, if valid.
    /// It is still an assertion by the build environment, not an authenticated attestation.
    pub source_revision: Option<String>,
    /// SHA-256 of the Cargo.lock bytes present when this binary was built, when available.
    pub cargo_lock_sha256: Option<String>,
    /// SHA-256 of this package's Cargo.toml bytes present when this binary was built.
    pub package_manifest_sha256: Option<String>,
    /// Build and runtime facts; build claims are not independently authenticated.
    pub execution_environment: BTreeMap<String, String>,
    pub experiment_id: Option<String>,
    pub model_schema_version: Option<String>,
    pub unit_registry_version: Option<String>,
    /// SHA-256 of the exact source bytes read from the input artifact.
    pub input_artifact_sha256: String,
    /// SHA-256 of the parsed model serialized using RFC 8785 JCS.
    pub serialized_model_sha256: String,
    /// SHA-256 of the result payload's RFC 8785 JCS bytes before CLI-envelope rendering.
    pub output_payload_sha256: String,
    /// Execution parameters whose values are not already fully represented by the model.
    pub execution_settings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunReceipt {
    pub payload: RunReceiptPayload,
    /// SHA-256 of the RFC 8785 JCS bytes of the payload; this field is excluded from that hash.
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunReceiptVerification {
    pub receipt_digest_valid: bool,
    pub input_artifact_digest_matches: bool,
    pub serialized_model_digest_matches: bool,
    pub output_payload_digest_matches: bool,
}

impl RunReceiptVerification {
    pub fn is_valid(&self) -> bool {
        self.receipt_digest_valid
            && self.input_artifact_digest_matches
            && self.serialized_model_digest_matches
            && self.output_payload_digest_matches
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

/// Serialize digest inputs according to RFC 8785 (JCS), rather than a Rust-specific struct order.
fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    serde_json_canonicalizer::to_vec(value)
        .map_err(|error| format!("could not canonicalize receipt material as RFC 8785 JCS: {error}"))
}

fn nonempty_build_value(value: Option<&'static str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn insert_build_environment(
    environment: &mut BTreeMap<String, String>,
    key: &str,
    value: Option<String>,
) {
    if let Some(value) = value {
        environment.insert(key.to_owned(), value);
    }
}

impl RunReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn build<M: Serialize, O: Serialize>(
        execution_mode: impl Into<String>,
        input_artifact_bytes: &[u8],
        model: &M,
        output_payload: &O,
        experiment_id: Option<String>,
        model_schema_version: Option<String>,
        unit_registry_version: Option<String>,
        execution_settings: BTreeMap<String, String>,
    ) -> Result<Self, String> {
        let model_bytes = canonical_json_bytes(model)?;
        let output_bytes = canonical_json_bytes(output_payload)?;

        // These values are embedded by build.rs, not read from the runtime environment.
        let source_revision = nonempty_build_value(option_env!("SYMTROPY_BUILD_SOURCE_REVISION"));
        let cargo_lock_sha256 = nonempty_build_value(option_env!("SYMTROPY_BUILD_CARGO_LOCK_SHA256"));
        let package_manifest_sha256 =
            nonempty_build_value(option_env!("SYMTROPY_BUILD_PACKAGE_MANIFEST_SHA256"));

        let mut execution_environment = BTreeMap::from([
            ("architecture".to_owned(), std::env::consts::ARCH.to_owned()),
            ("operating_system".to_owned(), std::env::consts::OS.to_owned()),
            (
                "build_profile".to_owned(),
                (if cfg!(debug_assertions) { "debug" } else { "release" }).to_owned(),
            ),
        ]);
        insert_build_environment(
            &mut execution_environment,
            "rustc_version_at_build",
            nonempty_build_value(option_env!("SYMTROPY_BUILD_RUSTC_VERSION")),
        );
        insert_build_environment(
            &mut execution_environment,
            "cargo_target_triple",
            nonempty_build_value(option_env!("SYMTROPY_BUILD_TARGET")),
        );
        insert_build_environment(
            &mut execution_environment,
            "cargo_host_triple",
            nonempty_build_value(option_env!("SYMTROPY_BUILD_HOST")),
        );

        let payload = RunReceiptPayload {
            receipt_schema_version: RUN_RECEIPT_SCHEMA_VERSION.to_owned(),
            digest_algorithm: "sha256".to_owned(),
            canonicalization_algorithm: RECEIPT_CANONICALIZATION_ALGORITHM.to_owned(),
            execution_mode: execution_mode.into(),
            package_version: env!("CARGO_PKG_VERSION").to_owned(),
            source_revision,
            cargo_lock_sha256,
            package_manifest_sha256,
            execution_environment,
            experiment_id,
            model_schema_version,
            unit_registry_version,
            input_artifact_sha256: sha256_hex(input_artifact_bytes),
            serialized_model_sha256: sha256_hex(&model_bytes),
            output_payload_sha256: sha256_hex(&output_bytes),
            execution_settings,
        };
        let receipt_bytes = canonical_json_bytes(&payload)?;
        Ok(Self {
            payload,
            receipt_sha256: sha256_hex(&receipt_bytes),
        })
    }

    /// Check the receipt's internal SHA-256 digest.
    ///
    /// This detects accidental or deliberate field modification only if the expected digest is
    /// retained independently. It does not authenticate the writer or make unsigned data trusted.
    pub fn verify(&self) -> Result<bool, String> {
        // Fail closed for unknown digest contracts even if a self-consistent hash is supplied.
        if self.payload.receipt_schema_version != RUN_RECEIPT_SCHEMA_VERSION
            || self.payload.digest_algorithm != "sha256"
            || self.payload.canonicalization_algorithm != RECEIPT_CANONICALIZATION_ALGORITHM
        {
            return Ok(false);
        }
        let expected = sha256_hex(&canonical_json_bytes(&self.payload)?);
        Ok(expected == self.receipt_sha256)
    }

    /// Verify the receipt and the supplied byte-for-byte input/model/output representations.
    ///
    /// The caller must serialize the parsed model and output payload with the same representation
    /// used when the receipt was produced. This function reports each link separately so a
    /// consumer can diagnose which artifact no longer matches.
    pub fn verify_artifact_bytes(
        &self,
        input_artifact_bytes: &[u8],
        serialized_model_bytes: &[u8],
        output_payload_bytes: &[u8],
    ) -> Result<RunReceiptVerification, String> {
        Ok(RunReceiptVerification {
            receipt_digest_valid: self.verify()?,
            input_artifact_digest_matches: sha256_hex(input_artifact_bytes)
                == self.payload.input_artifact_sha256,
            serialized_model_digest_matches: sha256_hex(serialized_model_bytes)
                == self.payload.serialized_model_sha256,
            output_payload_digest_matches: sha256_hex(output_payload_bytes)
                == self.payload.output_payload_sha256,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn settings() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("numeric_format".to_owned(), "IEEE-754 binary64".to_owned()),
            ("solver".to_owned(), "arithmetic-dag-v1".to_owned()),
        ])
    }

    fn receipt(input: &[u8], model: &serde_json::Value, output: &serde_json::Value) -> RunReceipt {
        RunReceipt::build(
            "run",
            input,
            model,
            output,
            Some("experiment:test".to_owned()),
            Some("0.1".to_owned()),
            Some("0.1.0".to_owned()),
            settings(),
        )
        .expect("receipt should serialize")
    }

    #[test]
    fn sha256_matches_a_published_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn unchanged_material_produces_same_receipt_digest() {
        let model = json!({"a": 1, "b": [2, 3]});
        let output = json!({"answer": 42});
        let first = receipt(br#"{"model":1}"#, &model, &output);
        let second = receipt(br#"{"model":1}"#, &model, &output);
        assert_eq!(first, second);
        assert_eq!(first.verify(), Ok(true));
    }

    #[test]
    fn input_model_output_or_execution_settings_change_the_receipt_digest() {
        let model = json!({"a": 1});
        let output = json!({"answer": 42});
        let baseline = receipt(br#"{"model":1}"#, &model, &output);
        assert_ne!(baseline.receipt_sha256, receipt(br#"{"model":2}"#, &model, &output).receipt_sha256);
        assert_ne!(baseline.receipt_sha256, receipt(br#"{"model":1}"#, &json!({"a": 2}), &output).receipt_sha256);
        assert_ne!(baseline.receipt_sha256, receipt(br#"{"model":1}"#, &model, &json!({"answer": 43})).receipt_sha256);

        let different_settings = BTreeMap::from([
            ("numeric_format".to_owned(), "decimal-arbitrary-precision".to_owned()),
            ("solver".to_owned(), "different-solver".to_owned()),
        ]);
        let changed_settings = RunReceipt::build(
            "run",
            br#"{"model":1}"#,
            &model,
            &output,
            Some("experiment:test".to_owned()),
            Some("0.1".to_owned()),
            Some("0.1.0".to_owned()),
            different_settings,
        ).unwrap();
        assert_ne!(baseline.receipt_sha256, changed_settings.receipt_sha256);
    }

    #[test]
    fn editing_payload_without_recomputing_digest_is_detected() {
        let mut value = receipt(br#"{"model":1}"#, &json!({"a": 1}), &json!({"answer": 42}));
        value.payload.execution_mode = "forged".to_owned();
        assert_eq!(value.verify(), Ok(false));
    }

    #[test]
    fn receipt_distinguishes_model_serialization_from_raw_input_bytes() {
        let model = json!({"a": 1});
        let output = json!({"answer": 42});
        let compact = receipt(br#"{"a":1}"#, &model, &output);
        let spaced = receipt(br#"{ "a": 1 }"#, &model, &output);
        assert_ne!(compact.payload.input_artifact_sha256, spaced.payload.input_artifact_sha256);
        assert_eq!(compact.payload.serialized_model_sha256, spaced.payload.serialized_model_sha256);
    }

    #[test]
    fn receipt_is_versioned_and_states_its_digest_algorithm() {
        let value = receipt(br#"{"model":1}"#, &json!({"a": 1}), &json!({"answer": 42}));
        assert_eq!(value.payload.receipt_schema_version, RUN_RECEIPT_SCHEMA_VERSION);
        assert_eq!(value.payload.receipt_schema_version, "0.3.0");
        assert_eq!(value.payload.digest_algorithm, "sha256");
        assert_eq!(
            value.payload.canonicalization_algorithm,
            RECEIPT_CANONICALIZATION_ALGORITHM
        );
        assert_eq!(value.payload.unit_registry_version.as_deref(), Some("0.1.0"));
    }

    #[test]
    fn workspace_build_embeds_lockfile_and_package_manifest_digests() {
        let value = receipt(br#"{"model":1}"#, &json!({"a": 1}), &json!({"answer": 42}));
        assert!(value
            .payload
            .cargo_lock_sha256
            .as_deref()
            .is_some_and(|digest| digest.len() == 64));
        assert!(value
            .payload
            .package_manifest_sha256
            .as_deref()
            .is_some_and(|digest| digest.len() == 64));
    }

    #[test]
    fn artifact_verification_reports_each_digest_link_separately() {
        let input = br#"{"a":1}"#;
        let model = json!({"a": 1});
        let output = json!({"answer": 42});
        let value = receipt(input, &model, &output);
        let model_bytes = canonical_json_bytes(&model).unwrap();
        let output_bytes = canonical_json_bytes(&output).unwrap();

        let valid = value.verify_artifact_bytes(input, &model_bytes, &output_bytes).unwrap();
        assert!(valid.is_valid());

        let bad_input = value.verify_artifact_bytes(br#"{"a":2}"#, &model_bytes, &output_bytes).unwrap();
        assert!(!bad_input.input_artifact_digest_matches);
        assert!(bad_input.serialized_model_digest_matches);
        assert!(bad_input.output_payload_digest_matches);

        let bad_output = value.verify_artifact_bytes(input, &model_bytes, br#"{"answer":43}"#).unwrap();
        assert!(bad_output.receipt_digest_valid);
        assert!(!bad_output.output_payload_digest_matches);
        assert!(!bad_output.is_valid());
    }

    #[test]
    fn independent_known_answer_fixture_matches_all_precomputed_digests() {
        let input_bytes = include_bytes!("../fixtures/receipt-known-answer-input.json");
        let output_bytes = include_bytes!("../fixtures/receipt-known-answer-output.json");
        let model: serde_json::Value = serde_json::from_slice(input_bytes).unwrap();
        let output: serde_json::Value = serde_json::from_slice(output_bytes).unwrap();
        let model_bytes = canonical_json_bytes(&model).unwrap();
        let output_serialized = canonical_json_bytes(&output).unwrap();
        let receipt: RunReceipt = serde_json::from_str(include_str!(
            "../fixtures/receipt-known-answer.json"
        ))
        .unwrap();

        // These expected hashes were independently generated with Python hashlib, not by
        // RunReceipt::build or this module's sha256_hex helper.
        assert_eq!(
            sha256_hex(input_bytes),
            "cc014e7a9694d0bf7a836dd7ec8aa19552424111024d60822ad09a637a503e84"
        );
        assert_eq!(
            sha256_hex(&model_bytes),
            "71d93acd678db5ef8385b1e21984caff6547b22b2fe1c79753c4818d55f1841b"
        );
        assert_eq!(
            sha256_hex(&output_serialized),
            "8cd5aa75a94d38bbd330f9942a42e6136e9759125c09f2c23da95964621460e6"
        );
        assert_eq!(
            receipt.receipt_sha256,
            "b0b045ab62955c0e0342ee9746447dee39181c32318920ab13ac40b15c871775"
        );
        assert_eq!(receipt.verify(), Ok(true));
        assert!(receipt
            .verify_artifact_bytes(input_bytes, &model_bytes, &output_serialized)
            .unwrap()
            .is_valid());

        // The exact input hash intentionally includes formatting and the final newline.
        let mut changed_input = input_bytes.to_vec();
        changed_input.push(b' ');
        let altered = receipt
            .verify_artifact_bytes(&changed_input, &model_bytes, &output_serialized)
            .unwrap();
        assert!(!altered.input_artifact_digest_matches);
        assert!(altered.serialized_model_digest_matches);
        assert!(altered.output_payload_digest_matches);
        assert!(!altered.is_valid());
    }

    #[test]
    fn rfc8785_canonical_json_sorts_object_properties_recursively() {
        let value = serde_json::json!({
            "c": true,
            "nested": {"z": 1, "a": 2},
            "b": false,
            "a": "Hello!"
        });
        let bytes = canonical_json_bytes(&value).unwrap();
        assert_eq!(
            bytes,
            br#"{"a":"Hello!","b":false,"c":true,"nested":{"a":2,"z":1}}"#
        );
    }

    #[test]
    fn rfc8785_canonical_json_uses_ecmascript_number_formatting_vectors() {
        assert_eq!(canonical_json_bytes(&1e23_f64).unwrap(), b"1e+23");
        assert_eq!(canonical_json_bytes(&1e-7_f64).unwrap(), b"1e-7");
        assert_eq!(canonical_json_bytes(&-0.0_f64).unwrap(), b"0");
        assert_eq!(
            canonical_json_bytes(&5e-324_f64).unwrap(),
            b"5e-324"
        );
    }

    #[test]
    fn receipt_schema_identifies_jcs_hashing_contract() {
        let value = receipt(br#"{"model":1}"#, &serde_json::json!({"a": 1}), &serde_json::json!({"answer": 42}));
        assert_eq!(value.payload.receipt_schema_version, "0.2.0");
        assert_eq!(value.payload.canonicalization_algorithm, "RFC8785-JCS");
        assert_ne!(value.payload.canonicalization_algorithm, "serde_json::to_vec");
    }

    #[test]
    fn verifier_rejects_unknown_serialization_contract_even_with_matching_digest() {
        let mut value = receipt(br#"{"model":1}"#, &json!({"a": 1}), &json!({"answer": 42}));
        value.payload.canonicalization_algorithm = "made-up-json-format".to_owned();
        value.receipt_sha256 = sha256_hex(&canonical_json_bytes(&value.payload).unwrap());
        assert_eq!(value.verify(), Ok(false));

        value = receipt(br#"{"model":1}"#, &json!({"a": 1}), &json!({"answer": 42}));
        value.payload.receipt_schema_version = "999.0.0".to_owned();
        value.receipt_sha256 = sha256_hex(&canonical_json_bytes(&value.payload).unwrap());
        assert_eq!(value.verify(), Ok(false));
    }

    #[test]
    fn verification_fails_closed_for_empty_or_wrong_digest() {
        let mut value = receipt(br#"{"model":1}"#, &json!({"a": 1}), &json!({"answer": 42}));
        value.receipt_sha256 = "not-a-sha256-digest".to_owned();
        assert_eq!(value.verify(), Ok(false));
    }
}
