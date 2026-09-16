# PHYS-EVID-03B0A — Independent Policy+Suite Transcript Framing Oracle v0.1

## Status

Source-first, crypto-free preregistration candidate stacked directly on PHYS-EVID-03A0.

Parent product head:

```text
0b1636f2521136ca54302b632e465d9f8bc233f9
```

Source 03A0 fixture:

```text
tests/fixtures/persistent_physics_attestation_core_v1_vector.json
blob 539986c087fd17cd10d413efd638cd8d583bdb76
```

This tranche freezes only the exact B0 signing-transcript framing. It performs no credential hashing, key generation, signature, or verification.

## Synthetic credential field

Use exactly:

```text
a0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebf
```

This is a framing placeholder only and is not a valid/authorized evidence-attestor credential claim.

## Transcript grammar

```text
ASCII "xenia-symtropy-physics-evidence-hybrid-v1"
u32be(transcript_version = 1)
u32be(crypto_policy_profile_len = 18)
ASCII "hybrid-pqc-auth-v1"
u32be(attestation_suite_profile_len = 26)
ASCII "hybrid-ed25519-ml-dsa65-v1"
credential_id[32]
u32be(attestation_core_len)
exact PHYS-EVID-03A core bytes
```

## Fixed offsets

```text
domain                       [0..41]
transcript_version           [41..45]
crypto_policy_profile_len    [45..49]
crypto_policy_profile        [49..67]
attestation_suite_len        [67..71]
attestation_suite            [71..97]
credential_id                [97..129]
attestation_core_len         [129..133]
attestation_core             [133..]
```

The fixed prefix is exactly 133 bytes.

## Exact outputs

```text
EndpointSample:
  03A core     285 bytes
  transcript   418 bytes

ConsecutivePresence:
  03A core     290 bytes
  transcript   423 bytes
```

Exact transcript hex is frozen in `tests/fixtures/b0_authentication_transcript_v1_vector.json` and independently reconstructed by `tests/verify_b0_authentication_transcript_v1.py`.

## Binding controls

The oracle proves framing-level sensitivity before cryptography exists:

```text
T(policy, suite, credential, core)
 != T(policy-v2, suite, credential, core)

T(policy, suite, credential, core)
 != T(policy, ml-dsa87-suite, credential, core)

T(policy, suite, credential, core)
 != T(policy, suite, changed-credential, core)

T(policy, suite, credential, endpoint-core)
 != T(policy, suite, credential, consecutive-core)
```

The alternate policy and suite strings are chosen with the same lengths as v1 so the inequality proves content binding rather than merely a length change.

## Independence boundary

The oracle uses only Python stdlib `json`, `struct`, and `pathlib`. It reads the frozen 03A0 JSON vector plus its own transcript fixture.

It does not:

- read Rust source;
- inspect Cargo/target output;
- hash the synthetic credential;
- generate/load keys;
- sign or verify anything;
- contact a network;
- execute subprocesses;
- infer crypto policy from a suite mapping.

Both policy and suite strings are explicit framing inputs.

## Cross-repository requirement

`Luminous-Dynamics/xenia-peer#341` must later reproduce these exact transcript bytes. The first signer implementation should treat this fixture as an external compatibility target rather than generating expected bytes from signer output.

## Qualification strategy

No dedicated CI lane is created with this source tranche. A later exact qualification should compose with the first Xenia transcript implementation or otherwise have a justified execution path, freeze all source fixture blobs, and explicitly avoid predecessor PASS laundering.

## Non-claims

No valid credential identity, credential hashing, Ed25519 validity, ML-DSA validity, key possession, authentication, issuer authorization, trusted time, replay/action authority, contradiction resolution, custody, delivery, settlement, consensus, or governance authority is established here.
