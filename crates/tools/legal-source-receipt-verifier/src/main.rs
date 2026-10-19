// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{env, fs, process};

const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct SourceUseRequest {
    schema_version: String,
    source_component_id: String,
    source_version_id: String,
    operation: String,
    purpose: String,
    actor_class: String,
    territory: String,
    as_of_utc: String,
    requested_retention_seconds: u64,
    observed_terms_sha256: Option<String>,
    observed_source_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Profile {
    schema_version: String,
    profile_id: String,
    profile_status: String,
    source: ProfileSource,
    terms: ProfileTerms,
    reviewer: Option<ProfileReviewer>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ProfileSource {
    component_id: String,
    version_id: String,
    content_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ProfileTerms {
    terms_sha256: Option<String>,
    review_status: String,
    verified_at_utc: Option<String>,
    valid_from_utc: Option<String>,
    valid_until_utc: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ProfileReviewer {
    decision: String,
    reviewed_at_utc: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SourceUseReceipt {
    receipt_version: String,
    evaluator_version: String,
    profile_id: String,
    profile_sha256: String,
    request_sha256: String,
    permission_decision: Option<String>,
    admission_decision: String,
    reason_code: String,
    permission_grant_id: Option<String>,
    expected_terms_sha256: Option<String>,
    observed_terms_sha256: Option<String>,
    expected_source_sha256: Option<String>,
    observed_source_sha256: Option<String>,
    request: SourceUseRequest,
    claim_ceiling: String,
}

#[derive(Clone, Debug, Serialize)]
struct VerificationReport {
    verifier_version: &'static str,
    receipt_version: String,
    profile_id: String,
    profile_sha256: String,
    request_sha256: String,
    expected_operation: String,
    operation_matches: bool,
    structural_consistency: &'static str,
    claimed_permission_decision: Option<String>,
    claimed_admission_decision: String,
    authentication_status: &'static str,
    authorization_decision: &'static str,
    may_proceed: bool,
    reason_code: &'static str,
    claim_ceiling: &'static str,
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => process::exit(1),
        Err(error) => {
            eprintln!("legal-source-receipt-verifier: {error}");
            process::exit(2);
        }
    }
}

fn run() -> Result<bool, String> {
    let mut args = env::args().skip(1);
    let command = args.next().ok_or_else(usage)?;
    match command.as_str() {
        "verify" => {
            let profile_path = args.next().ok_or_else(usage)?;
            let request_path = args.next().ok_or_else(usage)?;
            let receipt_path = args.next().ok_or_else(usage)?;
            let expected_operation = args.next().ok_or_else(usage)?;
            if args.next().is_some() {
                return Err(usage());
            }
            let profile_raw = fs::read(&profile_path)
                .map_err(|error| format!("cannot read {profile_path}: {error}"))?;
            let request_raw = fs::read(&request_path)
                .map_err(|error| format!("cannot read {request_path}: {error}"))?;
            let receipt_raw = fs::read(&receipt_path)
                .map_err(|error| format!("cannot read {receipt_path}: {error}"))?;
            let report = verify_receipt(
                &profile_raw,
                &request_raw,
                &receipt_raw,
                &expected_operation,
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .map_err(|error| format!("cannot serialize verification report: {error}"))?
            );
            // Structural consistency is not authentication or authorization.
            Ok(true)
        }
        _ => Err(usage()),
    }
}

fn usage() -> String {
    [
        "usage: legal-source-receipt-verifier verify <profile.json> <request.json> <receipt.json> <expected-operation>",
    ]
    .join("\n")
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn verify_receipt(
    profile_raw: &[u8],
    request_raw: &[u8],
    receipt_raw: &[u8],
    expected_operation: &str,
) -> Result<VerificationReport, String> {
    let profile: Profile = serde_json::from_slice(profile_raw)
        .map_err(|error| format!("invalid profile JSON: {error}"))?;
    let request: SourceUseRequest = serde_json::from_slice(request_raw)
        .map_err(|error| format!("invalid request JSON: {error}"))?;
    let receipt: SourceUseReceipt = serde_json::from_slice(receipt_raw)
        .map_err(|error| format!("invalid receipt JSON: {error}"))?;

    if profile.schema_version != "0.1.0"
        || request.schema_version != "0.1.0"
        || receipt.receipt_version != "0.1.0"
    {
        return Err("unsupported profile, request, or receipt schema version".to_string());
    }
    if receipt.profile_sha256 != sha256_hex(profile_raw) {
        return Err("receipt profile digest does not bind exact profile bytes".to_string());
    }
    if receipt.request_sha256 != sha256_hex(request_raw) {
        return Err("receipt request digest does not bind exact request bytes".to_string());
    }
    if receipt.profile_id != profile.profile_id {
        return Err("receipt profile_id does not match profile".to_string());
    }
    if receipt.request != request {
        return Err("receipt-embedded request differs from the supplied request".to_string());
    }
    if request.operation != expected_operation {
        return Err(format!(
            "request operation {} does not match required operation {}",
            request.operation, expected_operation
        ));
    }
    if receipt.expected_terms_sha256 != profile.terms.terms_sha256
        || receipt.observed_terms_sha256 != request.observed_terms_sha256
    {
        return Err("receipt terms digests do not match the profile and request".to_string());
    }
    if receipt.expected_source_sha256 != profile.source.content_sha256
        || receipt.observed_source_sha256 != request.observed_source_sha256
    {
        return Err("receipt source digests do not match the profile and request".to_string());
    }
    if request.source_component_id != profile.source.component_id
        || request.source_version_id != profile.source.version_id
    {
        return Err("request source component/version does not match the profile".to_string());
    }

    let terms_match = profile.terms.terms_sha256 == request.observed_terms_sha256;
    if !terms_match
        && (receipt.admission_decision != "unknown"
            || receipt.reason_code != "terms_digest_mismatch_requires_rereview")
    {
        return Err("terms mismatch is not represented as unknown with the required reason".to_string());
    }

    let source_match = match (
        profile.source.content_sha256.as_deref(),
        request.observed_source_sha256.as_deref(),
    ) {
        (Some(expected), Some(observed)) => expected == observed,
        (None, None) => true,
        _ => false,
    };
    if !source_match {
        let expected_reason = match (
            profile.source.content_sha256.as_deref(),
            request.observed_source_sha256.as_deref(),
        ) {
            (Some(_), None) => "observed_source_digest_missing",
            (Some(_), Some(_)) => "source_digest_mismatch",
            (None, Some(_)) => "profile_source_digest_unavailable",
            (None, None) => unreachable!(),
        };
        if receipt.admission_decision != "unknown" || receipt.reason_code != expected_reason {
            return Err(format!(
                "source digest mismatch must remain unknown with reason {expected_reason}"
            ));
        }
    }

    let reviewer_approved = profile
        .reviewer
        .as_ref()
        .is_some_and(|reviewer| reviewer.decision == "approved");
    if profile.profile_status == "approved" && !reviewer_approved {
        return Err("profile says approved but reviewer approval is missing".to_string());
    }
    if profile
        .reviewer
        .as_ref()
        .is_some_and(|reviewer| reviewer.reviewed_at_utc > request.as_of_utc)
        && receipt.reason_code != "profile_review_after_request_time"
    {
        return Err("receipt does not reflect review occurring after request time".to_string());
    }
    if profile
        .terms
        .verified_at_utc
        .as_deref()
        .is_some_and(|verified_at| verified_at > request.as_of_utc.as_str())
        && receipt.reason_code != "terms_verified_after_request_time"
    {
        return Err("receipt does not reflect terms verified after request time".to_string());
    }

    if receipt.admission_decision == "admitted" {
        if receipt.permission_decision.as_deref() != Some("allowed") {
            return Err("admitted receipt must claim permission_decision allowed".to_string());
        }
        if !terms_match || !source_match {
            return Err("receipt cannot be admitted with mismatched source/terms digests".to_string());
        }
        if receipt.reason_code != "operation_explicitly_permitted_within_scope" {
            return Err("admitted receipt has unexpected reason code".to_string());
        }
        if receipt.permission_grant_id.is_none() {
            return Err("admitted receipt must identify the permission grant".to_string());
        }
    } else if receipt.admission_decision == "unknown"
        && receipt.permission_decision.as_deref() == Some("denied")
    {
        return Err("unknown admission cannot claim permission decision denied".to_string());
    } else if !["denied", "unknown"].contains(&receipt.admission_decision.as_str()) {
        return Err(format!(
            "unsupported admission_decision {}",
            receipt.admission_decision
        ));
    }

    Ok(VerificationReport {
        verifier_version: PACKAGE_VERSION,
        receipt_version: receipt.receipt_version,
        profile_id: receipt.profile_id,
        profile_sha256: receipt.profile_sha256,
        request_sha256: receipt.request_sha256,
        expected_operation: expected_operation.to_string(),
        operation_matches: true,
        structural_consistency: "consistent",
        claimed_permission_decision: receipt.permission_decision,
        claimed_admission_decision: receipt.admission_decision,
        authentication_status: "unsigned_untrusted",
        authorization_decision: "unknown",
        may_proceed: false,
        reason_code: "unsigned_receipt_is_not_authorization",
        claim_ceiling: "Structural consistency only. Unsigned receipts do not authenticate the issuer and never authorize production use.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_profile() -> Vec<u8> {
        include_bytes!("../fixtures/fictional-source-profile-v0.1.json").to_vec()
    }

    fn fixture_request() -> Vec<u8> {
        include_bytes!("../fixtures/benchmark-evaluation-request-v0.1.json").to_vec()
    }

    fn make_receipt(
        profile_raw: &[u8],
        request_raw: &[u8],
        decision: &str,
        permission: Option<&str>,
        reason: &str,
    ) -> Vec<u8> {
        let profile: Profile = serde_json::from_slice(profile_raw).expect("profile parses");
        let request: SourceUseRequest = serde_json::from_slice(request_raw).expect("request parses");
        let receipt = SourceUseReceipt {
            receipt_version: "0.1.0".to_string(),
            evaluator_version: "0.1.0".to_string(),
            profile_id: profile.profile_id,
            profile_sha256: sha256_hex(profile_raw),
            request_sha256: sha256_hex(request_raw),
            permission_decision: permission.map(str::to_string),
            admission_decision: decision.to_string(),
            reason_code: reason.to_string(),
            permission_grant_id: Some("grant-benchmark-evaluation".to_string()),
            expected_terms_sha256: profile.terms.terms_sha256,
            observed_terms_sha256: request.observed_terms_sha256.clone(),
            expected_source_sha256: profile.source.content_sha256,
            observed_source_sha256: request.observed_source_sha256.clone(),
            request,
            claim_ceiling: "Synthetic test only".to_string(),
        };
        serde_json::to_vec_pretty(&receipt).expect("receipt serializes")
    }

    #[test]
    fn matching_unsigned_receipt_is_consistent_but_never_authorizes() {
        let profile = fixture_profile();
        let request = fixture_request();
        let receipt = make_receipt(
            &profile,
            &request,
            "admitted",
            Some("allowed"),
            "operation_explicitly_permitted_within_scope",
        );
        let report =
            verify_receipt(&profile, &request, &receipt, "benchmark_evaluation").expect("verifies");
        assert_eq!(report.structural_consistency, "consistent");
        assert_eq!(report.authentication_status, "unsigned_untrusted");
        assert_eq!(report.authorization_decision, "unknown");
        assert!(!report.may_proceed);
    }

    #[test]
    fn exact_request_bytes_are_digest_bound() {
        let profile = fixture_profile();
        let request = fixture_request();
        let receipt = make_receipt(
            &profile,
            &request,
            "admitted",
            Some("allowed"),
            "operation_explicitly_permitted_within_scope",
        );
        let mut tampered = request.clone();
        tampered.push(b' ');
        assert!(verify_receipt(&profile, &tampered, &receipt, "benchmark_evaluation").is_err());
    }

    #[test]
    fn operation_substitution_is_rejected() {
        let profile = fixture_profile();
        let request = fixture_request();
        let receipt = make_receipt(
            &profile,
            &request,
            "admitted",
            Some("allowed"),
            "operation_explicitly_permitted_within_scope",
        );
        assert!(verify_receipt(&profile, &request, &receipt, "model_training").is_err());
    }

    #[test]
    fn fake_profile_digest_is_rejected() {
        let profile = fixture_profile();
        let request = fixture_request();
        let mut receipt: serde_json::Value =
            serde_json::from_slice(&make_receipt(
                &profile,
                &request,
                "admitted",
                Some("allowed"),
                "operation_explicitly_permitted_within_scope",
            ))
            .expect("receipt parses");
        receipt["profile_sha256"] =
            serde_json::Value::String("0000000000000000000000000000000000000000000000000000000000000000".to_string());
        let bytes = serde_json::to_vec(&receipt).expect("receipt serializes");
        assert!(verify_receipt(&profile, &request, &bytes, "benchmark_evaluation").is_err());
    }

    #[test]
    fn terms_mismatch_can_only_be_consistent_as_unknown() {
        let profile = fixture_profile();
        let mut request: serde_json::Value =
            serde_json::from_slice(&fixture_request()).expect("request parses");
        request["observed_terms_sha256"] =
            serde_json::Value::String("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string());
        let request_raw = serde_json::to_vec_pretty(&request).expect("request serializes");
        let receipt = make_receipt(
            &profile,
            &request_raw,
            "unknown",
            Some("unknown"),
            "terms_digest_mismatch_requires_rereview",
        );
        let report = verify_receipt(&profile, &request_raw, &receipt, "benchmark_evaluation")
            .expect("consistent unknown result");
        assert_eq!(report.authorization_decision, "unknown");
        assert!(!report.may_proceed);
    }

    #[test]
    fn admitted_receipt_cannot_override_a_terms_mismatch() {
        let profile = fixture_profile();
        let mut request: serde_json::Value =
            serde_json::from_slice(&fixture_request()).expect("request parses");
        request["observed_terms_sha256"] =
            serde_json::Value::String("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string());
        let request_raw = serde_json::to_vec_pretty(&request).expect("request serializes");
        let receipt = make_receipt(
            &profile,
            &request_raw,
            "admitted",
            Some("allowed"),
            "operation_explicitly_permitted_within_scope",
        );
        assert!(verify_receipt(&profile, &request_raw, &receipt, "benchmark_evaluation").is_err());
    }

    #[test]
    fn source_digest_mismatch_cannot_be_admitted() {
        let profile = fixture_profile();
        let mut request: serde_json::Value =
            serde_json::from_slice(&fixture_request()).expect("request parses");
        request["observed_source_sha256"] =
            serde_json::Value::String("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_string());
        let request_raw = serde_json::to_vec_pretty(&request).expect("request serializes");
        let receipt = make_receipt(
            &profile,
            &request_raw,
            "unknown",
            Some("unknown"),
            "source_digest_mismatch",
        );
        let report = verify_receipt(&profile, &request_raw, &receipt, "benchmark_evaluation")
            .expect("consistent unknown result");
        assert_eq!(report.authorization_decision, "unknown");
        assert!(!report.may_proceed);
    }

    #[test]
    fn admission_unknown_cannot_claim_permission_denied() {
        let profile = fixture_profile();
        let request = fixture_request();
        let receipt = make_receipt(
            &profile,
            &request,
            "unknown",
            Some("denied"),
            "some_reason",
        );
        assert!(verify_receipt(&profile, &request, &receipt, "benchmark_evaluation").is_err());
    }

    #[test]
    fn unsupported_receipt_schema_is_rejected() {
        let profile = fixture_profile();
        let request = fixture_request();
        let mut receipt: serde_json::Value =
            serde_json::from_slice(&make_receipt(
                &profile,
                &request,
                "admitted",
                Some("allowed"),
                "operation_explicitly_permitted_within_scope",
            ))
            .expect("receipt parses");
        receipt["receipt_version"] = serde_json::Value::String("9.9.9".to_string());
        let bytes = serde_json::to_vec(&receipt).expect("receipt serializes");
        assert!(verify_receipt(&profile, &request, &bytes, "benchmark_evaluation").is_err());
    }
}
