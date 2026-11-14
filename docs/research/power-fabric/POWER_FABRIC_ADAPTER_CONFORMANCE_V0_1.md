# Power Fabric standards adapter conformance harness v0.1

**Status:** executable mapping ledger and semantic round-trip comparator; no IEC/CIM/SCL adapter or conformity claim  
**Date:** 2026-11-14  
**Owner issue:** https://github.com/Luminous-Dynamics/symtropy/issues/5215  
**Foundation:** Power Fabric IR v0.2 / PR #5211

## 1. Purpose

Implement the conformance harness before broad format adapters. A transformation must state what it preserved, what it semantically transformed, what it lost, what it could not map, and what was ambiguous. “Export succeeded” must not mean that the target file parsed while important source semantics disappeared.

## 2. Version-pinned mapping profile

Every mapping profile records:
- profile ID and version;
- adapter version;
- import/export/round-trip direction;
- standard designation and part;
- edition;
- amendment/corrigendum identifier where applicable;
- publication status (published, working draft, technical report);
- jurisdiction or national adoption scope.

A working draft or technical report is not silently treated as a published edition. The helper only reports whether the referenced publication status is Published; this is not a judgment that the standard is applicable or that an implementation conforms.

## 3. Source semantic inventory

The source inventory is frozen before transformation. Each in-scope semantic item has:
- stable semantic item ID;
- kind (object, relationship, property, extension, namespace, other);
- namespace;
- semantic digest;
- explicit in-scope flag.

The inventory must cover all objects, relationships, relevant properties, units, namespaces and extension payloads inside the declared mapping profile scope. The profile owner is responsible for documenting its scope boundaries. Out-of-scope items still receive an explicit NotInScope mapping entry; they are never silently ignored.

## 4. Mapping outcomes

| Outcome | Required information | Ledger effect |
|---|---|---|
| ExactPreservation | One or more target IDs and mapping rule; no loss record | Complete |
| SemanticallyEquivalent | Target IDs and mapping rule; semantic equivalence rule is documented | Complete |
| ExplicitlyLossy | Target IDs, mapping rule and one or more machine-readable loss records | CompleteWithExplicitLosses |
| Unsupported | No resolved target; explanation | IncompleteUnsupported |
| Ambiguous | Candidate target IDs and explanation | IncompleteAmbiguous |
| NotInScope | Explanation; only for an item frozen as out of scope | Explicit scope accounting |

A source item must have exactly one ledger entry. Duplicate entries, source IDs absent from the frozen inventory, and missing entries are rejected. An in-scope item cannot be marked NotInScope, and an out-of-scope item cannot be mapped as though it were in scope.

## 5. Round-trip semantic comparison

The comparator uses stable semantic item IDs and semantic digests, not raw XML/JSON/RDF byte equality. A target serializer may reorder XML elements or RDF triples without changing meaning; conversely, a syntactically valid file can silently lose a relationship or extension.

For each in-scope source item:
- matching ID and semantic digest: preserved;
- changed or missing item with ExplicitlyLossy mapping: reported as an explicit difference;
- changed or missing item with Unsupported/Ambiguous mapping: reported as unresolved/incomplete;
- changed or missing item whose mapping claimed ExactPreservation or SemanticallyEquivalent: rejected as undeclared semantic loss.

Additional target-only items are reported separately rather than silently treated as source preservation. The result also retains the exact profile, adapter, standard reference and ledger status. Unsupported/ambiguous mapping entries keep the result incomplete even when some same-ID payload happens to survive.

## 6. Qualification states

- SemanticallyEquivalent: every in-scope source item survived with matching semantic digest and no unresolved/explicit losses.
- EquivalentWithExplicitLosses: all differences are covered by explicit loss records; no unsupported/ambiguous item remains.
- Incomplete: unresolved semantics remain.

A successful source parser, target parser or schema validator is not a semantic round-trip result. Evidence of conformance must include exact standards/profile/adapter versions, frozen source and target documents, inventories, mapping ledger, loss/ambiguity report and reproducible comparator output.

## 7. Initial adapter sequence

1. Freeze narrowly scoped 81346 reference-designation examples, retaining source namespace and site designation policy.
2. Freeze a small IEC 61970 CIM distribution profile and the relevant IEC 61968 distribution extension semantics.
3. Add IEC 61850-6 SCL only for the appropriate IED/substation configuration scope.
4. Map applicable IEC 62351 security-profile metadata by part/edition; do not infer secure implementation from a standard reference alone.
5. Keep a separate South African jurisdiction profile for current SANS adoption and installation-law scope.

Do not force residential/building wiring or household service semantics into a CIM or SCL class where the source model does not represent them. Preserve opaque extensions and explicit unmapped status.

## 8. Frozen benchmark targets

- 100% of in-scope source semantic items receive exactly one mapping ledger entry.
- Zero silent loss of in-scope object, relation, property, unit, namespace or extension identity.
- 100% of unsupported or ambiguous semantics are machine-readable and leave the ledger incomplete.
- 100% of explicit losses carry a category, description, target reference and mapping rule.
- 100% rejection of round-trip differences claimed as exact/semantically equivalent.
- 100% deterministic mapping ledger and comparison order for identical frozen inputs.
- Compare adapter engineering time, reconciliation effort, round-trip loss count and maintenance effort against matched vendor/manual workflows.

These are proposed correctness targets, not achieved results or a standards certification.

## 9. Claim ceiling

This harness is not itself an IEC/ISO/IEEE/SANS adapter, conformance assessment, product certification, utility acceptance or electrical installation approval. Mapping-profile correctness depends on an expert-reviewed interpretation of the exact licensed standards text and relevant national adoption. Public catalogue metadata is insufficient to implement normative rules.
