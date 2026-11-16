# ROBOT-CONFIG-CHANGE-QUAL-001 — development artifacts v0.1

Status: public synthetic development tranche. No robot hardware or production integration is implied.

Issue: https://github.com/Luminous-Dynamics/symtropy/issues/5488

## Research question

When a robot configuration changes, which previous capability claims and evidence remain applicable, and what is the minimum competent requalification needed to support the target configuration?

This package supplies a small public development corpus and a separately stored expected-disposition oracle. It does not yet include a Rust evaluator or a hidden holdout. The public oracle is intentionally visible and MUST NOT be represented as a blind challenge.

## Why this work is timely

Two 2026 robotics benchmarking results materially change the design:

1. Gläßer et al., *Benchmarking Real-World Robotic Manipulation through Stochastic Challenges* (CoRL 2026; arXiv:2611.12804, submitted 9 November 2026), argues for standardizing a reproducible distribution of physical task configurations rather than a single fixed test set. The authors report 568 trials with Stretch 3 and Spot with arm across two labs. We will adapt the distribution/seed/replication idea to configuration-change challenges; the reported manipulation success rates are not construction-robot comparators.
   - https://arxiv.org/abs/2611.12804
   - https://rpl-bonn.github.io/stochastic-challenges/

2. Jiang et al., *What Are We Actually Benchmarking in Robot Manipulation?* (CoRL 2026; arXiv:2606.04233) diagnoses shortcut solvability, insufficient statistical significance, creeping overfitting and data-source dependence in prominent manipulation benchmarks. We explicitly include these benchmark-validity checks rather than optimize only an aggregate score.
   - https://arxiv.org/abs/2606.04233
   - https://ripl.github.io/manipulation_benchmark_audit/

The industry context is also maturing:
- AMRA-201:2026 is a published general requirements/test-method standard for mobile robots on solid travel surfaces. It provides a relevant test-method comparator, not a robot configuration-change impact oracle: https://amr-alliance.org/amra-201/
- VDA 5050 v3.0 supports free-navigation path sharing and traffic zones, but transport/fleet semantics do not establish physical capability or safety: https://www.vda.de/en/press/press-releases/2026/260421_PM_VDA_5050_EN
- NIST's Mobility Performance of Robotic Systems programme focuses on test methods and metrology for mobile robot performance: https://www.nist.gov/programs-projects/mobility-performance-robotic-systems

Our proposition is deliberately narrower: **claim-specific evidence applicability after a modular robot configuration delta**. We are not claiming to replace AMRA, NIST test methods, OEM commissioning, ROS 2, MoveIt, fleet management, or functional safety systems.

## Existing owner boundaries

- Symtropy #1834: capability profiles and task/platform compatibility.
- Symtropy #3522/#3531: physical trial receipts and claim-specific evidence admissibility.
- Symtropy #5411: operational reconfiguration chronology through first accepted output.
- Symtropy #3628: accepted production, endurance, failures, maintenance and verified restoration.
- Symtropy #3591: interlaboratory repeatability/reproducibility.
- Symthaea #10543: generic design-change impact/evidence-applicability benchmark.
- Symthaea #4918: generic claim-relative dependency lineage and common-mode independence.
- Symthaea #12046: decision-sufficient evidence-request planning.
- Mycelix #5149: component identity, provenance, custody and scoped approvals.

This work is an adapter and benchmark. It must consume these owners' stable contracts; it must not clone their graphs, ledgers, state machines or authority.

## Files

- `robot-config-change-corpus-v0.1.json`: 16 candidate-visible synthetic cases plus a small claim catalog. References such as `fixture://robot/base/A` are symbolic, not URLs to hardware and not cryptographic digests.
- `development-oracle-v0.1.json`: public expected dispositions for the development cases. This is a debug oracle, not hidden test data.
- `schemas/robot-config-change-corpus-v0.1.schema.json`: closed JSON Schema Draft 2020-12 for the corpus package.
- `stochastic-challenge-protocol-v0.1.md`: design for seeded configuration distributions, cross-lab sampling, statistical power, benchmark-validity diagnostics and later guarded physical trials.

All robot identities, module identifiers, evidence references, units and witness names in these files are fabricated. No vendor capability or physical measurement is asserted.

## Disposition semantics

- `PreservedByExactIdentity`: the claim's declared dependency closure is unchanged under the canonical semantic identity rules.
- `PreservedByQualifiedEquivalence`: a claim-specific, admitted equivalence witness covers the changed dependency and the relevant operating envelope.
- `PreservedByExactRecomputation`: the claim was recomputed from the exact target inputs using a separately qualified method (not exercised in this initial corpus).
- `RequiresApplicabilityReview`: the change scope or applicability cannot yet be resolved without competent review.
- `RequiresBenchReverification`: a controlled bench procedure is the minimum resolving action.
- `RequiresPhysicalReverification`: a physical configuration-dependent observation/referee is required.
- `RequiresIndependentAdjudication`: evidence exists but requires an independent competent assessor.
- `NotPreserved`: prior evidence is not applicable to the target claim/configuration; this is not a failure verdict about the new configuration.
- `UnknownImpact`: dependency/change scope is incomplete or ambiguous.
- `Unsupported`: the transformation or claim lies outside the adapter's declared support.

