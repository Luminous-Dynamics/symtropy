# Exact-Model Parts Compatibility Evidence Graph v0.1

**Status:** contract and synthetic examples only. No real part fitment, purchase, delivery, repair, or safety qualification is asserted.  
**Tracker:** [PARTS-EVIDENCE-001 #1924](https://github.com/Luminous-Dynamics/symtropy/issues/1924). Related: [washer sourcing and parts resilience](https://github.com/Luminous-Dynamics/symtropy/blob/research/large-appliance-repairable-architecture-v0-1/docs/tech/WASHER_ACQUISITION_AND_PARTS_RESILIENCE_V0_1.md), [repair-outcome benchmark](https://github.com/Luminous-Dynamics/symtropy/blob/research/large-appliance-repairable-architecture-v0-1/docs/tech/PROOF_CARRYING_REPAIR_OUTCOME_BENCHMARK_V0_1.md), and [architecture decision engine](https://github.com/Luminous-Dynamics/symtropy/blob/research/large-appliance-repairable-architecture-v0-1/docs/tech/REPAIRABILITY_ARCHITECTURE_DECISION_ENGINE_V0_1.md).

## 1. Paradigm shift: compatibility is a scoped, versioned claim

A part is not compatible just because the brand matches, model names look similar, photographs resemble each other, a marketplace page says “fits,” or a catalogue lists a generic part category. Positive fitment claims need exact model/revision/market scope, dated provenance and declared evidence.

The ledger separates:
- **Manufacturer listed:** a model-specific catalog relationship is documented. This alone does not prove physical fit or functional equivalence.
- **Seller claim / cross-reference inferred:** a research lead, not confirmed fitment.
- **Physical fit verified:** the interface and assembly were checked; function remains unproven.
- **Functional compatibility verified:** fit and declared function checks passed in the stated scope. Safety qualification is still a separate gate.
- **Safety-qualified for declared scope:** separate safety evidence and qualified procedures are required.
- **Verified incompatible:** a negative test or authoritative record demonstrates a mismatch.
- **Unknown:** insufficient evidence; neither compatible nor incompatible.
- **Superseded:** historical mapping remains auditable but cannot be presented as current.

A compatibility record never authorizes hazardous DIY work, a safety-interlock bypass, sealed refrigerant work, or an unqualified substitution of a safety-critical component.

## 2. Evidence chain, identity and freshness

A future empirical claim should link exact model/revision/market and, where relevant, production or serial ranges, to part identity, dated evidence, source artifact digest, verification method and reviewer. Physical fit, function, and safety are distinct observations. Supplier listing, quote, accepted order, physical delivery, received-part identity and post-repair verification are separate states.

The fixture uses only `SYN-*` tokens, null source URLs and `empirical_eligible: false`. Even the abstract “functional compatibility verified” example is synthetic contract data—not a claim about any real manufacturer, part, model, or service action.

This graph is intended to connect to the existing repair outcome ledger: a repair episode can reference the compatibility claim, actual part consumed, supplier path, delivery receipt, and post-repair verification. It must not create a parallel work-order or provenance authority.

## 3. Source resilience: storefront count is not supply resilience

Two online listings may share one upstream distributor or stock pool. The graph records an upstream dependency-group token; independently verified delivered paths count distinct upstream groups, not seller names. Track the ladder separately: listing found → exact-model evidence → quote → order accepted → physical delivery → identity/condition inspection → qualified fit/function verification.

Do not treat a catalogue listing as a quote, a quote as a delivery, or multiple resellers of the same stock as resilient independent supply.

## 4. Research and data limits

The [Open Repair Alliance October 2026 release](https://openrepair.org/announcements/over-half-a-million-records-of-open-data-on-repair/) reports 619,875 repair attempts across 34 countries, from community repair events; the [current download page](https://openrepair.org/open-data/downloads/) says the dataset includes records through July 2026, uses ORDS v0.3 and is CC BY-SA 4.0. This is useful for repair-event vocabulary and workflow questions. It is a selected community-event sample, not a representative estimate of South African large-appliance failure prevalence or part-fit reliability.

[GS1's system architecture](https://www.gs1.org/standards/gs1-system-architecture-document/current-standard) is useful background on interoperable product identification and shared data semantics, not an appliance-part compatibility source. Preserve exact model suffixes, revision tokens, market and serial scope rather than rely on name similarity.

## 5. Proposed benchmarks (not frozen targets)

| Metric | Initial hypothesis | Required evidence |
|---|---|---|
| Exact-model evidence coverage | Every declared common-repair class has evidence or an explicit unknown/single-source marker | Frozen repair-class denominator and model/revision records |
| False-compatible rate | Zero false-compatible claims on a frozen positive, negative and ambiguous holdout corpus | Independent fit/function checks; count false positives |
| Wrong-order rate | Reduce misfit orders and returns vs documented baseline | All orders/returns and exact identities |
| Time to correct part | Reduce median/P90 from fault confirmation to received-and-identified part | Actual order acceptance and delivery timestamps |
| Independent source count | Aim for two independent verified paths where feasible | Upstream dependency and fitment evidence |
| Evidence freshness | Active claims are dated, scoped and marked stale/superseded when applicable | Captured-on date, applicability bounds and recheck record |

Do not optimize source count alone. Many unverified listings do not beat one demonstrated path. Show coverage, evidence confidence, fit-misorder risk, delivery time, and source independence separately.

## 6. Audit gates and low-capex path

The Rust auditor rejects duplicate identities, unlinked evidence, seller/catalog claims promoted into fit/function states, physical-fit claims without fit receipts, functional claims without fit and function receipts, incompatibility without negative evidence, unknown records with invented results, expired records called current, invalid supersession references, and supplier redundancy hidden by multiple storefronts.

Start with one common, low-risk washer repair class and one exact model/revision. Separate manufacturer/authorized source records from seller claims, then verify a delivered part under qualified procedures and link its restoration evidence. If exact fit cannot be supported, preserve unknown and do not recommend purchase.

A successful schema/Rust audit is only contract integrity. It does not establish real compatibility, stock, price, repair success, legal conformity or product safety.
