# Causal Selection Identification Contract v1

## Purpose

SEL-07C1 inserts an explicit causal-identification authority between descriptive association and causal-effect execution.

The governing theorem is:

```text
strong design label
    != complete identification evidence
    != current identification authority
    != causal effect
```

A small p-value, large descriptive contrast, randomized-sounding design name, or controlled-simulation label cannot mint a causal-selection effect by itself.

C1 produces **no effect estimate**.

## Current-authority prerequisite

`CausalSelectionIdentification::declare(...)` consumes only a current `ValidatedSelectionAnalysisFrame<'_>`.

The frame already binds and replays the exact:

- SEL-07A comparison design;
- SEL-06C selection evidence;
- SEL-06A consequence evidence;
- original and estimand denominators;
- predictor materialization;
- outcome materialization;
- support/missingness policy;
- protocol/calibration policy;
- context/window.

C1 does not reopen those sources to construct a more convenient causal population.

## No B2-statistic input

C1 deliberately does **not** accept `ConsequenceAssociationEstimate` or `ValidatedConsequenceAssociation<'_>` as an input to identification declaration.

Therefore a small SEL-07B2 descriptive reference probability has no code path that can satisfy:

- assignment mechanism;
- temporal precedence;
- intervention fidelity;
- interference policy;
- outcome ascertainment;
- attrition policy;
- population integrity;
- protocol-freeze provenance;
- simulation replay/isolation criteria.

This makes the distinction structural rather than documentary:

```text
statistical extremeness != identification evidence
```

SEL-07C2 may later consume both B2 descriptive evidence and C1 identification authority, but neither substitutes for the other.

## V1 identification tier

V1 exposes one causal-identification tier:

- `InterventionIdentified`.

It is available only to:

- `RandomizedInterventional`; and
- `ControlledSimulationIntervention` source designs.

The following SEL-07A classes cannot mint this tier in V1:

- descriptive association;
- matched/stratified observational;
- within-family or lineage-controlled;
- common-environment;
- reciprocal-context;
- declared neutral/null comparison.

Those designs may provide valuable evidence, but reduced confounding risk is not treated as equivalent to intervention identification.

A future assumption-bound observational tier must be introduced as a distinct typed authority rather than silently widening `InterventionIdentified`.

## V1 causal target

The first target remains intentionally narrow:

```text
do(Comparison) - do(Reference)
```

for viability-window death risk under the exact binary predictor and exact context/window already frozen into the validated analysis frame.

The typed target is `BinaryViabilityRiskDifference`.

C1 contains no generic lifetime fitness and no context-free population-genetic selection coefficient.

## Required common intervention criteria

Randomized interventions require exactly these criteria:

1. `AssignmentMechanism`
2. `AllocationIntegrity`
3. `TemporalPrecedence`
4. `InterventionFidelity`
5. `InterferencePolicy`
6. `OutcomeAscertainment`
7. `AttritionMissingness`
8. `AnalysisPopulationIntegrity`
9. `ProtocolFreezeProvenance`

Every criterion must appear exactly once.

Missing, duplicate, or unexpected criteria fail closed.

The criterion order in persisted evidence is canonical enum order; caller input order is non-semantic.

## Controlled-simulation criteria

`ControlledSimulationIntervention` requires every common criterion plus:

10. `SimulationReplayIdentity`
11. `InterventionIsolation`
12. `ScenarioContext`
13. `SimulationStochasticityPolicy`

Thus a simulation being deterministic or labeled controlled is not by itself enough to create causal-identification authority.

The evidence must explicitly bind replay identity, intervention isolation/spillover treatment, scenario/context identity, and stochasticity/seed-policy authority.

## Criterion evidence identity

Every `IdentificationEvidenceRef` binds:

- typed criterion;
- evidence authority ID;
- evidence revision;
- evidence content digest;
- qualification authority reference;
- exact subject design digest;
- exact subject frame digest.

Evidence from another design or frame fails even if the human-readable criterion name is identical.

This prevents randomization evidence, timing evidence, or simulation replay evidence from one experiment from being silently reused for another analysis frame.

## Explicit qualification authority

Criterion evidence and criterion qualification are separate identities.

`IdentificationEvidenceQualificationRef` binds:

- qualification authority ID;
- qualification revision;
- qualification content digest.

This distinction exists because:

```text
some evidence bytes exist
    != those bytes satisfy the criterion
```

A future C2 causal-effect capability therefore cannot consume a bare criterion content digest without also preserving which qualification authority accepted it.

Changing either the evidence identity or the qualification identity changes causal-identification identity.

## External-trust boundary

Evolution-core binds external identification and qualification authorities; it does not infer their scientific validity from hashes.

