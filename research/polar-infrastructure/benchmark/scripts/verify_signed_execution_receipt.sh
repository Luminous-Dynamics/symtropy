#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: verify_signed_execution_receipt.sh RECEIPT.json BUNDLE.sigstore.json PAIRED_INPUT.json POLICY.json" >&2
  exit 2
}

[[ $# -eq 4 ]] || usage
receipt=$1
bundle=$2
paired=$3
policy=$4

: "${SYMTROPY_TRUSTED_POLICY_SHA256:?set this from an out-of-band trusted configuration; do not derive it from the untrusted policy file}"

[[ -f "$receipt" && -f "$bundle" && -f "$paired" && -f "$policy" ]] || {
  echo "all receipt, bundle, paired input, and policy paths must be existing files" >&2
  exit 2
}

[[ "$SYMTROPY_TRUSTED_POLICY_SHA256" =~ ^[a-f0-9]{64}$ ]] || {
  echo "trusted policy digest must be 64 lowercase hex characters" >&2
  exit 1
}
actual_policy_sha=$(sha256sum "$policy" | cut -d ' ' -f 1)
[[ "$actual_policy_sha" == "$SYMTROPY_TRUSTED_POLICY_SHA256" ]] || {
  echo "policy digest does not match the out-of-band trust anchor" >&2
  exit 1
}

jq -e '
  .schema_version == "0.1.0" and
  .artifact_kind == "symtropy-execution-attestation-verification-policy" and
  .status == "design_only_not_enabled" and
  .scoring_authorized == false and
  .attestation_scheme == "sigstore_signed_blob" and
  .bundle_format.format_id == "sigstore_standardized_bundle_v0_3" and
  .bundle_format.media_type == "application/vnd.dev.sigstore.bundle.v0.3+json" and
  .bundle_format.legacy_bundle_allowed == false and
  .cosign.version_policy == "exact_reviewed_release" and
  .cosign.version == "v3.1.4" and
  .cosign.verify_command == "verify-blob" and
  .cosign.skip_transparency_log_allowed == false and
  .cosign.insecure_ignore_sct_allowed == false and
  .cosign.mutable_identity_regex_allowed == false and
  .trust_boundary.policy_digest_must_be_pinned_out_of_band == true
' "$policy" >/dev/null

jq -e '
  .mediaType == "application/vnd.dev.sigstore.bundle.v0.3+json"
' "$bundle" >/dev/null || {
  echo "unsupported Sigstore bundle mediaType; legacy/nonstandard bundle formats are rejected" >&2
  exit 1
}

jq -e '
  .schema_version == "0.1.0" and
  .artifact_kind == "symtropy-controller-execution-receipt" and
  .scoring_eligible == false and
  .execution_attestation.scheme == "none" and
  .execution_attestation.status == "not_implemented"
' "$receipt" >/dev/null || {
  echo "receipt is not a v0.1 design-only/ineligible receipt; scoring remains disabled" >&2
  exit 1
}

cosign_version_output=$(cosign version 2>&1)
grep -Eq 'GitVersion:[[:space:]]+v3\.1\.4([[:space:]]|$)' <<<"$cosign_version_output" || {
  echo "Cosign must be exactly v3.1.4 for this reviewed verification profile" >&2
  printf '%s\n' "$cosign_version_output" >&2
  exit 1
}

# Use the exact pinned identity and issuer. Never add --insecure-ignore-sct or a
# transparency-log bypass. The bundle signs the exact receipt bytes.
identity=$(jq -r '.cosign.certificate_identity' "$policy")
issuer=$(jq -r '.cosign.certificate_oidc_issuer' "$policy")
cosign verify-blob \
  --bundle "$bundle" \
  --certificate-identity "$identity" \
  --certificate-oidc-issuer "$issuer" \
  "$receipt"

python3 "$(dirname "$0")/verify_execution_receipt.py" "$receipt" "$paired"

echo "cryptographic signature check passed for exact receipt bytes; scoring_authorized=false; physical validity not inferred"
