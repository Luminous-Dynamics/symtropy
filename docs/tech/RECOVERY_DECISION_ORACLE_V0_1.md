---
title: Evidence-Backed Cross-Domain Resource Recovery Decision Oracle
version: 0.1
status: research-design
date: 2026-10-17
issue: https://github.com/Luminous-Dynamics/symtropy/issues/1850
---

# Evidence-Backed Cross-Domain Resource Recovery Decision Oracle v0.1

## Status and claim ceiling

This is a research/design contract plus synthetic logic fixtures. It is not a qualified optimizer, asset certification procedure, legal determination, environmental permit, engineering sign-off, or physical controller. All fixtures are effects-disabled: a plan may be recommended for human/authority review, but no fixture authorizes cutting, lifting, transport, reuse, repair, opening/closing infrastructure, or hazardous-material handling.

The first target is a small deterministic Rust reference implementation, independently qualified against this specification. Do not claim field accuracy from synthetic cases.

## Paradigm shift

Optimize for **qualified useful value recovered per total burden**, not gross objects detected, tonnes moved, nominal material value, or robot throughput.

The decision pipeline is:

```text
observations / records
  -> asset + domain-profile identity
  -> evidence sufficiency, coverage, uncertainty and currentness
  -> non-compensable legal / safety / ecological / consent gates
  -> feasible candidate plans
  -> whole-chain multi-objective comparison under uncertainty
  -> decision receipt + human/authority review
  -> separately authorized execution (outside V0)
```

An observation is not a fact without provenance and scope. A prediction is not permission. A preferred plan is not an actuation command. An asset marked "dead", "damaged", "obsolete" or "waste" is not thereby available for recovery.

## Two stronger paradigms: evidence vectors and information economics

Do not collapse heterogeneous observations into a single condition score. Photovoltaic screening illustrates why: an I–V curve, electroluminescence image, and insulation/wet-leakage test measure different failure dimensions. A 2025 study of 76 modules used visual inspection, insulation resistance, I–V curves and electroluminescence; its reported 68% reuse result is specific to its sample and qualification rules, not a general target. V0 therefore includes a hostile case where performance screening passes but a separate safety gate remains unknown.

Choose the next test by **decision value**, not just asset risk, sensor novelty, or missing-data count. A 2026 Bayesian value-of-information study for marine concrete inspection reported that data-scarce zones could be more valuable to inspect than high-risk but data-saturated zones; the authors reported 46% lower cost and 8.81% risk reduction for their wharf case. Those figures are case-specific, not universal targets. V0 uses deliberately synthetic arithmetic to test that the declared net information value equals expected decision loss avoided minus test cost, delay burden and inspection risk. If the arithmetic or action label is inconsistent, reject the fixture.

For positive expected net value, a measurement may become an inspection candidate; it still does not clear the unresolved gate before the measurement result is observed and qualified. For non-positive expected net value, the rule only says not to choose that test **on value alone**; it does not authorize a hazardous or otherwise blocked operation.

## Interoperability, not another competing passport

Where available, link rather than duplicate established product-lifecycle and conformity records. The UNECE/UN Transparency Protocol Digital Product Passport already describes product or batch identity, provenance, dimensions, conformity evidence, lifecycle data and linked component records. This oracle should be a **decision-time companion receipt**: reference the applicable passport / component records; add the current evidence snapshot, domain-profile version, gate outcomes, unresolved uncertainty, feasible plans, comparison method, and review disposition. It should not claim to issue a DPP or replace a conformity credential. Legal requirements remain specific to jurisdiction and product class.

The intended seam is:

```text
product / batch passport + linked evidence credentials
  -> versioned current evidence snapshot
  -> domain profile + mandatory gates
  -> recovery comparison
  -> immutable decision receipt referencing the source record
```

That allows forestry lots, modules, turbine blades, infrastructure components and repairable machines to share the provenance/decision envelope while retaining their own measurements, tests, authorities and laws.

## Shared protocol, non-shared physics

The common layer owns evidence references, decision semantics, reason codes, deterministic comparison, and receipts. Each domain profile owns its physics, units, hazards, thresholds, applicable tests, calibration, regulators/authorities and acceptance criteria.

Initial candidate profiles:

