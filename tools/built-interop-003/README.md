# BUILT-INTEROP-003 — license-aware external IDS corpus inventory

Tracking issue: https://github.com/Luminous-Dynamics/symtropy/issues/2020

## What this PR does

Pins a metadata-only index of the official buildingSMART/IDS implementation test corpus at commit 870f9c4e6e8f414e737b4d84ca1aa9b46fc6c8f3 (commit date 2026-09-25). The pinned tree contains 334 paired cases / 668 files under Documentation/ImplementersDocumentation/TestCases/. The index records paths, Git blob object IDs and byte sizes for each IDS and IFC input, plus a license pointer.

No IFC/IDS fixture contents are copied into this repository. The upstream LICENSE identifies CC BY-ND 4.0. CI checks out that exact immutable commit into a temporary workspace, verifies the upstream LICENSE and all 668 file identities/sizes, then exits; it does not alter or redistribute them.

## Important non-claims

Every entry is marked expected_outcome UNVERIFIED and execution_status NOT_RUN_EXTERNAL_CORPUS. Category names and filenames may contain “pass” or “fail,” but those are not treated as ground truth. This inventory is not a benchmark score, IDS checker, or evidence of adapter conformance.

The Rust parser-backed adapter in PR #2013 implements only a declared IFC4/IDS 1.0 IFCWALL.Name literal subset. Its coverage against this inventory is not assessed here. Keep XSD validity, IDS semantic auditing, and IFC-vs-IDS model checking as separate gates.

References:
- Official test-case guide: https://github.com/buildingSMART/IDS/blob/870f9c4e6e8f414e737b4d84ca1aa9b46fc6c8f3/Documentation/ImplementersDocumentation/developer-guide.md
- Pinned upstream license: https://github.com/buildingSMART/IDS/blob/870f9c4e6e8f414e737b4d84ca1aa9b46fc6c8f3/LICENSE
- IDS 1.0 final-standard announcement: https://www.buildingsmart.org/information-delivery-specification-ids-v1-0-is-approved-as-a-final-standard/

## Reproducibility

Run the metadata integrity check after checking out the pinned upstream repo:

    bash tools/built-interop-003/verify-corpus.sh tools/built-interop-003/corpus-manifest-v1.json /path/to/IDS

The verifier requires the external checkout HEAD to equal the pin, verifies the pinned license blob, checks case/file counts and uniqueness, compares the full manifest path set with the pinned upstream tree (so an omitted case fails), and compares each Git blob OID and byte length. Git blob OIDs are repository object references, not raw-byte SHA-256 hashes.

## Next step

Only after exact inputs are verified should the adapter execute a category/capability matrix. Each result must use documented expected outcomes or an independent reference implementation, record checker/version and feature support, distinguish parse failure, unsupported features and supported nonconformance, and remain NOT_RUN_EXTERNAL_CORPUS when an oracle is unavailable. Do not store, modify or redistribute CC BY-ND files without rights review.