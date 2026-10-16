---
title: Space Economy Reference Benchmark Suite
version: 0.1
status: proposed-reference-campaign
date: 2026-10-16
scope: cross-stack, deterministic economic and infrastructure decision benchmarks
evidence_class: synthetic-fixtures-until-replaced-or-augmented-by-qualified-data
---

# Space Economy Reference Benchmark Suite v0.1

## Purpose

This suite turns the Symtropy space-economy architecture into falsifiable engineering work. It is a **benchmark specification and synthetic fixture corpus**, not evidence that the engine, a lunar process, a market, or a flight system has passed. Fixture values use synthetic cost units (SCU); they are not price estimates or forecasts.

The scope complements, rather than replaces, the economic ledger, interplanetary trade, deep-space logistics, space-bootstrap, and industrial-closure specifications. The benchmark asks whether those systems can jointly make decisions that are physically consistent, economically transparent, resilient to disruption, and honest about evidence.

## Paradigm shift

**Optimize delivered, qualified capability over its lifecycle—not launches, payload mass, production volume, or token throughput in isolation.**

The economic object is a *service delivered under constraints*. A communications service, delivered kilogram, repairable asset-year, watt-hour at a lunar worksite, or kilogram of qualified product has a cost, reliability, location, delivery window, provenance, uncertainty, and end-of-life consequence.

This implies six architectural shifts:

1. **Mission → service network.** Compare interoperable services and infrastructure portfolios, not only individually optimized spacecraft or missions.
2. **Import-versus-make → lifecycle trade space.** Local production can reduce Earth-launched mass while increasing cost, energy demand, maintenance, or risk. Never declare ISRU economical from mass closure alone.
3. **Cheapest plan → feasible Pareto frontier.** Apply physical, contractual, safety, and sustainability gates first; only then compare cost, mass, availability, recovery, and resilience. Preserve alternatives instead of hiding value judgments in one arbitrary weighted score.
4. **Perfect information → delay-aware decisions.** Prices, inventories, offers, and operating states are local observations with source and age. A financial claim cannot create physical inventory; stale or expired information cannot silently become current truth.
5. **One expected case → robust operation.** Evaluate outages, launch slips, degraded equipment, energy scarcity, corrupted evidence, and tail costs. A cheap nominal plan that fails a hard service floor is infeasible.
6. **Simulation output → evidence-bearing claim.** Every recommendation must bind exact scenario input, assumptions, units, evidence class, model version, source commit, and verifier output. Estimated, simulated, observed, measured, and independently qualified values must remain distinguishable.

## System responsibilities

- **Sol-Atlas:** spatial/time context, asset and infrastructure topology, terrain/illumination/coverage layers, and links to their source datasets. It must expose the frame, epoch, resolution, validity region, and provenance of spatial inputs.
- **Symtropy:** deterministic scenario execution, causal inventories and energy ledgers, resource/process accounting, logistics scheduling, constrained optimization, and Pareto analysis.
- **Symthaea:** bounded hypothesis generation and plan comparison. It may propose actions or explain trade-offs; it does not supply missing evidence, turn unknowns into zero, or grant execution authority.
- **Mycelix:** provenance, versioned claims, permissioned sharing, custody/contract attestations, and append-only correction/retraction. A record attests that a claim was made with specified evidence; it does not make the claim physically true.
- **Independent verifier:** checks fixture hashes, arithmetic invariants, negative controls, candidate output shape, and exact source/tested revision. No component may certify its own result solely from its own narrative.

No live spacecraft command path is in scope for this benchmark.

## Benchmark protocol

### Input contract

Every run binds:

- `suite_id`, `suite_version`, and scenario ID;
- exact scenario-manifest digest and fixture class;
- units and any conversion profile;
- candidate implementation, exact source commit, dependency/toolchain profile, and configuration;
- input evidence classes and validity bounds;
- baseline implementation/version;
- deterministic seed, where randomness is used.

