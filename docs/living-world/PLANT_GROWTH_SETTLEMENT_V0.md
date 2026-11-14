# Plant Growth Settlement V0

## Status

Normative design contract for future canonical plant growth. This document composes developmental response with finite-resource ecology; it does not claim exact plant chemistry or a Rust growth engine exists yet.

## Core rule

Developmental response proposes growth. It does not mint structure or biomass by itself.

Preferred path:

```text
heredity + stage + developmental history
        -> growth/allocation proposals
physiology + habitat + canonical stock
        -> feasibility / resource intents
        -> same-tick arbitration
        -> atomic growth settlement
        -> plant structural graph update
        -> refreshed derived fields / presentation
```

A positive reaction-norm value is not permission to create matter.

## Growth proposal

A proposal is read-only prospective state. It may describe actions such as:

- extend growth tip;
- create lateral branch/root;
- thicken existing segment;
- allocate foliage/reproductive structure;
- repair/remodel damaged tissue;
- senesce/drop retained structure.

It binds its source plant, developmental/species content version, target structural coordinate, authoritative tick/round and required resource/process assumptions.

## Resource separation

Keep at least three concepts distinct:

1. **exact canonical stock authority** — what conserved/stock quantity can actually be transferred;
2. **biological allocation model** — modeled costs/yields used to form exact intents;
3. **structural state** — geometry-driving biological result after settlement.

Do not make radius/length changes silently alter exact body stock, and do not infer exact stock from rendered volume.

## Atomicity

A structural growth commit and the exact stock/process settlement that authorizes it must not diverge.

Invalid states include:

- biomass/resource removed but no structural growth committed;
- structural segment created but required exact stock retained at source;
- growth cursor/tip advanced after a failed settlement;
- quantization residual advanced while enclosing growth transaction failed.

## Competition

Multiple growth tips and multiple organisms may compete for finite water/nutrients/other declared stocks during one tick.

Intent generation reads one canonical snapshot; allocation uses explicit deterministic arbitration rather than loop/thread order.

The scarcity rotation/policy authority contract applies wherever indivisible exact units create rounding advantage.

## Quantization

If a continuous/fixed-point biological model crosses an exact integer authority boundary:

- QI request residual belongs to the demand-generation phase;
- scarcity decides the exact grant;
- QR residual, when needed, belongs to the committed product/growth conversion phase.

Unmet demand is not hidden in either residual.

## Developmental history

For cumulative/structural R2/R3 responses, current environment influences **future growth actions** rather than rewriting previously grown structure.

Example:

```text
shade history
    -> internode-length response
    -> next-tip extension proposal
    -> resource-constrained settlement
    -> new segment with committed rest length
```

When light later improves, the old long internode remains long. Future internodes may differ.

## Allometry / constraints

Species rules may impose constraints such as:

- minimum/maximum segment proportions;
- branching-order limits;
- support/load limits;
- root/shoot allocation tradeoffs;
- stage-dependent growth eligibility;
- maintenance-before-growth policy.

Constraint evaluation must be versioned biological policy and deterministic from canonical inputs.

## Failure semantics

A rejected growth proposal is not partial growth.

Possible causes:

- insufficient exact resource;
- stale developmental/source state;
- target tip no longer active;
- structural constraint violated;
- stage changed;
- damage invalidated target;
- exact-unit resolution insufficient;
- arbitration grant below required threshold.

Failure leaves canonical stock, structural graph, residuals and growth-tip state unchanged unless the process explicitly commits a different outcome such as stress from scarcity.

## Coarse/offscreen growth

Offscreen plants may use aggregated growth only when the coarse representation retains sufficient statistics for every enabled future process.

A coarse closure may update stand/population structure distributions rather than individual graphs for exchangeable plants.

Landmark plants or plants with future-relevant structural history may remain individual structural authorities.

Promotion may materialize unresolved structure deterministically only within what coarse authority actually knew; it must not invent past scars/branch correlations and then call them historical facts.

## First implementation tranche

Keep V0 narrow:

1. one plant;
2. one active shoot tip;
3. one exact/typed resource budget or qualified placeholder stock boundary;
4. one reaction-norm-driven extension tendency;
5. one atomic segment creation;
6. deterministic replay.

Do not begin with a whole forest or complete photosynthesis model.

## Qualification fixtures

1. sufficient stock + valid proposal -> one structural commit;
2. insufficient stock -> zero structural mutation;
3. duplicate growth retry -> no duplicate segment/resource charge;
4. two tips competing for scarce resource -> arbitration independent of iteration order;
5. environmental improvement changes later growth but not earlier R3 structure;
6. failed transaction advances neither QI/QR state incorrectly;
7. same history/save reload -> identical graph and stock settlement;
8. renderer absent -> growth still executes;
9. exact stock totals reconcile across growth;
10. four-history tree benchmark shows resource-constrained, causal divergence rather than unconstrained slider changes.

## Non-goals

V0 does not define full photosynthesis, respiration, hydraulics, carbon chemistry, leaf-area optimization, FEM, or species calibration.

It freezes the rule that **development can propose a form, but finite canonical ecology must pay for every future-bearing structural change before that form becomes history**.
