---
title: Inheritance, Office, Property, Reconstitution, and Claim Adjudication Runtime
version: 0.1
status: implementation-spec
scope: succession claims, wills, estates, offices, institutional assets, reconstituted claimants, machine forks, hearings, interim authority and enforcement
owner: engineering/design/civic/legal/economy
related:
  - ../canon/HOUSEHOLDS_LINEAGES_DYNASTIES_GUILDS_AND_CIVIC_SUCCESSION_CONTRACT_V0_1.md
  - ../canon/DEATH_RECONSTITUTION_AND_SOURCE_CHAIN_RECOVERY.md
  - PLAYER_PROMISE_OFFICE_REPUTATION_AND_LEGACY_RUNTIME_V0_1.md
  - SETTLEMENT_FOUNDING_CHARTER_INSTITUTION_AND_PUBLIC_SERVICE_RUNTIME_V0_1.md
  - CIVIC_TIME_PROMISE_SCHEDULING_AND_CAUSAL_EVENT_RUNTIME_V0_1.md
---

# Inheritance, Office, Property, Reconstitution, and Claim Adjudication Runtime

## Purpose

This runtime resolves what happens when a person dies, disappears, returns, forks, retires, loses capacity, completes a term, or leaves obligations that outlive them.

It prevents one generic inheritance rule from incorrectly transferring:

- private objects;
- household access;
- public office;
- professional credentials;
- corporate control;
- route authority;
- source-chain standing;
- debts;
- custody;
- promises.

> **Continuity of person, continuity of ownership, and continuity of authority are separate claims.**

# 1. Claim Domains

Every claim specifies one or more domains:

- personal identity;
- private movable property;
- land or habitat use;
- household account;
- intellectual or cultural stewardship;
- business ownership;
- cooperative share;
- public office;
- professional license;
- public-service authority;
- archive custody;
- guardianship or care;
- debt;
- contract;
- route authority;
- machine body or hardware;
- source-chain record;
- title or ceremonial role.

A ruling in one domain does not automatically settle another.

# 2. Core Entities

## 2.1 Claim

```text
claim_id
worldline_id
claimant_id
subject_domain
subject_ref
claim_basis
filed_event
requested_remedy
evidence_refs
opposing_claims
current_status
jurisdiction
privacy_class
```

Claim bases include:

- will;
- charter;
- election;
- appointment;
- adoption;
- kinship;
- household compact;
- apprenticeship;
- contract;
- contribution;
- possession;
- emergency service;
- source-chain continuity;
- fork descent;
- public trust;
- custom;
- alien legal translation;
- equitable reliance.

## 2.2 Estate

An estate is a temporary custody structure for unresolved private assets and obligations.

```text
estate_id
decedent_or_missing_id
opening_event
asset_refs
debt_refs
private_record_refs
custodian
beneficiary_claims
care_obligations
maintenance_costs
closure_conditions
```

## 2.3 Office Tenure

```text
office_id
institution_id
holder_id
mandate_basis
term_start
term_end
interim_rules
succession_method
conflict_rules
removal_rules
continuity_plan
```

Office is not stored inside a character inventory.

## 2.4 Continuity Claim

```text
continuity_claim_id
prior_person_ref
claimant_person_ref
source_chain_evidence
memory_overlap
body_continuity
legal_status
social_recognition
conflicting_claimants
adjudication_state
```

# 3. Trigger Events

The runtime opens succession when it receives:

- confirmed death;
- presumed death threshold;
- missing-person declaration;
- retirement;
- resignation;
- term expiry;
- incapacity;
- removal;
- dissolution;
- machine fork recognition;
- reconstitution;
- duplicate continuity discovery;
- institutional merger;
- route disconnection;
- worldline migration.

Different triggers produce different interim rules.

# 4. Interim Authority

Public services cannot wait for every dispute to finish.

Interim authority may be assigned through:

- deputy succession;
- council rotation;
- emergency custodian;
- worker committee;
- automated safe-mode authority;
- court appointment;
- constituency selection;
- alien-equivalent process.

Interim authority must record:

- scope;
- duration;
- prohibited irreversible actions;
- oversight;
- audit;
- compensation;
- replacement process.

An interim holder cannot silently convert emergency custody into permanent ownership.

# 5. Wills and Directives

A valid directive may address:

- private possessions;
- gifts;
- household shares;
- funeral or memorial preferences;
- archive access;
- intellectual stewardship;
- dependents;
- machine maintenance;
- unfinished projects;
- private messages.

A will cannot automatically transfer:

- another person's consent;
- public office;
- professional competence;
- a public utility;
- a dependent as property;
- an autonomous machine person;
- alien territory;
- source-chain truth.

# 6. Household Continuity

When a household member dies or leaves, the household may need:

- continued housing;
- access to shared funds;
- medication;
- childcare;
- funeral labor;
- debt relief;
- reassignment of care;
- privacy protection;
- time before asset division.

The simulation must avoid instantly liquidating a household because its highest-earning member died.

Emergency protections may freeze eviction, utility cutoff, or account closure.

# 7. Reconstitution

A reconstituted person may file claims based on personal continuity.