Missing or incompatible required fields produce an explicit `indeterminate` or `rejected` result, not a guessed default. The same input and exact implementation must replay to the same decision and same normalized result. Floating-point tolerances, if later needed, must be explicit per metric and must never be used to relax conservation or hard policy gates.

### Result contract

Each candidate returns:

- feasibility status and each gate's result;
- the chosen option **and** the remaining Pareto-nondominated options;
- normalized lifecycle cost and cost per qualified service unit;
- Earth-launched mass and local energy/material use where applicable;
- service coverage/availability and outage/recovery outcomes;
- resource, environmental, and evidence uncertainties;
- rejected alternatives with machine-readable reasons;
- the input digest, source revision, assumptions, and verifier receipt.

A lone scalar score is insufficient. Reports must show the metric vector and disclose any policy weights used for a final choice.

### Required order of operations

1. Parse and type-check units, frame/time profiles, evidence classes, and applicability.
2. Verify physical, custody, authority, and hard policy invariants.
3. Mark infeasible options and emit precise rejection reasons.
4. Evaluate lifecycle economics and service outcomes for remaining options.
5. Produce Pareto alternatives and any explicitly policy-conditioned recommendation.
6. Replay and independently verify the decision receipt.

## v0.1 scenario families

The frozen JSON corpus contains six small synthetic cases:

| ID | Purpose | Required result |
|---|---|---|
| `LIFECYCLE_MASS_COST_TRADEOFF` | Prevent launch-mass reduction from being mistaken for economic superiority | Report the mass/cost trade-off; no universal winner |
| `CISLUNAR_RELAY_N_MINUS_ONE` | Evaluate a critical service under a single-provider outage | Reject a plan that misses demand; a feasible dual-provider option survives one provider loss |
| `DELAYED_MARKET_AND_DOUBLE_SALE` | Exercise stale offers, acceptance after expiry, duplicate sale, and replay | Reject stale/expired and conflicting transactions; idempotent replay cannot duplicate custody or settlement |
| `PROCESS_MASS_BALANCE_AND_DUPLICATE_EVENT` | Check process input/output/waste balance and event idempotency | Preserve mass; reject the over-mass process and duplicate import |
| `DEBRIS_GATE_BEFORE_COST` | Prevent an economically cheap but policy-infeasible Earth-orbit option from winning | Reject the non-compliant option before economic ranking |
| `ROBUST_COST_AND_SERVICE_FLOOR` | Compare nominally cheap fragile supply with a robust provider | Enforce service floor in every required stress case before comparing cost |

These are **contract tests**, not calibrated engineering models. The corpus intentionally uses small, transparent numbers so that expected properties can be checked independently by inspection.

## Proposed benchmark targets

Targets below are gates to pursue, not measured results.

### Tier 0 — integrity (release blockers)

- 100% of frozen negative controls are rejected for the expected reason.
- Zero mass creation/loss outside an explicitly modeled source, sink, or uncertainty interval.
- Zero duplicate custody transfer, double sale, or duplicate settlement under replay.
- Zero hard-invariant violations accepted as a feasible plan.
- 100% of results bind an exact input digest, source revision, scenario ID, and verifier result.
- Exact deterministic replay for the deterministic corpus.
- No claimed pass inferred from a queued, cancelled, stale-head, skipped, or unrelated workflow.

### Tier 1 — decision quality

- For each small scenario with an enumerable solution, match the independent oracle on feasibility and expected ordering.
- Correctly preserve Pareto-incomparable choices; do not force a single answer when objectives genuinely trade off.
- Reduce lifecycle cost per qualified service unit by a target of **10–20%** against a documented, strong baseline in at least three representative scenario families, without breaking any hard constraints. This is a research target; accept it only when reproduced under equal assumptions.
- Report median, p95, worst-case, and sensitivity results; do not publish only the best seed or mean cost.

### Tier 2 — resilience and uncertainty

