# Civilizational Authority Contract v0.1

Status: normative design contract for future civilization-scale simulation work.

This document defines the authority boundaries that must hold if Symtropy is to
support persistent political, economic, social, military, and interplanetary
history without collapsing distinct kinds of state into one convenient scalar.

The central rule is:

> World truth, observation, belief, public claim, institutional record, legal
> authority, effective control, ownership, and legitimacy are distinct state.
> No subsystem may silently promote one into another.

This contract is intentionally implementation-light. It does not create a
politics engine, diplomacy engine, economy, combat system, Mycelix authority,
or narrative generator. It freezes the semantic boundaries later crates must
respect.

## 1. Authority classes

### 1.1 World truth

Canonical simulation state describing what the authoritative world currently
contains or what actually occurred according to the owning simulation authority.

Examples:

- a bridge span is physically impassable;
- a freighter contains 420 tonnes of grain;
- a resident died at canonical tick 8,102;
- a station reactor failed because a coolant pump seized.

World truth is not automatically visible to any resident, institution, player,
or AI.

### 1.2 Observation

A bounded report produced by a person, sensor, document, institution, or other
observer.

An observation must be capable of carrying at least:

- observer/source identity;
- subject identity;
- canonical observation or receipt time;
- provenance/evidence references;
- confidence or uncertainty semantics when applicable;
- disclosure boundary when applicable.

Observation is evidence about world truth, not world truth itself.

### 1.3 Belief

An actor- or institution-scoped proposition derived from observations, prior
beliefs, inference, testimony, doctrine, memory, or misinformation.

Two actors may hold incompatible beliefs while canonical world truth remains
unchanged.

Belief is not authority and cannot mutate world truth merely because a powerful
actor believes it.

### 1.4 Public claim

A proposition deliberately asserted to an audience by an actor or institution.

A public claim may differ from the claimant's private belief. This distinction is
required for diplomacy, propaganda, deception, negotiation, journalism, and
political accountability.

The simulation must therefore permit:

```text
world truth != private belief != public claim
```

without treating the mismatch as an engine error.

### 1.5 Institutional record

A proposition or status recorded by an institution under that institution's
rules.

Examples:

- a court records Mara as lawful heir;
- a registry records Helix Cooperative as the owner of Dock 7;
- an admiralty records Fleet Group Three as combat-ready;
- a news service records an event as disputed.

A record is evidence of what an institution recorded. It does not necessarily
prove the underlying proposition true.

### 1.6 Legal or normative authority

A scoped entitlement to make a decision, issue an order, dispose of an asset,
represent an institution, or exercise a defined power under some governing
rule set.

Authority must be scoped. "Is ruler" is too weak. Later implementations should
be able to distinguish authority to:

- command a fleet;
- appoint an office;
- spend a treasury;
- sign a treaty;
- transfer an asset;
- issue regulations within a jurisdiction;
- authorize emergency action.

A legal authority claim may be disputed, unrecognized, stale, suspended, or
impossible to exercise.

### 1.7 Effective control

The practical ability to cause an asset, territory, institution, force, network,
or process to act.

Effective control is intentionally distinct from legal authority and ownership.

Example:

```text
lawful owner:      Helix Cooperative
recognized ruler: Mara Venn
fleet commander:  Admiral Kael
physical control: Kael-aligned marines
```

All four facts may simultaneously be valid.

### 1.8 Ownership

A claim recognized by some property/asset authority that an actor or institution
holds one or more rights over an asset.

Ownership must not be collapsed into custody, operation, possession, beneficial
interest, creditor interest, or physical control.

Later asset work should be able to model those relations separately.

### 1.9 Legitimacy

An actor- or population-relative judgment that an officeholder, institution,
rule, claim, or exercise of power ought to be accepted.

Legitimacy is not one global scalar. It may differ by population, institution,
ideology, religion, class, region, species, cohort, or other constituency.

A claimant may therefore be:

- legally recognized but broadly considered illegitimate;
- popularly legitimate but legally unrecognized;
- militarily dominant but institutionally unrecognized;
- recognized by one polity and rejected by another.

## 2. Required non-equivalences

Future implementation must preserve the following non-equivalences:

```text
observed(X)        != true(X)
believed(X)        != observed(X)
claimed(X)         != believed(X)
recorded(X)        != true(X)
recognized(X)      != true(X)
authorized(A, X)   != controls(A, X)
owns(A, X)         != controls(A, X)
controls(A, X)     != legitimate(A, X)
legitimate(A, X)   != authorized(A, X)
```

No convenience API may expose one boolean whose meaning silently combines these
relations.

## 3. Civil conflict must emerge from incompatible authority graphs

A civil war, coup, succession crisis, secession, occupation, or constitutional
crisis should not require a privileged `civil_war = true` state to become real.

It can emerge when incompatible claims and control relations coexist.

Example:

```text
Mara:
  lawful succession claim: yes
  parliament recognition:  yes
  treasury control:         yes
  fleet control:            no

Kael:
  lawful succession claim: disputed
  parliament recognition:  no
  frontier recognition:    partial
  fleet control:            yes
```

This state already contains the material of a succession crisis. A later conflict
system may consume it, but must not manufacture the underlying disagreement.

## 4. Institutions are generic

The core civilization model must not privilege one political form.

