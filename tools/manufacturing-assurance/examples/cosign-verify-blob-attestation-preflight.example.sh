#!/bin/sh
# Example cryptographic preflight only. This does not by itself create an
# admitted manufacturing receipt; validate the authenticated predicate after
# Cosign exits successfully, including repositoryId, scope, tier and materials.
set -eu

: "${COSIGN_BIN:?set COSIGN_BIN to the pinned Cosign v3.1.4 executable}"
: "${COSIGN_EXPECTED_SHA256:?set the approved platform-specific binary SHA-256}"
: "${BUNDLE_PATH:?set path to the Sigstore bundle}"
: "${TRUSTED_ROOT_PATH:?set path to the authenticated Sigstore TrustedRoot JSON}"
: "${TRUSTED_ROOT_EXPECTED_SHA256:?set digest from the authenticated root-update receipt}"
: "${ARTIFACT_PATH:?set path to exact subject artifact bytes}"
: "${PREDICATE_TYPE:?set exact organization-controlled predicate URI}"
: "${SIGNER_IDENTITY:?set exact Fulcio certificate subject}"
: "${OIDC_ISSUER:?set exact OIDC issuer}"
: "${WORKFLOW_REPOSITORY:?set exact owner/repository claim}"
: "${WORKFLOW_NAME:?set exact workflow identity claim}"
: "${WORKFLOW_REF:?set immutable protected release ref}"
: "${WORKFLOW_SHA:?set exact expected source commit SHA}"
: "${WORKFLOW_TRIGGER:?set expected workflow trigger}"

sha256_file() {
  sha256sum "$1" | cut -d ' ' -f 1
}

actual_cosign_sha256="$(sha256_file "$COSIGN_BIN")"
[ "$actual_cosign_sha256" = "$COSIGN_EXPECTED_SHA256" ] || {
  echo "REJECTED: Cosign binary digest mismatch" >&2
  exit 20
}

actual_root_sha256="$(sha256_file "$TRUSTED_ROOT_PATH")"
[ "$actual_root_sha256" = "$TRUSTED_ROOT_EXPECTED_SHA256" ] || {
  echo "REJECTED: trusted-root digest mismatch" >&2
  exit 21
}

# A separate TUF-aware update process MUST authenticate and freshness-check
# the root snapshot receipt before this preflight runs. This script only binds
# the root file to an expected digest; it does not implement TUF.

exec "$COSIGN_BIN" verify-blob-attestation \
  --bundle "$BUNDLE_PATH" \
  --trusted-root "$TRUSTED_ROOT_PATH" \
  --type "$PREDICATE_TYPE" \
  --certificate-identity "$SIGNER_IDENTITY" \
  --certificate-oidc-issuer "$OIDC_ISSUER" \
  --certificate-github-workflow-repository "$WORKFLOW_REPOSITORY" \
  --certificate-github-workflow-name "$WORKFLOW_NAME" \
  --certificate-github-workflow-ref "$WORKFLOW_REF" \
  --certificate-github-workflow-sha "$WORKFLOW_SHA" \
  --certificate-github-workflow-trigger "$WORKFLOW_TRIGGER" \
  --check-claims=true \
  "$ARTIFACT_PATH"