- Detect 100% of the frozen single-fault cases and reject invalid recovery plans.
- Demonstrate service demand coverage after the required single-provider failure where the scenario claims N-1 capability.
- Keep uncertainty and evidence class attached through transformations; never promote simulated or estimated resources into accepted inventory without an explicit acceptance event.
- On a future measured corpus, show calibration and tail-risk performance against a baseline before using probabilistic claims operationally.

### Tier 3 — scale and external validity

- Measure runtime and memory against increasing asset, provider, route, event, and uncertainty-set counts.
- Use independently sourced ephemerides, environmental measurements, costs, process yields, and availability data when available; record data license, publication/retrieval date, resolution, and limitations.
- Compare with independent tools or a separately implemented oracle. A model and checker sharing the same bug do not constitute independent verification.
- Run shadow-mode replay with domain review before any operational use; flight qualification remains a separate program.

## Metamorphic and adversarial tests

The implementation should grow beyond fixed expected answers. At minimum:

- Raising an option's cost while all else remains equal must not improve its cost ranking.
- Removing a required provider or lowering usable capacity cannot make a previously infeasible demand feasible.
- Duplicating an idempotent event cannot change inventory, custody, or settlement.
- Reordering independent events must preserve results; reordering causally dependent events must either preserve defined causal semantics or produce an explicit conflict.
- Increasing a local-production energy price must not make that option look cheaper.
- Introducing an expired offer or unqualified resource must not expand the feasible action set.
- Tightening a hard disposal/safety constraint cannot enlarge the feasible set.
- Replacing measured evidence with a lower evidence class must not silently increase decision authority.
- A model version, ephemeris, frame transformation, process recipe, or cost assumption change must change the input/derivation identity where it affects results.

## Data and standards boundary

This suite is informed by:

- [NASA's LunaNet Interoperability Specification](https://www.nasa.gov/directorates/somd/space-communications-navigation-program/lunanet-interoperability-specification/), Version 5, published 7 February 2025. It defines a framework for interoperable lunar networking, communications, navigation/PNT, and information services.
- [CCSDS](https://ccsds.org/), which develops space data-system standards for interoperability and cross-support across participating agencies.
- [ISO 24113:2023](https://www.iso.org/standard/83494.html), which defines debris-mitigation requirements for objects launched into or passing through near-Earth space. Applicability must be evaluated for the particular mission; this is not a blanket lunar-surface or off-Earth factory standard.
- [ESA's debris-mitigation verification guidance](https://technology.esa.int/page/space-debris-mitigation), including its 2025 revision of requirements/guidance.
- [NASA's 29 June 2026 lunar enabling-infrastructure announcement](https://www.nasa.gov/directorates/stmd/nasa-seeks-industry-input-to-accelerate-lunar-surface-technologies/), which identifies surface power, ISRU, advanced manufacturing, and nanomaterials among the integrated capabilities needing development.
- [Gao et al., *Lunar surface energy infrastructure* (2 September 2026)](https://www.nature.com/articles/s44453-026-00053-w), a review of lunar power architectures and first-order integrated cost-benefit considerations.

These references motivate interoperability, sustainability, and integrated-resource scenarios. They do not provide validated cost inputs for the synthetic fixtures, and citing a standard does not establish compliance.

## Implementation sequence

1. Keep the v0.1 manifest as a frozen, digest-pinned research artifact.
2. Add a standalone validator for its schema and invariants before connecting it to runtime code.
3. Implement the simple deterministic cases in a small Rust reference harness with no live-service dependencies.
4. Add fault-injection controls and an independent verifier with machine-readable receipts.
5. Connect the benchmark adapters to Symtropy, Symthaea, Sol-Atlas, and Mycelix only through explicit typed contracts.
6. Add externally sourced datasets and realistic uncertainty only when their provenance and interpretation can be independently reviewed.

The first milestone is successful rejection of incorrect claims, not a grand-economy visualization. The suite succeeds when it can tell us that a plan is infeasible, a proposed saving is illusory, or the available evidence is insufficient—and can show another reviewer exactly why.