The foundational abstraction should be a generic institution or organization,
not `House`, `Kingdom`, `Corporation`, `Church`, `Nation`, or `Guild`.

Specific institutional forms should be compositions of generic properties such
as:

- membership;
- offices;
- jurisdictions;
- decision procedures;
- succession/selection procedures;
- assets;
- obligations;
- recognized authority grants;
- external recognitions;
- internal constituencies.

This allows hereditary houses, democratic federations, corporations, religious
orders, cooperatives, military commands, AI collectives, ship crews, and forms
not anticipated by the engine to inhabit one simulation vocabulary.

## 5. Succession creates claims, not instant truth

A succession or office-selection process must produce a candidate result or
claim under a named rule set. It must not automatically prove universal
recognition, legitimacy, or effective control.

The following may therefore diverge after one office transition:

```text
selected candidate
lawful claimant
institutionally recorded officeholder
externally recognized officeholder
effective controller
popularly legitimate officeholder
```

Disagreement between these views is valid simulation state.

## 6. Information is scoped

Civilization-scale AI must not read authoritative world state by default.

Actors should make ordinary decisions from information available through their
observations, records, communications, memories, and inferences.

A privileged omniscient read path may exist for debugging, verification, and
simulation administration, but it must not be the normal cognitive interface
for residents or institutions.

This extends the observer-scoped knowledge principles already present in
`symtropy-residents`.

## 7. History is evidence-bearing causality

Historical events must be able to preserve:

- stable event identity;
- canonical time;
- actor when any;
- observer/provenance when any;
- subjects and locations when applicable;
- direct causal parents;
- evidence or source references;
- typed payload.

A historical explanation must be derived from recorded causal/evidence state,
not invented after the fact to fit the current world.

The target question is:

> Why did this state become true?

The engine should be able to traverse recorded causes and show the strongest
available answer while preserving uncertainty and disagreement where the record
is incomplete.

## 8. Storage representation does not rewrite history

Snapshots, journal segmentation, compaction, archival, indexing, or migration
may change how history is stored. They must not silently change historical
identity or rewrite settled event content.

A later persistence extension should therefore distinguish:

```text
historical identity
storage location
storage segmentation
indexing/materialization
```

## 9. Multiscale simulation must not fabricate identity

Coarse simulation may represent anonymous populations, aggregate inventories,
or institutional statistics. Promotion to greater fidelity must not imply that
previously unrepresented exact persons or assets had detailed canonical history
that was never simulated or retained.

Persistent exact identities remain exact across fidelity changes. Aggregate
population may be refined through explicit generation/materialization semantics
that preserve known totals and constraints without inventing unsupported past.

This is a civilization-specific application of Symtropy's broader fidelity and
information-sufficiency discipline.

## 10. Mycelix boundary

Mycelix may provide real-player identity, governance, contracts, permissions,
provenance, and organization infrastructure.

Mycelix governance semantics must not be silently imposed on fictional
civilizations. A fictional monarchy, junta, corporation, commune, or religious
order must remain representable even when real player collaboration is secured
through Mycelix.

The bridge is therefore:

```text
fictional civilization semantics -> generic Symtropy institution model
real player collaboration        -> optional Mycelix-backed implementation
```

not:

```text
every fictional institution -> one Mycelix governance model
```

## 11. Authority ownership

Civilization code must consume authoritative outputs owned by other systems
rather than re-declare them.

Examples:

- physical state remains owned by physical/world simulation authorities;
- resident embodied/knowledge state remains owned by resident-domain code;
- ecological exact/coarse transitions remain owned by Living World authority;
- engineering truth remains outside civilization politics;
- network transport is not political authority;
- renderer/UI state is never canonical political truth.

Adapters may project these outputs into civilization decisions, but may not mint
stronger authority by wrapping them.

## 12. V0 acceptance examples

A conforming future civilization implementation must be capable of representing
all of the following without contradiction:

1. A claimant is legal heir but does not control the fleet.
2. A fleet commander controls military assets but owns none of them.
3. A corporation owns a station while an occupying force controls it.
4. Two institutions record incompatible accounts of the same historical event.
5. A government publicly blames sabotage while its private intelligence service
   assigns greater probability to mechanical failure.
6. A treaty partner recognizes one successor while a domestic institution
   recognizes another.
7. A popular leader has high legitimacy among one constituency and low
   legitimacy among another.
8. A distant governor exercises delegated emergency authority because
   communication latency prevents timely central decisions.
9. A coarse population can be simulated without inventing millions of exact
   historical persons.
10. A saved/compacted campaign can reconstruct the same settled causal history.

## 13. Deliberate non-features

This contract does not yet define:

- an `Institution` runtime type;
- an office/succession implementation;
- ownership registries;
- treaty schemas;
- economic production graphs;
- combat or war aims;
- orbital mechanics;
- communication-delay algorithms;
- population refinement algorithms;
- UI;
- player progression;
- a narrative generator.

Those are successor tranches. They should remain small and independently
reviewable while preserving the distinctions frozen here.

## 14. North-star theorem

A civilization-scale Symtropy world should be able to produce political history
without a privileged narrative script because people, institutions, information,
resources, authority, control, and consequences interact causally.

The engine should simulate the conditions from which stories emerge, not encode
prewritten story outcomes as authoritative world state.
