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

pub const RUN_RECEIPT_SCHEMA_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunReceiptPayload {
    pub receipt_schema_version: String,
    pub digest_algorithm: String,
    pub execution_mode: String,
    pub package_version: String,
    /// Optional build provenance supplied by the caller via SYMTROPY_SOURCE_REVISION.
    /// It is informational unless verified against an independent source.
    pub source_revision: Option<String>,
    /// Runtime/build facts available without trusting external metadata.
    pub execution_environment: BTreeMap<String, String>,
    pub experiment_id: Option<String>,
    pub model_schema_version: Option<String>,
    pub unit_registry_version: Option<String>,
    /// SHA-256 of the exact source bytes read from the input artifact.
    pub input_artifact_sha256: String,
    /// SHA-256 of the parsed model's deterministic serde_json serialization.
    /// This is not a cross-version canonical-JSON claim.
    pub serialized_model_sha256: String,
    /// SHA-256 of the result payload's serde_json serialization before CLI-envelope rendering.
    pub output_payload_sha256: String,
    /// Execution parameters whose values are not already fully represented by the model.
    pub execution_settings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunReceipt {
    pub payload: RunReceiptPayload,
    /// SHA-256 of the serialized payload; this field is excluded to avoid recursive hashing.
    pub receipt_sha256: String,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

fn deterministic_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|error| format!("could not serialize receipt material: {error}"))
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
        let model_bytes = deterministic_json_bytes(model)?;
        let output_bytes = deterministic_json_bytes(output_payload)?;

        let source_revision = std::env::var("SYMTROPY_SOURCE_REVISION")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());

        let mut execution_environment = BTreeMap::from([
            ("architecture".to_owned(), std::env::consts::ARCH.to_owned()),
            ("operating_system".to_owned(), std::env::consts::OS.to_owned()),
            (
                "build_profile".to_owned(),
                (if cfg!(debug_assertions) { "debug" } else { "release" }).to_owned(),
            ),
        ]);
        if let Ok(version) = std::env::var("SYMTROPY_RUSTC_VERSION") {
            let version = version.trim();
            if !version.is_empty() {
                execution_environment.insert("rustc_version_reported_by_builder".to_owned(), version.to_owned());
            }
        }

        let payload = RunReceiptPayload {
            receipt_schema_version: RUN_RECEIPT_SCHEMA_VERSION.to_owned(),
            digest_algorithm: "sha256".to_owned(),
            execution_mode: execution_mode.into(),
            package_version: env!("CARGO_PKG_VERSION").to_owned(),
            source_revision,
            execution_environment,
            experiment_id,
            model_schema_version,
            unit_registry_version,
            input_artifact_sha256: sha256_hex(input_artifact_bytes),
            serialized_model_sha256: sha256_hex(&model_bytes),
            output_payload_sha256: sha256_hex(&output_bytes),
            execution_settings,
        };
        let receipt_bytes = deterministic_json_bytes(&payload)?;
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
        let expected = sha256_hex(&deterministic_json_bytes(&self.payload)?);
        Ok(expected == self.receipt_sha256)
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
        assert_eq!(value.payload.digest_algorithm, "sha256");
        assert_eq!(value.payload.unit_registry_version.as_deref(), Some("0.1.0"));
    }

    #[test]
    fn verification_fails_closed_for_empty_or_wrong_digest() {
        let mut value = receipt(br#"{"model":1}"#, &json!({"a": 1}), &json!({"answer": 42}));
        value.receipt_sha256 = "not-a-sha256-digest".to_owned();
        assert_eq!(value.verify(), Ok(false));
    }
}
