# Content-addressed synthetic branch artifacts v0.1

This directory supplies exact bytes for the state/tape identities referenced by the 96-row NanoDrone recoverability fixture.

- snapshots/: 12 unique pre-action state records (family × checkpoint × training seed).
- disturbance-tapes/: 48 unique exogenous tape records (same state identity × branch seed).
- The tape file for a given branch seed is shared across intervention arms; distinct branch seeds have distinct tape bytes.
- Each filename is SHA-256 of the exact UTF-8 JSON bytes, including the final newline.

**Evidence ceiling:** the payloads are synthetic fixture data used to verify byte integrity, identity matching, and paired artifact reuse. They are not captured simulator states, restored physics snapshots, real RNG tapes, or independent runtime attestations. A real runner must serialize and hash the actual state and RNG/tape bytes and independently attest capture and restore.