| Profile | Assets | Domain-owned constraints and evidence |
|---|---|---|
| Forest salvage | recently dead / damaged timber | habitat retention, protected status, permissions, stability, pest/pathogen controls, usable timber yield, access and ground disturbance |
| Photovoltaics | retired PV modules | electrical isolation, insulation resistance, I–V curve, electroluminescence or other required tests, second-life acceptance |
| Wind blades | decommissioned composite blades | structural condition, transport/handling geometry, permitted reuse, repair, mechanical/chemical recovery profiles |
| Post-disaster materials | timber, concrete, steel and mixed debris | structural suitability, separation, contamination/asbestos and worker/public safety |
| Infrastructure components | bridge / structure components | qualified inspection, defect severity, engineering assessment and accountable authority |
| Industrial equipment | pumps, motors, spares | diagnostic coverage, failure mode, repairability, remaining useful life evidence, parts and process provenance |
| Electronic waste / batteries | circuit boards and cells | data-security disposition, chemistry identification, electrical/thermal hazard, qualified recycling route |

These are candidate profiles, not a claim that one score, sensor, threshold, or optimizer can govern them all. New profiles require their own evidence and tests.

## Decision contract

### 1. Versioned identity

Each run binds:
- asset/lot/subject identity and scope;
- domain-profile ID + version;
- evidence schema and policy/profile versions;
- planner/oracle version;
- units and quantization rules;
- source records, hashes/revisions, capture time and freshness rules;
- scenario and seed identity when simulation is used.

Changing a decision-relevant profile or threshold changes the run identity.

### 2. Evidence is typed

Decision-relevant evidence states must be distinguishable:
- observed / measured with source, time, method and uncertainty;
- inferred with model/profile and calibration domain;
- declared by an accountable actor;
- missing;
- stale;
- conflicting;
- invalid or out of applicability;
- not required under this profile.

Coverage must be explicit. "No defect observed" is not equivalent to "the relevant surface/region was inspected." Never turn missing evidence into a zero-valued defect or a passing test.

### 3. Hard gates precede scoring

Non-compensable constraints include applicable legal permissions, consent/authority, ecological or protected-area exclusions, worker/public safety, structural/electrical/hazard status, biosecurity and required evidence sufficiency.

Gate outcomes are `PASS`, `BLOCK`, or `UNKNOWN`; `UNKNOWN` is not `PASS`. If a required gate is `BLOCK` or `UNKNOWN`, the affected plan is not feasible, even when it has high estimated value. Gate outcomes and structured reason codes are part of the receipt.

A gate may block one plan but not necessarily every plan: for example, a suspect contaminated asset might be held from reuse while a qualified hazardous-material assessment remains a candidate. A profile must state that scope explicitly. Do not invent an alternative disposition merely to avoid abstaining.

### 4. Generate explicit alternatives

Where admissible, consider a bounded set of applicable plans:
- leave / retain in place;
- observe or inspect further;
- maintain / repair;
- reuse as-is after required qualification;
- disassemble / remanufacture;
- recover via ground, cable, aerial or local handling where relevant;
- material recycle;
- controlled storage / transfer to an authorized specialist;
- disposal only where an authorized and evidenced path exists.

A plan's cost, impact or yield estimate must have a unit, provenance/evidence class, uncertainty bounds and scope. A plan can be ineligible because its estimates are unknown; the planner must not fabricate completeness.

### 5. Compare the whole chain

Compare per **qualified usable output**, not cost per flight, gross tonnes or raw volume. Depending on the domain, objectives may include:
- qualified output quantity and grade / acceptance rate;
- full cost: mobilization, inspection, approvals, disassembly, handling, transport, processing, rejects, storage and downstream qualification;
- energy and lifecycle emissions where the boundary and factors are supportable;
- ecological damage, disturbance, habitat retention or other externalities;
- worker/public risk, exposure and reversibility;
- time-to-qualified-output and critical-resource availability;
- uncertainty, downside risk and sensitivity to plan/model assumptions.

Keep mandatory constraints outside the objective vector. Do not allow money or carbon credits to "buy off" a failed safety, legal or ecological gate. Report a Pareto set or a clearly disclosed preference policy rather than concealing incompatible objectives in one arbitrary scalar.

### 6. Decide when to measure again

The value-of-information decision asks whether an additional observation/test is likely to change the admissible plan enough to justify its cost, delay and own risks:

