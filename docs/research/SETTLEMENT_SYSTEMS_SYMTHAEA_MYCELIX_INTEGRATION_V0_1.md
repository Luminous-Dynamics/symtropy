# Settlement Systems: Symtropy × Symthaea × Mycelix Integration Contract v0.1

**Status:** Prototype integration contract. The new `symtropy-settlement-core` crate is an initial synthetic Rust kernel. Its CI and runtime behavior have not yet been verified in this branch. No city or habitat physical model is claimed.

## Intent

Give the City / Settlement Systems programme one coherent cross-repository architecture without merging every concern into one executable or treating simulation as authority.

The shared engineering principle is:

```text
human/community-approved constraints + evidence provenance
    -> immutable scenario/context projection
    -> deterministic synthetic execution
    -> policy comparison under uncertainty
    -> accountable human/community decision
    -> separately authorized real-world execution, if ever applicable
```

The kernel must keep operational facts, inferred states, simulated outcomes, evidence validity, community authorization and physical authority distinct.

## Existing work to reuse

### Symtropy
- `crates/domains/symtropy-settlement-core` (this branch): typed synthetic resource kinds, reserves, dependency propagation, explicit unknown states, service-floor evaluation and deterministic report digests.
- [CIVIC-WATER-001 #1645](https://github.com/Luminous-Dynamics/symtropy/issues/1645): synthetic water-service workflow and policy comparison.
- [Cross-platform scenario pack #1595](https://github.com/Luminous-Dynamics/symtropy/issues/1595): scenario identity, faults, observables, replay and out-of-domain dispositions.
- [Engineering twin rehearsal boundary #1467](https://github.com/Luminous-Dynamics/symtropy/issues/1467): exact read-only projections and the rule that a synthetic trajectory cannot overwrite canonical twin state.
- Existing adapters: `symtropy-symthaea-adapter` and `symtropy-mycelix-bridge`. Do not add either to the headless kernel as a required dependency.

### Symthaea
- [Settlement Systems architecture/benchmark agenda PR #7689](https://github.com/Luminous-Dynamics/symthaea/pull/7689).
- [Multi-service resource-flow fixture issue #7690](https://github.com/Luminous-Dynamics/symthaea/issues/7690).
- [Culturally plural city profiles #7686](https://github.com/Luminous-Dynamics/symthaea/issues/7686).
- [Earth-to-space settlement systems #7687](https://github.com/Luminous-Dynamics/symthaea/issues/7687).
- [Policy robustness issue #7618](https://github.com/Luminous-Dynamics/symthaea/issues/7618) and [adaptive pathways / changing criteria #7636](https://github.com/Luminous-Dynamics/symthaea/issues/7636).

### Mycelix
- [CIV-ECON-001 / Civilization OS #4804](https://github.com/Luminous-Dynamics/mycelix/issues/4804): rights/floors, evidence, authority, resources and claims must not collapse into one another.
- [Synthetic community-infrastructure composition #6156](https://github.com/Luminous-Dynamics/symthaea/issues/6156): distinct stewardship, economic interest, legal title, operator qualification, service covenants, reserves, external authority and engineering currentness. (The issue is tracked in the Symthaea repository and consumes opaque Mycelix references.)
- Mycelix remains the source of its own governance, provenance, rights and currentness claims; the simulator must not manufacture them.

### Public-service contracts
The Luminous Platform civic/public-service contract work may eventually provide a shared scenario/observation conformance pack. The consumer must re-check the exact source/contract head and named checks before pinning it. The settlement kernel must not depend on an unmerged or unqualified contract branch.

## Ownership boundaries

### Symtropy — deterministic synthetic execution
Own:
- frozen scenario execution and deterministic event/resource-allocation semantics;
- synthetic time, fault injection, scenario replay and run health;
- resource-kind separation, protected reserves and declared dependency semantics;
- policy baseline execution where a policy is explicitly supplied;
- reporting known results separately from unknown/blocked outcomes;
- exact scenario/configuration/input/output commitments.

Does not own:
- authoritative city/habitat state or measurements;
- rights, local democratic legitimacy, legal title or governance authorization;
- policy truth or physical model credibility;
- real infrastructure dispatch, actuation, emergency control or operator authority.

### Symthaea — reasoning, uncertainty and policy comparison
Own:
- scenario analysis and policy hypotheses;
- candidate policy generation and comparative assessment;
- uncertainty, missing evidence, distribution shift, regret and abstention;
- advice that explicitly names assumptions and claim ceilings.

Must not:
- alter or weaken a protected service/rights floor during evaluation;
- convert confidence into approval, evidence validity or actuation rights;
- silently change the information available to one policy versus another;
- present Symtropy rollouts as physical experiments or verified outcomes.

### Mycelix — governance, provenance and community authority
Own:
- community/context profile provenance and revision/currentness;
- who is authorized to set or amend profile constraints;
- consent, privacy, dispute, appeal, revocation and supersession semantics;
- rights and service-floor approval references;
- the distinction between a simulation result and a measured/verified outcome.

Must not:
- infer physical availability solely from a governance decision;
- let reputation, voting popularity or model confidence override applicable mandatory floors;
- convert a simulation run into authenticated evidence of a real event without a separate evidence/verification path.

### Human and community authority
The system may present trade-offs, disagreement and missing context. It cannot claim that the appropriate affected community has participated, that its preferences are known, or that a decision has democratic/legal legitimacy unless the exact upstream process/evidence supports that claim.

## Proposed data flow

```text
Mycelix
  versioned local-context / rights-floor / authority references
          | read-only, provenance-preserving projection
          v
Symthaea
  scenario design + candidate policies + uncertainty / held-out plans
          | exact frozen EvaluationRequestV1
          v
Symtropy settlement core
  deterministic synthetic execution + resource allocation + floors
          | SettlementRunReceiptV1
          v
Symthaea
  paired policy comparison + regret / robustness / abstention
          | recommendation + evidence refs, never actuation authority
          v
Human/community review
  authorized decision and explicit approval provenance
          |
          v
Mycelix / other operational systems
  append-only decision, governance and outcome lineage
```

There is deliberately no direct `simulation -> canonical physical state` write path and no default `model recommendation -> real actuation` path.

## First contracts

The Rust core currently owns a small subset; these remain proposed cross-repository envelopes until separately reviewed and frozen.

### `SettlementContextProjectionV1`
- exact context/profile identity and generation;
- domain profile: synthetic Earth district, remote Earth settlement, orbital, lunar or Mars;
- locally approved service-floor references (opaque IDs + currentness/provenance refs);
- requirements, preferences and uncertainty kept separately;
- source/model identity and validity window;
- unknown, absent, stale, disputed and restricted states retained explicitly;
- declared time horizon, units, coordinate/frame/time conventions;
- claim ceiling and permitted intended use.

Never place resident-level sensitive data in a synthetic fixture. Use synthetic cohorts and opaque refs for the prototype.

### `EvaluationRequestV1`
- exact scenario, policy, configuration and context projection identities/digests;
- exact solver/model adapters and versions, if any;
- declared inputs/observations and freshness;
- allowed action set, intervention latency and resource budget;
- hard constraints and separate advisory objectives;
- deterministic seed/time profile;
- required tests, stop conditions and negative controls;
- unsupported/out-of-domain semantics.

One frozen request must not give a learned candidate access to hidden future truth or inputs unavailable to the baselines.

### `SettlementRunReceiptV1`
- request/scenario/policy identity and exact digests;
- deterministic execution profile and output digest;
- component effective states, resource allocations and protected-reserve accounting;
- per-service floor status: `Met`, `Breached` or `Blocked`;
- missing/stale/conflicting/unsupported state;
- injected fault/event list and recovery outputs if modeled;
- all solver/model validity limits and report claim ceiling.

The SHA-256 values emitted by the current kernel are integrity/replay links, not signatures, authenticated source statements or proof of physical validity.

### `PolicyComparisonV1`
- paired run receipts using the same frozen scenarios and evidence;
- policy-specific available information, resource budget and tie-break rule;
- floor violations before any soft objective comparison;
- scenario-wise outcome vectors, worst-case and lower-tail values;
- regret and distributional results without invented scenario probabilities;
- incomplete/abstained comparisons made explicit;
- no scalar score permitted to mask a hard constraint breach or affected-group harm.

## Cultural plurality is a data-governance requirement

The city profile contract must distinguish:
1. environmental and infrastructure circumstances;
2. essential floors and legal/rights constraints;
3. preferences actually expressed by participants or authorized bodies;
4. modeled cohort assumptions;
5. unresolved disagreement, missing context and minority harm.

Do not infer tastes, religion, family structure, ability, values or priorities from identity proxies. Do not use synthetic profiles as evidence that a real culture or community has been represented adequately. For real-world use, require local review, correction/withdrawal paths, data minimization and explicit authorization of the use.

## Space-domain extensions

The same core may eventually support separate domain adapters, not a single generic "planet physics" setting:

- Earth district: water, electricity, buildings, drainage, heat, access and transport models.
- Remote Earth station: resource stock, maintenance crew and resupply delays.
- Orbital habitat: microgravity-specific life support, power, thermal control, fire and pressure boundaries.
- Lunar surface: vacuum, dust, radiation, thermal cycles, power and repair/external servicing.
- Mars surface: environment-specific life support, dust, communications latency, limited resupply and rescue constraints.

City service floors must not be reused as astronaut safety limits. Life-support, medical, pressure, fire, radiation and mission constraints require separately versioned engineering authorities and qualified physical models. Simulation output cannot waive an interlock or safety rule.

## First vertical and acceptance criteria

For the first synthetic city fixture:
- grid energy supports a water pump and clinic cooling, with backup supply and a separate potable-water reservoir;
- every resource uses a distinct synthetic unit code;
- protected reserve is never allocated to routine demand;
- a known grid failure can cause a water-floor breach while the clinic floor remains met, making distributional trade-offs visible;
- stale grid telemetry produces an indeterminate/blocked outcome when the unknown capacity could change the conclusion;
- declared sufficient backup may preserve both floors;
- unknown generations, duplicate IDs, unit mismatch and dependency cycles fail closed;
- permuting input record order does not change canonical digests;
- no reported pass implies actual urban resilience or physical flow validation.

For CI qualification: the new Rust crate's tests and workspace build must pass at the exact PR head. The authoring environment did not have Cargo/rustc, so tests are authored but not locally executed; until exact-head CI runs they are unverified.

## Sequence

1. Qualify the headless core and its synthetic tests without waiting on external system integrations.
2. Add an independent semantic checker and a frozen JSON fixture/corpus; keep negative controls permanent.
3. Coordinate the fixture vocabulary with Symthaea #7690 and the scenario identity/failure contracts in Symtropy #1595.
4. After the Civic Evaluation Pack and source contracts are qualified, build a consumer in the existing `symtropy-symthaea-adapter` / `symtropy-mycelix-bridge` boundary rather than coupling the core directly to large runtimes.
5. Add a read-only Symthaea policy-advisory adapter, with equal-information paired comparisons and explicit abstention.
6. Add Mycelix profile/floor/authority projection only using immutable references and valid-current authorization evidence.
7. Add one independent physical reference solver at a time; pre-register units, assumptions, tolerances, negative controls and claim scope.
8. Only after domain verification, consider human-reviewed decision support for a real settlement. No actuation path is in scope here.

## Non-goals

- No control/dispatch of real utilities, spacecraft, habitat life support, vehicles, pumps or grid.
- No live resident personal data in the synthetic harness.
- No implicit authority from Phi, model confidence, reputation, vote count or optimizer objective.
- No hidden changes to authorized service floors.
- No claim of validated urban digital twin, physical causal model, crew safety, legal compliance or community legitimacy.

## Claim ceiling

This document defines integration ownership and proposed message contracts. The current Rust kernel is a synthetic deterministic prototype. No Symthaea/Mycelix runtime adapter, calibrated city physics, field validation, deployment approval or space-habitat qualification is yet established.


## Implementation review update — 16 October 2026

The prototype now gives each resource supply and demand an explicit `allocation_pool_id`. Equal resource kind alone no longer permits implicit substitution across pools. This avoids the hidden assumption that every source of, for example, synthetic electrical-energy units is available to every consumer.

The review also identified and corrected a subtler issue: unknown capacity had initially been compared independently against each demand, so the same stale/unknown supply could make multiple mutually incompatible demands all appear individually feasible. The revised deterministic rule considers one **shared unknown-capacity upper bound per (resource kind, allocation pool)** in declared demand-priority order. Unknown capacity is never actually allocated; it only determines whether a minimum-demand outcome is `Indeterminate` or a known `Shortfall`. The report records the pool-wide unresolved upper bound before each demand's priority decision.

New authored regressions cover:
- Same-kind supply cannot be borrowed across explicitly distinct allocation pools.
- Stale/unknown capacity cannot be double-counted across competing demands.
- Higher-priority uncertainty may be reported as `Indeterminate` while a lower-priority demand is `Shortfall` when the shared best-case bound is insufficient for both.
- Input-order permutation preserves the report digest.

The core now has 13 authored Rust tests. They are **not claimed as executed**: the authoring environment has no local Cargo/rustc toolchain, and the exact-head GitHub workflow remains the verification path.

## Research alignment

These additions follow established resilience-planning concerns without claiming conformity or validation:
- NIST's Community Resilience Planning Guide treats infrastructure dependencies and cascading consequences as part of community recovery planning, and emphasizes community-set goals and recovery metrics: https://www.nist.gov/community-resilience/planning-guide and https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.1190GB-16.pdf
- NASA's Environmental Control and Life Support System spans pressure, fire safety, oxygen, ventilation, waste, water recovery, air revitalization and oxygen generation—illustrating why a habitat must be modeled as coupled subsystem domains, not a generic "city" profile: https://www.nasa.gov/reference/environmental-control-and-life-support-systems-eclss/
- Research on urban digital twins finds that interdisciplinary integration, consensual context and operationalization remain central gaps: https://doi.org/10.3390/smartcities8010032

These are design references, not model coefficients or certification standards for the prototype.

## Known remaining limitations

The current kernel is static and declarative. It does not yet model event time, resource production/consumption over multiple ticks, duration-limited backup use, repair completion, transient outages, restoration delays, physical capacity flows, or time-to-stable-recovery. Service-floor results are consequences of the declared graph and allocation rules, not estimates from electrical, hydraulic or thermal solvers. The next vertical should add a deterministic event timeline and explicit finite-duration interventions before using terms such as "recovery time."


### Unknown consumer / contested inventory guard

The kernel now also blocks allocation across a shared resource pool when any demand consumer in that pool has an unknown operational state and there is potentially available inventory. The directly unknown consumer is reported as `ConsumerStateUnknown`; otherwise-available consumers in that pool receive `PoolBlockedByUnknownConsumer`. The related service floors become `Blocked` rather than allowing a known consumer to consume inventory that an unknown higher- or lower-priority consumer may need.

This is intentionally conservative v1 semantics. It trades availability of a speculative allocation for explicit ambiguity until the contested pool's evidence is resolved. Other resource pools can continue independently.

A new regression tests this behavior. The kernel now has 15 authored Rust tests; exact-head hosted CI remains unverified.


### Strict deserialization guard

The scenario, component, dependency, resource, service-floor and report structs now use Serde's `deny_unknown_fields` contract, and `ComponentState` rejects unrecognized extra fields. Regression coverage exercises unexpected fields at the scenario, nested component and tagged operational-state levels. This prevents newly supplied values from being silently discarded during parsing. The policy is intentionally fail-closed; any future schema expansion needs an explicit version/review rather than relying on ignored extras.


### Explicit source-selection semantics

Supply-source choice now uses an explicit `allocation_priority` on each supply record, descending, with the stable supply ID only as a tie-break. Canonical alphabetical order is no longer allowed to act as an undocumented allocation policy. This priority is a declared synthetic policy input, not a real-grid dispatch rule or a guarantee of physical source preference.
