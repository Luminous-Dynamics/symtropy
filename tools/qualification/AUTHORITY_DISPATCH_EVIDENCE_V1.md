# QUAL-001B authority-dispatch evidence verification v1

The evidence sidecar contains a digest of the exact dispatch payload bytes. A digest string alone is not independently replayable: the verifier also needs the payload bytes (or an immutable artifact containing them).

A complete evidence bundle should therefore retain both:
- the exact dispatch payload bytes;
- authority-dispatch-evidence.json.

authority_dispatch_evidence_verify_v1.py independently parses and validates the dispatch payload, validates the evidence object, recomputes the payload SHA-256, and requires the contract identity recorded in evidence to equal the identity in the payload.

This establishes cryptographic correspondence of the dispatch portion of the evidence bundle.

It does not establish verifier authority, contract truth, execution correctness, or qualification. The approved verifier-release record and independent qualification admission remain separate authority boundaries.