These are claim-specific dispositions, not a universal ordinal evidence ladder or a single robot readiness score.

## Benchmark protocol

Compare on identical frozen inputs:
1. preserve-all by robot family/model name;
2. invalidate-all after every change;
3. direct-component-only invalidation;
4. generic change-impact output from Symthaea #10543;
5. the robotics dependency-aware adapter;
6. an independently implemented exhaustive oracle for small finite cases.

Report at least:
- critical false-preservation count (hard gate: zero);
- affected-claim recall (target: >=95% overall, 100% for predeclared safety/quality-critical claims);
- exact oracle agreement on supported small instances (target: 100%);
- false invalidation and unnecessary requalification separately;
- requalification actions versus invalidate-all (aspirational >=30% reduction with all hard gates intact);
- expert review reconstruction time (aspirational >=20% reduction);
- evidence-request count and cost by claim/source;
- abstention, unknown and unsupported rates;
- results by component family, claim criticality, ODD and change family.

Do not use one scalar score to trade away safety or quality. Statistical units must be independent configuration challenges/articles/labs as appropriate; repeated frames, heartbeats and telemetry ticks are not independent robot trials.

## Execution sequence

1. Validate the JSON files and closed schema locally.
2. Independently hand-check every expected development disposition.
3. Implement a dependency-light Rust reference evaluator with exact, deterministic reason codes.
4. Add metamorphic and mutation tests, including name-matching shortcut probes and within-envelope distribution perturbations.
5. Pin the exact Symthaea #10543 and #4918 interface versions before writing adapters.
6. Build a separate held-out challenge set with evaluator-held labels; commit only its commitment/manifest, not answers, to the public participant view.
7. After synthetic correctness, run a guarded bench campaign with one modular base and at least two interchangeable tools or sensors, independent metrology and a preregistered test plan.

## Current verification

This initial artifact tranche is authored as JSON/Markdown only. It has not been compiled or run against robot hardware. Any local JSON parsing/schema checks performed by the authoring environment must be reported separately from Rust tests, CI, HIL and physical qualification.

## Nonclaims

No evidence in this package establishes real robot capability, hardware interchangeability, safety, reliability, calibration validity, legal authority, standards conformity, vendor interoperability, production productivity or physical qualification. There is no controller, actuator, dispatch, E-stop, robot network or work-authorization path.


## V0.1.1 audit correction

A manual consistency audit found and corrected one unsafe oracle label in RCQ-016. The case declares `dependency_snapshot_complete: false` and includes an unclassified module with no admitted dependency mapping. The prior oracle marked `operator.recovery.observability` as `PreservedByExactIdentity` because its currently declared dependency list did not include the unknown module. That was unsound: an incomplete dependency snapshot cannot prove that the unknown module is unrelated.

The corrected disposition is:

```text
RCQ-016 / operator.recovery.observability
UnknownImpact / DEPENDENCY_CLOSURE_INCOMPLETE
```

**Conservative rule for this benchmark:** when the case-level dependency snapshot is incomplete or unknown, every evaluated claim in that case is `UnknownImpact` or `Unsupported` unless an independently admitted owner contract explicitly bounds the missing scope. A missing edge is not evidence of independence. This is a fail-closed benchmark rule; it does not assert that every physical change actually affects every claim.

The public oracle remains development-visible, and this correction is not a candidate performance result.

## Claim-profile binding before evaluator implementation

The corpus currently contains a small local claim catalog and symbolic `target_claim_profile_ref` values. Those are useful development fixtures, but they are not yet pinned, consumable upstream claim contracts. A dependency intersection alone cannot decide whether the competent next step is applicability review, bench reverification, physical reverification, or independent adjudication.

Before implementing the scored Rust evaluator, pin a read-only claim-profile binding contract. The design note is in `claim-profile-binding-contract-v0.1.md`. It specifies references to existing owners and the minimum fields the evaluator needs without creating a parallel capability registry, evidence graph, independence graph, or authority system.

## Industry and standards crosswalk

`qualification-industry-crosswalk-v0.1.md` records the existing test-configuration, metrology, interoperability, commercial-product and safety-standard comparators. The benchmark should ingest and compose those methods where applicable; it should not claim to replace them.

The public development oracle now has a closed Draft 2020-12 schema at `schemas/development-oracle-v0.1.schema.json`. Schema shape validation is still a separate check from semantic oracle review and evaluator tests; no independent validator or Rust evaluator result is claimed by adding this schema.