```text
expected decision loss avoided by test
    > test cost + test delay burden + test risk
    => consider test
otherwise
    => choose among current admissible options or abstain
```

V0 may implement a finite synthetic version of this calculation. It must disclose assumed priors, outcomes, units and decision-loss definitions. Do not present it as a calibrated real-world probability without held-out data.

### 7. Receipt and authority boundary

Emit a deterministic decision receipt containing:
- canonical input/profile/version digests;
- evidence records or immutable references and their currentness;
- gate results and reasons;
- feasible and rejected plans with explanations;
- objective vector, units, uncertainty/scenario range and comparison method;
- unresolved evidence and the reason for abstention;
- selected option for review, if one exists;
- explicit `NO_ACTUATION_AUTHORIZED` in V0;
- parent receipt/reference if this decision is revised.

Replay of identical canonical inputs, profile/version and planner version must produce identical canonical receipt bytes. Human review or later authorization is a separate event and does not rewrite the original decision receipt.

## Reference benchmark contract

The bundled `recovery-decision-scenarios-v0.1.json` contains 17 synthetic fixtures designed to test logic, evidence-envelope completeness and inspection-value arithmetic—not real detection or recovery performance. Each evidence record includes an explicit synthetic source reference, method, evidence class, uncertainty kind, coverage and freshness class. Every fixture sets `expected_actuation=false`.

### Mandatory gates and correctness

- **Gate escape rate:** 0. No plan with a required `BLOCK` or `UNKNOWN` gate appears as feasible.
- **Unknown permission / consent / protected status:** 100% held or blocked.
- **Unknown hazard or mandatory test:** 100% held from the affected operation.
- **No hidden actuation:** 100% of receipts carry `NO_ACTUATION_AUTHORIZED`.
- **Deterministic replay:** 100% byte-identical canonical receipts for identical normalized input/profile/version.
- **Provenance completeness:** 100% of decision-relevant facts, thresholds, estimates and comparisons have source/profile/version identities, and every synthetic evidence record declares scope, source reference, uncertainty kind, coverage and freshness.
- **No scalar safety shortcut:** a passing performance signal cannot silently stand in for an independent mandatory safety test.
- **Value-of-information arithmetic:** 100% of synthetic VOI fixtures reproduce the reported net value exactly, and the declared inspect/do-not-inspect-on-value-alone label matches its sign.
- **Counterfactual isolation:** rendering, map styling, prose, UI selection or absent sensors cannot silently mutate canonical physical state or grant permission.
- **No silent unit conversion/profile substitution:** mismatches reject with reason codes.

### Decision value and robustness

Report the following separately by domain profile; do not pool incomparable units into a single score:
- qualified yield / output acceptance rate;
- full-chain cost per qualified unit;
- energy and lifecycle emissions where defensible;
- rejects/rework rate;
- inspection and qualification time;
- disturbance/harm measures selected by the domain expert;
- abstention rate and reasons;
- decision flips under declared uncertainty perturbations;
- worst-case regret against an explicitly defined feasible reference oracle;
- value-of-information gain per inspection/test cost.

Synthetic tests can check arithmetic, gate ordering and perturbation behavior. Calibration metrics such as Brier score, confidence coverage and empirical false-negative rates require real, representative, held-out labeled observations and a declared sampling protocol. Do not set universal numeric field-performance targets before building that baseline.

### Transfer benchmark

Use leave-one-domain-profile-out tests. The shared protocol invariants should transfer; materials, hazard thresholds, physical models and objective factors must not. The expected result for a novel/unqualified profile is refusal or abstention, not successful prediction through analogy.

## Initial cross-domain synthetic cases

The corpus intentionally contains:
- deadwood that must remain for habitat;
- a synthetic cost case where cable extraction beats helicopter extraction;
- a remotely attractive aerial-lift candidate that is still held for missing operational evidence;
- a declining live tree that cannot be reclassified as deadwood;
- PV modules eligible for second-life review only after specified tests;
- a PV module with failed insulation held from reuse;
- a PV module with performance-screening results but unknown insulation safety, held until the independent test is resolved;
- one synthetic inspection where the expected decision loss avoided exceeds inspection burden, and another where it does not—both keep the unresolved operation on hold;
- a wind blade with unresolved condition and transport qualification;
- disaster debris with unknown asbestos status;
- lab-tested concrete eligible for engineering review, not automatic structural approval;
- an infrastructure crack with unresolved severity and authority;
- an industrial pump that is a repair candidate but not an automatically authorized repair;
- a swollen battery held from ordinary disassembly;
- electronic equipment with unresolved data-security disposition;
- woody debris in a sensitive habitat/watercourse where retention is the default until a qualified assessment.

