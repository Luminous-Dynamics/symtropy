# Run receipt known-answer fixture

This fixture is intentionally static: its expected SHA-256 values were generated independently from Rust using Python's `hashlib.sha256` and JSON serialization with sorted keys for the parsed model/output values. The receipt uses schema `0.3.0` and identifies `RFC8785-JCS`. The fixed fixture has null build lockfile/manifest metadata; real workspace builds fill those fields from build-time digests. The independent oracle canonicalizes the fixed ASCII/integer-only fixture with sorted keys and compact separators, which is byte-identical for this constrained data; it does not claim full JCS implementation coverage. All model, output, and receipt-payload digests are over RFC 8785 JCS bytes.

- `receipt-known-answer-input.json`: exact raw artifact bytes; its trailing newline is part of the digest.
- `receipt-known-answer-output.json`: raw output artifact example. The recorded output digest is for the parsed JSON value reserialized as compact JSON, not these raw bytes.
- `receipt-known-answer.json`: expected receipt and all precomputed digests.

Expected digests:

| Material | SHA-256 |
|---|---|
| Exact input bytes | `cc014e7a9694d0bf7a836dd7ec8aa19552424111024d60822ad09a637a503e84` |
| Parsed model serialization | `71d93acd678db5ef8385b1e21984caff6547b22b2fe1c79753c4818d55f1841b` |
| Parsed output serialization | `8cd5aa75a94d38bbd330f9942a42e6136e9759125c09f2c23da95964621460e6` |
| JCS receipt payload serialization | `b0b045ab62955c0e0342ee9746447dee39181c32318920ab13ac40b15c871775` |

The test suite verifies the exact strings above and separately validates the links between the static receipt and input/model/output bytes. This fixture proves only serialization/digest agreement under the recorded contract; it is not a signature or a scientific validation result.
