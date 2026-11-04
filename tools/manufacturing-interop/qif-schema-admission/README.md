# QIF 3.0 companion schema validation (v0.1)

This lane is intentionally named **companion** validation. It checks a pinned public QIF Community schema/fixture bundle; it is not a validation claim against the independently obtained normative ISO/DMSC distribution.

## What is pinned

- QIF Community source tree: commit `64268c175a3058d09db29c14869b94b781518709`.
- 22 schema files, each pinned by its Git blob SHA-1 in `source-pins-v0.1.json`.
- Two community QIF 3.0 sample instances, pinned by their Git blob SHA-1.
- The QIF Python binding's QIFDocument root wrapper and QIF-namespace-adjusted XMLDSIG schema are deliberately distinguished from the other QIF application/library XSDs.

The workflow verifies the exact tree commit and every source blob before using any file. It also computes SHA-256 digests over the retrieved bytes. Git blob SHA-1 pins repository object identity; SHA-256 records the exact bytes used by the validation job.

## Offline validation profile

The workflow constructs an isolated local bundle:

- `QIFApplications/QIFDocument.xsd`: public QIF Python-binding root wrapper.
- `QIFApplications/*.xsd`: QIF 3.0 application/library schema copies pinned from the QIF Community CPP-Kramer schema directory.
- `QIFLibrary/xmldsig-core-schema-qif3-namespace.xsd`: QIF-namespace-adjusted XML-signature companion schema used by the Python binding wrapper.

It inspects live XSD `xs:include` and `xs:import` references, rejects absolute/network references, confirms each local dependency resolves within the staged bundle, and validates with `xmllint --nonet`. The remote XMLDSIG import present in other QIFDocument copies is not used. The selected binding schema profile is not interchangeable with the unmodified XMLDSIG namespace/schema profile.

Positive controls:
- `repeatabilityTestUsingWhile.QIF` (plan fixture).
- `Exploded_Results1.QIF` (results / external-reference fixture).

Negative controls:
- QIF `versionQIF` mutated from the schema's fixed `3.0.0` to `2.1.0`.
- QIF namespace mutated from `http://qifstandards.org/xsd/qif3` to the QIF 2.1 namespace.

Both mutations must fail. A missing local include, attempted network reference, wrong schema profile or invalid sample fails the workflow.

## Dispositions and claim ceiling

A successful run reports:

`COMPANION_VALID_NOT_NORMATIVE`

It explicitly records:
- `normative_conformance_claimed = false`;
- `signature_semantics_verified = false`;
- `qif_xslt_rules_executed = false`.

This lane verifies local XSD parsing/validation for two pinned companion samples and these specific negative controls. It does not establish conformance of a vendor QIF implementation, complete conformance with all QIF 3.0 semantic/rule checks, cryptographic XML-signature validity, metrology traceability, measurement uncertainty, calibration validity, component conformity, or product-release authority.

## Why the normative package is separate

The DMSC/ISO 23952 normative package and the QIF Community's public bindings/samples are distinct source artifacts. The public companion repository describes itself as supplemental. Its Python/C# binding schema adjusts XML-signature namespace/global-element handling for code generation. We therefore preserve that profile's exact identity and do not promote it to a normative ISO/DMSC schema claim.

The normative package stays `NOT_ADMITTED` until its exact files, complete import closure, redistribution/usage terms and exact bytes can be independently pinned. This workflow does not automate a DMSC download form or treat the public companion bundle as a substitute.