The presence of a qualification reference means:

- the authority identity is explicit;
- its exact revision/content identity is evidence;
- its relationship to the exact criterion/design/frame is preserved in the C1 record;
- fresh replay must reproduce the same binding.

It does **not** mean Symtropy independently proved that the external authority was competent, honest, calibrated, or scientifically correct.

Those trust/qualification properties must be established by the relevant external evidence system and remain auditable as separate claims.

C1 therefore provides a fail-closed causal-identification **authority boundary**, not an oracle that turns arbitrary digests into truth.

## Exact criterion-set theorem

For the selected supported design class:

```text
persisted criterion set == exact required criterion set
```

not merely a superset.

C1 rejects:

- missing criterion;
- duplicate criterion;
- criterion not permitted by the source design class;
- evidence bound to another design;
- evidence bound to another frame;
- noncanonical persisted order;
- denominator inconsistency.

This prevents an ambiguous bag of causal-sounding evidence from becoming a capability.

## Denominator preservation

C1 stores both:

- original denominator; and
- estimand denominator.

They must remain consistent with the validated frame and obey:

```text
0 < estimand_denominator <= original_denominator
```

C1 does not create a new complete-case subset.

Any causal-effect executor must remain on the exact frame lineage.

## Representation identity vs current authority

`CausalSelectionIdentificationDigest` is deterministic representation identity.

It is not sufficient current authority.

A serialized identification must regain authority through non-Serde `ValidatedCausalSelectionIdentification<'_>`.

Current replay requires:

- the current validated analysis frame;
- a freshly supplied complete criterion-evidence set;
- the freshly supplied qualification authority references embedded in those criterion records.

C1 reconstructs the identification and requires exact equality.

Thus:

```text
serialized identification
    != current causal-identification authority
```

and:

```text
current causal-identification authority
    != causal-effect estimate
```

## Design class is necessary but insufficient

`RandomizedInterventional` and `ControlledSimulationIntervention` are eligibility gates only.

They do not bypass criterion evidence.

For example, a randomized design with no temporal-precedence evidence or no interference policy fails closed.

A controlled simulation with only the nine common criteria fails until all simulation-specific criteria are present.

## Descriptive and observational fail-closed boundary

A descriptive SEL-07A design cannot mint `InterventionIdentified`.

A matched observational design cannot mint `InterventionIdentified`.

The same rule applies to within-family, common-environment, reciprocal-context and neutral/null designs in V1.

This is deliberate conservatism. It prevents scientifically useful observational controls from being promoted into intervention causality by type naming alone.

## Wire-shape boundary

C1 contains no field for:

- causal effect estimate;
- descriptive p-value;
- scalar fitness;
- relative fitness;
- population-genetic selection coefficient;
- beneficial/deleterious mutation classification;
- adaptation;
- reproductive isolation;
- speciation.

The target identifies *what a later causal effect would mean*; it is not itself an effect value.

## Public adversarial corpus

The initial C1 corpus requires:

- complete randomized criterion set mints a persisted identification representation;
- Serde-restored identification requires fresh evidence+qualification replay before capability exists;
- descriptive design cannot mint intervention identification;
- matched observational design cannot mint intervention identification;
- missing temporal precedence fails;
- duplicate criterion fails;
- evidence from another frame/design fails;
- controlled simulation requires the four simulation-specific criteria;
- evidence/qualification drift stales restored identification;
- wire shape contains no effect/fitness/selection-coefficient/adaptation/p-value fields.

## Explicit non-claims

SEL-07C1 does not establish or estimate:

- causal effect magnitude;
- statistical significance of a causal effect;
- generic fitness;
- relative fitness in general;
- a population-genetic selection coefficient;
- beneficial/deleterious mutation status;
- adaptation;
- reproductive isolation;
- speciation.

It also does not independently prove the external scientific validity of each criterion's evidence or qualification authority.

## Successor boundary

SEL-07C2 may execute an intervention-identified causal viability-risk effect only by consuming a current `ValidatedCausalSelectionIdentification<'_>` together with the exact current B1 frame and B2 descriptive lineage.

C2 must not accept:

- raw design class alone;
- raw C1 digest alone;
- raw B2 association alone;
- small descriptive p-value alone.

Even when the causal point effect numerically equals the B2 descriptive risk difference, it must be a new evidence object whose causal interpretation is grounded in the current C1 capability.

A population-genetic selection coefficient remains a later, model-specific conversion.

## Evidence status

Source implementation, contract text, static review, and test design are not executable qualification.

A CI-only helper must freeze and execute the exact final C1 product head before any executable PASS is claimed.

Queued/unassigned CI is unexecuted evidence and must not be interpreted as success or product-code failure.