Every cost/quantity value in the corpus is synthetic and clearly labeled. These cases must never be represented as measured field outcomes or permissions.

## Research grounding

- PV second-life decision process using visual inspection, insulation resistance, I–V curves and electroluminescence on 76 modules from a 22-year-old installation; its 68% reuse finding is study-specific, not a target to generalize: [Marinna et al., Journal of Cleaner Production (2025), DOI 10.1016/j.jclepro.2025.144941](https://doi.org/10.1016/j.jclepro.2025.144941).
- A Bayesian value-of-information framework for marine concrete inspection found knowledge scarcity affected inspection value; its 46% cost and 8.81% risk reductions are particular to the reported wharf case: [Reliability Engineering & System Safety (2026), DOI 10.1016/j.ress.2026.112817](https://doi.org/10.1016/j.ress.2026.112817).
- The UN Transparency Protocol’s Digital Product Passport specification provides a useful interoperability target for item/batch identity, provenance, component links, conformity evidence and lifecycle data. The recovery decision receipt should link to these records rather than replace them: [UNECE/UNTP Digital Product Passport](https://untp.unece.org/docs/specification/DigitalProductPassport/).
- A 2025 review of end-of-life wind-turbine blades compares recovery/reprocessing stages and cascading use paths; it reinforces that route comparisons must cover interdependent processing stages instead of optimizing only collection: [Renewable and Sustainable Energy Reviews (2025), DOI 10.1016/j.rser.2025.115834](https://doi.org/10.1016/j.rser.2025.115834).
- Post-disaster circular reconstruction argues for recovery pathways alongside hazard-aware separation and institutional coordination: [Çetin & Kirchherr (2025), DOI 10.1007/s43615-024-00495-y](https://doi.org/10.1007/s43615-024-00495-y).
- Wind turbine blade review compares recovery and utilization methods, including hierarchical mechanical recycling: [Resources, Conservation & Recycling (2024), DOI 10.1016/j.resconrec.2024.107813](https://doi.org/10.1016/j.resconrec.2024.107813).
- Recent bridge research integrates robotic inspection, digital-twin updating and maintenance decision support; its domain-specific model and field findings do not qualify this oracle: [Communications Engineering (2026), DOI 10.1038/s44172-026-00637-0](https://doi.org/10.1038/s44172-026-00637-0).
- A review of salvage logging reports context-dependent ecosystem effects across 96 publications; retained deadwood can have protective/ecological functions: [US Forest Service review](https://research.fs.usda.gov/treesearch/61585).

Research motivates the architecture only. Domain thresholds and claims still require independent source review and applicable qualification.

## Sequencing and related work

1. Freeze schema, receipt and synthetic scenario expectations.
2. Build a small effects-disabled Rust reference oracle using explicit domain profiles; no ML required.
3. Independently implement/check selected invariants or use a separate oracle to verify the reference implementation.
4. Add real datasets only with licensing, provenance, sampling frame and split discipline.
5. Add a domain profile only after domain-expert review and dedicated hostile fixtures.
6. Keep actuation and operational permission beyond this product's boundary.

Integrations to consider: #1661 value of information, #1769 robust design/survey planning, #1777 constraints from primitive evidence, #1818 wood robotics, #1838 deadwood salvage, #1843 infrastructure-fleet resilience, Mycelix #5140 provenance and Sol Atlas #120 read-only spatial projection.

## Non-goals

- no universal physical model or threshold set;
- no AI model required to decide V0 fixtures;
- no permission inference from remote sensing;
- no replacement for ecologists, engineers, safety officers, asset owners or legal authorities;
- no claim that every dead/damaged/retired asset is recoverable;
- no automatic actuation or autonomous hazardous-material handling;
- no real-world performance claims from synthetic tests;
- no forced scalar ranking when evidence or objective trade-offs do not justify one.
