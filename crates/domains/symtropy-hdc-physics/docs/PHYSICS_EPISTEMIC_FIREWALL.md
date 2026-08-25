# Physics Epistemic Firewall

## Purpose

Symtropy may use HDC, CfC/LTC, learned surrogates, retrieval systems, or other
semantic models to observe and advise the simulation. They are never the
authoritative physics state.

The firewall separates two responsibilities:

- **advisory layer**: retrieve, classify, predict, estimate error, flag novelty,
  request fidelity, and suggest solver effort;
- **authoritative layer**: integrate equations of motion, enforce constraints,
  conserve/account modeled quantities, execute representation changes, and
  commit world state.

The rule is:

> Learned systems may propose. Certified physical kernels decide and commit.

## Typed proposal contract

`PhysicsAdvisory` carries:

- deterministic proposal id and physics tick;
- provenance (`HdcRetrieval`, `ContinuousTimeModel`, hybrid, rule-based, or
  external);
- requested action;
- calibrated confidence;
- predicted relative physical error when available;
- semantic novelty score when available;
- exact source-state digests supporting learned/semantic proposals.

The current action vocabulary is intentionally small:

- promote fidelity;
- demote fidelity;
- increase solver substeps;
- decrease solver substeps;
- request exact fallback;
- flag an anomaly.

It does not expose arbitrary force injection, direct transform mutation,
energy creation, material mutation, or hidden solver-state edits.

## Asymmetric admission

Accuracy-increasing requests are cheap epistemically: an advisor may ask for
more fidelity or more substeps without proving that the cheaper model is wrong.
A downstream resource policy can still decline the cost.

Accuracy-reducing requests are strict. By default they require:

- predicted relative error <= 1%;
- calibrated confidence >= 0.95;
- novelty <= 0.20;
- exact provenance for semantic/learned sources.

Novelty >= 0.80 escalates to exact/highest-certified handling instead of
allowing an approximation to continue.

These defaults are research starting points, not universal physical constants.
They must be calibrated per workload and validated against held-out exact runs.

## Required validation

A future adaptive-fidelity claim must compare an advisory run against an exact
or highest-certified reference on complete held-out scenarios. Report together:

1. wall time and peak memory;
2. physical error metrics relevant to the scenario;
3. conservation/reconciliation residuals;
4. false demotion rate;
5. exact-fallback rate;
6. novelty-detection lead time;
7. confidence/error calibration;
8. deterministic replay identity for the same advisory stream.

Do not claim an HDC/CfC scheduler improves physical accuracy merely because it
improves retrieval or prediction. Physical accuracy requires an intervention
study against exact metrics, consistent with `HDC_PHYSICS_RESEARCH_PROTOCOL.md`.

## Licensing boundary

The firewall types live in permissively licensed `symtropy-hdc-physics` and do
not depend directly on Symthaea's AGPL neural implementations. A bridge may
translate Symthaea HDC/CfC outputs into `PhysicsAdvisory` values without pulling
those implementations into the permissive physics crates.

This keeps the physical truth kernel reusable while allowing richer AGPL
intelligence layers to sit above it when desired.

## Future extensions

Only add a new advisory action when it can be admitted without bypassing exact
physics invariants. Likely additions include:

- request representation promotion/demotion;
- request solver backend;
- request local spatial refinement;
- request an uncertainty ensemble;
- request a validation probe or exact shadow step.

Representation changes must additionally pass mass, momentum, energy, topology,
and state-transfer contracts before becoming authoritative.
