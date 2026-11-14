# Power Fabric IR to Electrical Service Graph compiler bridge v0.1

**Status:** executable mapping compiler contract; not an electrical design approval or physics solver  
**Date:** 2026-11-14  
**Owner issue:** https://github.com/Luminous-Dynamics/symtropy/issues/5244  
**Parent:** benchmark corpus PR #5243 → adapter PR #5229 → resilience PR #5222 → change-impact/diagnostic/IR/service graph chain

## 1. Purpose

The validated Power Fabric IR and the Electrical Service Graph have distinct responsibilities:
- IR preserves stable identities, source identifiers, namespaced types, typed claims, provenance and extensions.
- ServiceGraph supplies the existing typed electrical topology and deterministic declared source-path evaluator.

This bridge compiles one into the other through explicit mapping rules. It does not invent a second graph model, infer mappings by string suffix, or automatically promote evidence maturity.

## 2. Explicit mapping profile

The profile requires:
- namespaced IR object kind → ServiceNodeKind;
- namespaced IR relationship kind → ServiceEdgeKind;
- explicit ClaimDomain → GraphEvidenceState policy;
- optional exact selected evidence claim ID per object.

Every IR object and relationship must map. An unknown mapping blocks compilation. Original IR stable object IDs and relationship IDs become graph node/edge IDs unchanged and are recorded in a receipt.

ServiceGraph stores directed relationships. An IR undirected relationship is rejected rather than silently converted into a directed edge. Supporting undirected source semantics requires an explicit future representation or reviewed normalization.

## 3. Evidence mapping

Node evidence selection is exact: the profile names one claim ID for an object. The compiler does not select the strongest-looking claim automatically.

For the selected claim:
- Value + evidence reference + explicit domain mapping → mapped GraphEvidence state/reference.
- Unknown, Unavailable, NotApplicable, missing evidence, or unmapped claim domain → GraphEvidence::Unknown plus an evidence-gap record.
- Contradictory → compilation blocked.

Relationship evidence comes from that relationship's own provenance and explicit domain mapping. Missing evidence or missing domain mapping remains Unknown plus an evidence-gap record.

The profile explicitly chooses domain→GraphEvidenceState semantics. In particular, Verified is not automatically equated with IndependentlyReviewed. The profile author must review the evidence and intended meaning before making that mapping.

## 4. Graph maturity ceiling

Graph maturity is supplied separately in GraphCompileContext; it is never inferred from the number of claims or serialization status.

- DesignOnly permits unknown node/edge evidence but records every gap.
- FieldReconciled requires every compiled node and edge to have FieldObserved or IndependentlyReviewed evidence.
- IndependentlyReviewed requires every compiled node and edge to have IndependentlyReviewed evidence.

GraphRevision continues to validate its separate maturity-evidence reference. The bridge checks every node and edge before invoking that constructor.

## 5. Compilation report

The report binds:
- source IR document and revision;
- mapping profile ID and version;
- target graph ID, revision and caller-supplied content digest;
- every object and relationship source type and target graph type;
- selected node evidence claim IDs and mapped evidence states;
- evidence gaps and their reasons.

The graph content digest is supplied by the caller. This bridge does not hash the IR, authenticate evidence, or claim the digest is canonical.

## 6. Frozen test targets

- Every IR object/relationship receives exactly one explicit mapping receipt.
- Stable object/relationship IDs are preserved byte-for-byte.
- Missing mappings, invalid evidence selection, undirected relationships or contradictory selected claims block compilation.
- Unknown/unavailable/not-applicable/missing-evidence claims remain Unknown and are reported.
- Caller-provided domain mapping controls evidence state.
- FieldReconciled and IndependentlyReviewed reject insufficient evidence on any node or edge.
- The existing source-path evaluator remains the only topology reachability implementation.

These are authored tests, not executed test results. No Rust toolchain was available in this execution; no local fmt, compile, Clippy or Cargo test pass is claimed.

## 7. Claim ceiling

This bridge compiles declared topology and evidence metadata only. It does not perform load-flow, short-circuit, arc-flash, protection coordination, grounding, cable sizing, islanding stability, battery runtime, physical source discovery, absence-of-voltage verification, isolation, safety certification, utility acceptance or Certificate of Compliance issuance.
