# Physics Epistemic Firewall

## Purpose

Symtropy may use HDC, CfC/LTC, learned surrogates, retrieval systems, or other semantic models to observe and advise the simulation. They are never the authoritative physics state.

The firewall separates two responsibilities:

- **advisory layer**: retrieve, classify, predict, estimate error, flag novelty, request fidelity, and suggest solver effort;
- **authoritative layer**: integrate equations of motion, enforce constraints, conserve/account modeled quantities, execute representation/lifecycle changes, and commit world state.

The rule is:

> Learned systems may propose. Certified physical kernels decide and commit.

A second rule is equally important:

> Missing evidence is not favorable evidence.

Unknown novelty is not equivalent to low novelty. Missing error calibration is not equivalent to zero predicted error. An invalid policy is not a permissive policy.

## Typed proposal contract

`PhysicsAdvisory` carries:

- deterministic proposal id and physics tick;
- provenance (`HdcRetrieval`, `ContinuousTimeModel`, hybrid, rule-based, or external);
- requested action;
- calibrated confidence;
- predicted relative physical error when available;
- semantic/operational novelty score when available;
- exact source-state digests supporting learned/semantic proposals.

The current action vocabulary is intentionally small:

- promote fidelity;
- demote fidelity;
- increase solver substeps;
- decrease solver substeps;
- request exact/highest-certified fallback;
- flag an anomaly.

It does not expose arbitrary force injection, direct transform mutation, energy creation, material mutation, reservoir creation/removal, or hidden solver-state edits.

## Asymmetric admission

Accuracy-increasing requests are cheap epistemically: an advisor may ask for more fidelity or more substeps without proving that the cheaper model is wrong. A downstream resource policy can still decline the cost.

Accuracy-reducing requests are strict. Under the default research policy they require all of:

- predicted relative error <= 1%;
- calibrated confidence >= 0.95;
- an **explicit** novelty estimate <= 0.20;
- exact provenance for semantic/learned sources.

A fidelity-reducing request with `novelty_score = None` is rejected. The firewall does not reinterpret missing novelty as zero.

Novelty >= 0.80 escalates to exact/highest-certified handling instead of allowing an approximation to continue.

These defaults are research starting points, not universal physical constants. They must be calibrated per workload and validated against held-out highest-certified runs.

## Policy validity is part of the firewall

`EpistemicFirewallPolicy` is serializable/configurable state, so construction is not permanent evidence of validity. `evaluate()` revalidates the policy every time before considering a proposal.

A policy fails closed when:

- the reduction-error threshold is non-finite or negative;
- confidence/novelty thresholds are non-finite or outside `[0, 1]`;
- the reduction novelty ceiling is above the exact-fallback novelty threshold;
- the maximum substep value is zero.

This matters for floating-point policy fields: a NaN comparison must never become an accidental admission path.

## Firewall admission is necessary, not sufficient

`AdvisoryDisposition::Accept` means only that the proposal passes the semantic admission boundary. It does **not** authorize an authoritative state mutation by itself.

Before any fidelity reduction is enacted, a downstream authoritative policy must also establish, for the affected region/interval:

- finite/numerically healthy authoritative state;
- complete conservation/accounting evidence for every reservoir required by the active model;
- state-versus-ledger reconciliation within the declared tolerance;
- no unresolved reservoir appearance/disappearance or body/world lifecycle transition;
- no unknown internal ledger port required by the proposed lower-fidelity model;
- solver/representation-specific validity and transition contracts;
- deterministic checkpoint/fallback state sufficient to abandon the approximation when a guardrail trips.

These checks deliberately remain outside `symtropy-hdc-physics`: the permissive advisory crate does not become the authority over the numerical physics core. It emits a typed proposal; the authoritative layer owns physical admission and commit.

Until the Phase-Zero convergence gates are complete, adaptive-fidelity work should remain advisory/shadow-only rather than mutating solver fidelity in production.

## Exact/highest-certified terminology

`FidelityTier::Exact` is a policy label for the highest-certified/reference path available to the scheduler. It is **not** a claim of mathematically exact floating-point simulation.

Public evidence should name the actual reference solver, timestep, tolerances, and validity envelope rather than relying on the enum label alone.

## Required validation

A future adaptive-fidelity claim must compare an advisory/intervention run against a highest-certified reference on complete held-out scenarios. Report together:

1. wall time and peak memory;
2. physical error metrics relevant to the scenario;
3. conservation and state-versus-ledger reconciliation residuals;
4. unresolved reservoir/world lifecycle events;
5. false demotion rate;
6. exact/highest-certified fallback rate;
7. novelty-detection lead time;
8. confidence/error/novelty calibration;
9. deterministic replay identity for the same advisory stream;
10. policy version/fingerprint and solver/reference revision.

Include fail-closed negative controls:

- missing predicted error on a reduction;
- missing novelty on a reduction;
- high novelty despite high confidence;
- missing exact evidence digest for learned/semantic proposals;
- NaN or out-of-range policy thresholds;
- incoherent novelty thresholds;
- malformed substep requests.

Do not claim an HDC/CfC scheduler improves physical accuracy merely because it improves retrieval or prediction. Physical accuracy requires an intervention study against exact/highest-certified metrics, consistent with `HDC_PHYSICS_RESEARCH_PROTOCOL.md` and the core Phase-Zero convergence program.

## Licensing boundary

The firewall types live in permissively licensed `symtropy-hdc-physics` and do not depend directly on Symthaea's AGPL neural implementations. A bridge may translate Symthaea HDC/CfC outputs into `PhysicsAdvisory` values without pulling those implementations into the permissive physics crates.

This keeps the physical truth kernel reusable while allowing richer AGPL intelligence layers to sit above it when desired.

## Future extensions

Only add a new advisory action when it can be admitted without bypassing exact/highest-certified physics invariants. Likely additions include:

- request representation promotion/demotion;
- request solver backend;
- request local spatial refinement;
- request an uncertainty ensemble;
- request a validation probe or exact shadow step.

Representation changes must additionally pass mass, momentum, angular momentum, energy/reservoir identity, topology, lifecycle provenance, projection/lifting error, numerical-residual, and state-transfer contracts before becoming authoritative.
