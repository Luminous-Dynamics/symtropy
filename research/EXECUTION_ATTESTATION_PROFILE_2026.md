# Symtropy execution-attestation profile, 2026

**Status:** verification policy design only; scoring remains disabled.  
**Policy:** `fixtures/execution-attestation-verification-policy-2026.json`  
**Schema:** `schemas/execution-attestation-verification-policy-0.1.0.schema.json`  
**Wrapper:** `scripts/verify_signed_execution_receipt.sh`

## 1. Design choice

The first attestation profile uses a Sigstore-signed blob for the exact UTF-8 bytes of the immutable execution receipt. The receipt body already carries the experiment's structured fields. The detached standardized bundle signs those exact receipt bytes, so no self-referential “the receipt contains its own signature digest” field is needed.

This verifies *who signed these exact bytes under the configured Sigstore trust policy*. It does not by itself prove that the signing job genuinely ran the simulator. The trusted signer must generate the receipt inside the job that performs the execution; it must not accept arbitrary external receipts and sign them just because their JSON is well formed.

The policy pins the expected GitHub Actions workflow identity and issuer to:

- Certificate identity: `https://github.com/Luminous-Dynamics/symtropy/.github/workflows/controller-execution-attestation.yml@refs/heads/main`
- OIDC issuer: `https://token.actions.githubusercontent.com`

The named protected-main attestation workflow is a future trust boundary, not implemented by this change. Therefore the policy is explicitly `design_only_not_enabled`, and every v0.1 receipt remains `scoring_eligible=false`.

## 2. Bundle and tool requirements

The policy requires the standardized Sigstore bundle media type `application/vnd.dev.sigstore.bundle.v0.3+json` and rejects legacy bundle formats. It pins Cosign to exact reviewed version **v3.1.4** (the latest release observed on 29 October 2026 when the policy was authored). The wrapper calls `cosign verify-blob` with the exact certificate identity and OIDC issuer and does not suppress transparency-log or certificate-transparency checks. The policy digest must be pinned out-of-band through `SYMTROPY_TRUSTED_POLICY_SHA256`; deriving the “trusted” digest from the same untrusted policy file would be circular.

This exact pin is intentionally conservative. Update it only after reviewing the newer release notes, the verification path, bundle format, test corpus and relevant advisories. Do not replace the exact identity with a broad regular expression.

### Security rationale

Cosign's release history records a verification bypass for legacy bundles (GHSA-fx35-mq7g-6g98), fixed in v3.1.3 and backported to v2.6.5. A separate malformed-attestation/predicate-check issue (GHSA-w6c6-c85g-mmv6) was fixed in v3.0.6 and v2.6.3. These issues reinforce two requirements: pin a reviewed verifier release, and reject legacy bundle formats rather than relying on automatic format detection. The current profile signs and verifies the receipt as a blob; a future in-toto predicate design must also use strict claim and predicate-type validation.

References:
- [Cosign v3.1.4 release (20 October 2026)](https://github.com/sigstore/cosign/releases)
- [GHSA-fx35-mq7g-6g98: verification bypass via legacy bundle](https://github.com/sigstore/cosign/security/advisories/GHSA-fx35-mq7g-6g98)
- [GHSA-w6c6-c85g-mmv6: malformed attestation/predicate validation issue](https://github.com/sigstore/cosign/security/advisories/GHSA-w6c6-c85g-mmv6)
- [Sigstore standardized bundle format](https://docs.sigstore.dev/about/bundle/)
- [Cosign blob verification](https://github.com/sigstore/cosign/blob/main/doc/cosign_verify-blob.md)
- [Cosign attestation verification and fail-closed policy guidance](https://docs.sigstore.dev/cosign/verifying/attestation/)
- [in-toto Attestation Framework](https://github.com/in-toto/attestation)
- [SLSA provenance specification](https://slsa.dev/spec/v1.2/provenance)

## 3. What the verification wrapper does

`verify_signed_execution_receipt.sh RECEIPT BUNDLE PAIRED_INPUT POLICY`:

1. Requires an out-of-band pinned policy digest.
2. Verifies the policy has scoring disabled and the exact reviewed identity/version/bundle policy.
3. Rejects bundles whose media type is not the standardized format.
4. Rejects receipts marked score-eligible or attempting to claim the not-yet-implemented v0.1 attestation state.
5. Checks Cosign is exactly v3.1.4, then delegates signature, receipt-byte binding, certificate identity, issuer and transparency checks to `cosign verify-blob`.
6. Runs the independent receipt verifier to recompute payload SHA-256 and check the receipt against the paired fixture.
7. Explicitly reports that scoring remains disabled and physical validity has not been inferred.

The wrapper's unit tests use a stubbed Cosign executable to test failure behavior, argument construction, policy pinning and standardized-bundle checks. Those tests **do not validate a real Sigstore signature**. No signed execution receipt is committed, and the named protected-main attestation workflow has not been implemented. The cryptographic step must be tested using an independently produced signed fixture before it can be trusted for scoring.

## 4. Threat model and fail-closed rules

The design must reject:

- policy drift, unless the trusted policy digest is updated through an independently reviewed path;
- wildcard or unexpected signer identities, unexpected OIDC issuer, legacy bundles, unsupported Cosign versions, skipped transparency checks, corrupted receipt bytes, and missing bundles;
- signatures made by a different workflow, even if its key is valid;
- receipt signing by code executed from an untrusted pull-request revision;
- a complete but unpaired receipt matrix, unequal budgets, different plant/configuration/observation-policy digests, or output-hash mismatch;
- any attempt to mark v0.1 receipts as scoring eligible.

The model still trusts the configured build/runner identity to faithfully execute the simulator and report measured resources. Stronger evidence may require an isolated runner, measured boot/attestation, independent output recomputation, a second replay, and reviewer-controlled experiment inputs. A signature does not establish physical validity, operational safety or generalization beyond the tested envelope.

## 5. Evidence ladder after implementation

| Level | Required evidence | Allowed claim |
|---|---|---|
| A0 | Synthetic schema/wrapper negative controls | The wrapper rejects the tested malformed policy and receipt conditions |
| A1 | Actual standardized bundle verifies against a known signer | The exact receipt bytes were signed by the expected identity under the selected trust root |
| A2 | Trusted execution workflow produces receipt and output hashes in the same isolated job | The attested runner reports completing this exact run with these inputs and measurements |
| A3 | Complete paired receipt set passes fair-budget and artifact-identity checks | The declared runs are comparable on the controlled input/resource dimensions |
| A4 | Validated plant, paired baseline experiment, locked test and uncertainty analysis | Performance differences are supported for the exact evaluated model and test distribution |
| A5 | Independent replay, second model/site and relevant HIL/field tests | Evidence supports the stated generalization claim within the demonstrated deployment envelope |

No higher level follows automatically from the preceding level; evidence must be reported claim-by-claim. This profile is currently at A0 design level only.