Possible outcomes:

- fully recognized personal continuity;
- recognized person with limited legal continuity;
- disputed continuity pending hearing;
- recognized fork or descendant person;
- insufficient evidence;
- multiple valid persons with shared ancestry.

Even full recognition does not automatically restore:

- completed office terms;
- marriages or intimate relationships;
- property lawfully distributed;
- guardianship;
- professional clearance;
- private access revoked during absence;
- command authority.

Possible remedies include:

- compensation;
- return of undistributed property;
- shared stewardship;
- new election;
- renewed credential review;
- mediated household agreement;
- public correction of identity records.

# 8. Machine Forks and Copies

Machine continuity claims require separate analysis of:

- source ancestry;
- active-time overlap;
- memory divergence;
- embodiment;
- chosen identity;
- prior obligations;
- consent to merge;
- hardware ownership;
- labor performed after fork.

If two forks are persons, neither is merely the other's backup.

Shared pre-fork property may be:

- divided;
- held jointly;
- assigned by use;
- compensated;
- converted to a lineage trust.

Pre-fork guilt or debt does not automatically create unlimited joint punishment.

# 9. Public Office and Reconstitution

If an office-holder returns after lawful succession:

1. personal continuity is assessed;
2. the office's mandate history is reviewed;
3. the successor's legitimacy is preserved unless law provides otherwise;
4. the returned person may stand for office through current rules;
5. emergency powers are not resurrected;
6. private loyalty does not override the charter.

This supports dramatic conflict while preserving institutional reality.

# 10. Professional Credentials

A credential may lapse or require review after:

- long absence;
- death and restoration;
- major body change;
- memory loss;
- legal discipline;
- outdated standards;
- alien-jurisdiction transition.

Review should test relevant competence rather than humiliate the claimant.

A famous lineage name cannot substitute for current ability.

# 11. Claim Adjudication

A hearing may involve:

- claimant testimony;
- household testimony;
- source-chain evidence;
- machine witnesses;
- charter records;
- professional assessment;
- prior wills;
- public-service necessity;
- alien translation experts;
- privacy-protected evidence;
- historical custom;
- equitable reliance.

The adjudicator records:

```text
facts_found
facts_contested
law_or_custom_applied
rights_affected
interim_orders
final_remedy
appeal_path
review_date
```

The game must not collapse complex claims into one persuasion roll.

# 12. Evidence Quality

Evidence may be:

- authenticated;
- corroborated;
- partial;
- stale;
- altered;
- privately held;
- inadmissible but informative;
- culturally mistranslated;
- unavailable because a route closed.

IRIS may organize evidence and uncertainty. It does not decide the case.

# 13. Property and Contribution

Property claims may account for:

- formal title;
- labor contribution;
- maintenance;
- household reliance;
- public subsidy;
- ecological obligation;
- community use;
- occupation during abandonment;
- improvement;
- historical theft.

A player-built structure may become a public landmark or inhabited home. The founder's return does not automatically evict later residents.

# 14. Debt and Obligation

Death or succession may affect:

- personal debt;
- secured debt;
- household debt;
- corporate debt;
- public obligation;
- care debt;
- moral promise.

The runtime must not automatically assign every debt to biological heirs.

A successor may accept obligations through:

- contract;
- estate benefit;
- office;
- institutional continuity;
- explicit assumption.

# 15. Alien Claims

Alien succession may not map to individual inheritance.

Examples:

- an oceanic memory current claims custody of a route pattern;
- a lithic assembly recognizes authority only after decades of continuity;
- a symbiotic relationship dissolves and creates several successor persons;
- a copied polity treats forks as descendants;
- a migratory culture assigns custody to a route rather than a person.

The runtime stores translation confidence and allows incompatible remedies.

# 16. Player Experience

The player may participate as:

- claimant;
- heir;
- successor;
- witness;
- custodian;
- professional evaluator;
- office-holder;
- mediator;
- investigator;
- household member;
- unrelated citizen affected by service continuity.

The most powerful choice may be to decline a claim.

# 17. Failure Modes

## Office as loot

A dead ruler's title transfers to the player's inventory.

**Prevention:** office tenure and mandate records.

## Resurrection rewind

A returned person automatically restores all relationships and possessions.

**Prevention:** domain-specific claims and current rights.

## Bloodline determinism

Biological relation overrides adoption, contribution, charter, and consent.

**Prevention:** plural claim bases.

## Litigation abstraction

A single dialogue roll resolves identity and public-service continuity.

**Prevention:** evidence, interim orders, constituency effects, appeal.

## Endless paralysis

A dispute disables essential services for months.

**Prevention:** bounded interim authority.

# 18. Minimum Proof

The initial benchmark should include:

- a founder's death;
- lawful interim succession;
- a later reconstitution;
- a private-property claim;
- an office claim rejected without denying personhood;
- an adopted apprentice with a professional claim;
- a machine fork with shared ancestry;
- a household protected from eviction;
- an appeal;
- a peaceful final arrangement;
- one branch where conflict escalates.

## Runtime Maxim

> **The returned person may be real. The successor's years were real too.**
