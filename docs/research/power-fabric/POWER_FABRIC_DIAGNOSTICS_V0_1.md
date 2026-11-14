# Power Fabric deterministic diagnostics v0.1

**Status:** executable diagnostic/report contract; not an electrical safety assessment  
**Date:** 2026-11-14  
**Owner issue:** https://github.com/Luminous-Dynamics/symtropy/issues/5212  
**Parent:** Power Fabric IR v0.2, PR #5211

## 1. Purpose

A compiler finding must be more useful than a pass/fail bit. Each diagnostic states what is wrong or unresolved, why it matters, which exact revision/object is affected, what evidence is associated with the finding, and what reconciliation is required.

This increment supplies the stable diagnostic/report model and validation. It does not yet implement every analysis pass that produces these diagnostics.

## 2. Diagnostic contract

Every diagnostic contains:
- stable diagnostic identity;
- stable diagnostic code;
- severity (Critical, Error, Warning, Info);
- compilation effect (BlockCompilation, Warn, Inform);
- document/object/relationship target;
- source pass identity;
- concise summary;
- remediation instruction;
- related object IDs;
- evidence references.

Related object IDs and evidence references are non-empty, unique, and deterministically ordered. A target and all related object references must resolve inside the exact IR revision.

Critical and Error diagnostics must block compilation. Warning diagnostics cannot be informational. Info diagnostics must be informational. This prevents a caller from labelling a critical finding while simultaneously allowing a clean compilation result.

## 3. Stable initial code registry

| Code | Name | Intended use |
|---|---|---|
| PF-DIAG-0001 | RequiredClaimUnknown | Required claim has no admissible value |
| PF-DIAG-0002 | ContradictoryClaim | Claims for one property conflict |
| PF-DIAG-0003 | EvidenceReferenceUnresolved | Evidence identifier cannot be resolved or is out of scope |
| PF-DIAG-0004 | StaleConfiguration | Equipment/configuration identity is stale |
| PF-DIAG-0005 | MappingUnsupported | Standard/profile semantics cannot be mapped |
| PF-DIAG-0006 | IncompleteSourceMap | Declared source map is incomplete within stated scope |
| PF-DIAG-0007 | ChangeImpactIncomplete | Dependency closure is incomplete |
| PF-DIAG-0008 | ProjectionStale | Human-facing projection does not match current IR revision |

Codes are stable identifiers. Wording and remediation text may improve, but a code's meaning must not silently change. A materially changed meaning requires a new code or code-registry version.

## 4. Report contract

A report binds:
- report ID;
- IR document ID and revision ID;
- input digest;
- compiler version;
- profile version;
- ordered diagnostics;
- derived compilation status.

The status is computed:
- Blocked: one or more blocking diagnostics;
- CompletedWithWarnings: no blockers, at least one warning;
- CompletedWithInformation: no blockers or warnings, at least one informational finding;
- CompletedWithoutFindings: this pass emitted no diagnostics.

CompletedWithoutFindings means only that this compilation pass emitted no findings. It does not mean that a whole electrical installation is complete, safe, compliant, energized, de-energized, isolated, or approved.

## 5. Determinism

Diagnostics are stored by stable diagnostic ID. Related object IDs and evidence references are sorted and duplicate-checked. The report binds a specific input digest and compiler/profile versions. The digest is supplied by the caller; this module does not compute or authenticate it.

A later compiler orchestrator must add pass dependency/version identity and preserve historical reports rather than mutating an earlier report after the source graph changes.

## 6. Benchmark protocol

- 100% rejection of duplicate diagnostic IDs and invalid targets in the frozen hostile corpus.
- 100% of Critical/Error findings block the report.
- 100% of Warning findings remain visible as warnings.
- 100% deterministic ordering for identical report inputs.
- Zero diagnostic targets or related object IDs silently rebound to a different IR revision.
- Full diagnostic report reproducibility across the same document/revision, digest, compiler version, profile version, and diagnostic inputs.

These are contract-correctness targets, not human comprehension or electrical safety results.

## 7. Claim ceiling

A diagnostic report is not an installation inspection, electrical calculation, protection coordination, proof of physical source completeness, evidence authenticity service, Certificate of Compliance, or permission to touch equipment. Analysis passes must not generate a false-safe conclusion from absent, unknown, stale, contradictory, or out-of-scope data.
