# Reproductive Investment V0

## Purpose

Define how organisms allocate finite biological resources to reproduction and dependent offspring without treating reproductive output as free population growth.

## Core rule

Reproductive investment is a canonical allocation process competing with maintenance, repair, growth, storage, locomotion, thermoregulation, defense, and other biological demands.

## Investment classes

Species may model different investments, including:

- gamete production;
- flowers/pollen/nectar;
- seed/fruit tissue;
- egg production;
- gestation;
- incubation;
- nesting/den construction;
- lactation/provisioning;
- guarding;
- parental transport;
- colony provisioning;
- symbiotic reproductive support.

No class is universal.

## Resource allocation

A reproductive policy consumes or reserves explicit resources according to the species/model.

Conceptually:

`physiology + reproductive stage + qualified environment/social state -> reproductive investment intent -> scarcity arbitration -> atomic settlement -> reproductive state/output`

Iteration order among sinks must not decide whether repair, growth, or reproduction gets scarce resources unless an explicit biological priority policy says so.

## Parent-offspring coupling

Where offspring depend on parents, future-bearing state may include:

- parent/offspring relationship;
- dependency stage;
- provisioning obligation;
- nest/den location;
- lactation or food-transfer state;
- protection/social state.

If this relationship changes future outcomes, collapse into anonymous population marginals is forbidden unless a qualified coarse representation preserves the dependency semantics.

## Cost persistence

Reproductive costs do not disappear when the parent leaves active simulation.

Energy depletion, body-mass transfer, nutrient depletion, injury risk, reduced maintenance, or other modeled costs must persist through coarse fidelity and save/reload.

## Failed investment

Aborted or failed reproduction must define what happens to invested resources and biological state where modeled.

Do not silently refund all investment because a visual offspring entity failed to spawn.

## Plants

For flora, reproductive allocation may directly compete with vegetative growth, storage, root investment, repair, and defense.

Flower/fruit/seed presence in presentation must derive from committed reproductive/phenological state.

## Animals

For fauna, reproductive investment may alter condition, movement, feeding need, risk tolerance, social behavior, body state, and parental memory/care.

Animation state is downstream presentation.

## Qualification

Test:

- reproduction cannot exceed available committed resources;
- competing sink order does not depend on ECS/thread iteration;
- failed transaction leaves all involved authorities unchanged;
- committed cost persists across unload/reload;
- dependent offspring relationships survive fidelity transitions where future-relevant;
- presentation offspring/fruit/egg count cannot mint canonical authority;
- repeated retries do not double-pay or double-recruit.

## Non-claims

V0 does not define universal parental care, mating effort, fertility cost, litter/clutch size, or real-species allocation values.
