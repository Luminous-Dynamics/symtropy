---
title: Vice Regulation, Informal Markets, Corruption, and Organized Crime Runtime
version: 0.1
status: implementation-spec
scope: licensing, inspections, prohibition, informal services, organized crime, corruption, extortion, laundering, enforcement, public legitimacy, substitution, and institutional capture
owner: design/economy/society/justice/simulation
related:
  - ../canon/VICE_ECONOMIES_PLEASURE_CITIES_AND_NIGHTLIFE_CONTRACT_V0_1.md
  - ../canon/JUSTICE_HARM_ACCOUNTABILITY_AND_REPAIR_CONTRACT_V0_1.md
  - PUBLIC_ADMINISTRATION_SUCCESSION_AND_CORRUPTION_RUNTIME_V0_1.md
  - ECONOMIC_LEDGER_MARKET_AND_INTEGRITY_RUNTIME_V0_1.md
  - SOCIAL_SIGNAL_RUMOR_REPUTATION_AND_PUBLIC_OPINION_RUNTIME_V0_1.md
---

# Vice Regulation, Informal Markets, Corruption, and Organized Crime Runtime

## Purpose

This runtime makes vice regulation and organized crime emerge from law, demand, exclusion, service capacity, institutional incentives, and real networks.

It rejects a universal crime meter and the assumption that every informal institution is predatory or every licensed institution is legitimate.

## Core Principle

> **Illicit power grows where demand, exclusion, protection, and weak appeal meet. Removing the organization without replacing its function may deepen the condition that created it.**

# 1. Regulatory Domain

Each regulated activity has distinct state for:

```text
legal classification
license requirements
age and capacity boundary
location and zoning rules
health and safety rules
labor rules
tax and fee structure
inspection authority
privacy boundary
appeal path
penalty range
emergency powers
neighboring-jurisdiction conflict
```

A city may legalize gambling while prohibiting certain credit practices, or permit adult venues while restricting recording and corporate housing.

# 2. License State

A license records:

```text
holder
activity scope
site
validity period
conditions
qualified staff
inspection history
complaints
violations
appeals
financial bonds or reserves
political and ownership disclosures
```

Licensing may improve safety and accountability or become exclusionary rent extraction.

# 3. Inspection Runtime

Inspections require:

- lawful authority;
- staff time;
- access or warrant conditions;
- instrumentation;
- evidence custody;
- worker and resident privacy safeguards;
- findings and uncertainty;
- correction period or emergency action;
- appeal.

Inspectors may be skilled, overworked, corrupt, discriminatory, intimidated, or captured.

Passing an inspection proves only what was inspected under those conditions.

# 4. Demand and Substitution

When an activity is restricted, demand may:

- decrease;
- move to another district;
- move online or into private space;
- shift to a substitute product;
- enter an informal network;
- become more expensive;
- attract organized enforcement;
- create political backlash.

The runtime must not assume prohibition deletes demand.

# 5. Informal Institution Model

An informal network may provide:

- credit;
- transport;
- medicine;
- worker protection;
- document access;
- dispute resolution;
- product testing;
- private space;
- migration support;
- identity concealment;
- emergency housing.

It may also engage in:

- extortion;
- trafficking;
- violence;
- fraud;
- debt coercion;
- political capture;
- evidence destruction.

Service and harm are tracked separately.

# 6. Organized Group State

A group stores:

```text
group_id
membership and hierarchy
territory or network reach
services provided
revenue sources
protected clients
victims and coercive dependencies
assets and custody
alliances and rivalries
public reputation by audience
corrupt relationships
violence capability
internal factions
succession rules
```

No species, origin, profession, or poverty category creates criminality automatically.

# 7. Protection and Extortion

Protection may be:

- genuine security unavailable elsewhere;
- mutual aid;
- worker-controlled defense;
- compulsory payment under threat;
- bundled with debt or housing;
- tolerated because official enforcement is worse.

The runtime records what threat is being reduced, who created it, who pays, and who can appeal.

# 8. Corruption Graph

Corruption is represented through relationships and transactions:

```text
gift or payment
favor
hidden ownership
selective inspection
procurement steering
information leak
evidence suppression
license acceleration
political donation
post-office employment
family or household tie
coercive threat
```

A suspicious relationship creates inquiry pressure, not automatic guilt.

# 9. Money Laundering and Asset Legitimacy

Laundering attempts to obscure provenance through:

- false venue revenue;
- shell ownership;
- inflated invoices;
- art, luxury, or gambling transactions;
- cross-jurisdiction settlement;
- identity or source-chain fraud;
- mixed legitimate and illicit supply chains.

The economic ledger preserves custody and uncertainty. It does not magically label all mixed funds as clean or dirty.

# 10. Enforcement Models

Possible enforcement bodies include:

- municipal regulators;
- worker safety councils;
- public-health inspectors;
- financial auditors;
- police or security services;
- community accountability groups;
- corporate compliance units;
- machine witnesses;
- joint interregional authorities.

Each has authority, competence, bias, incentives, and appeal constraints.

# 11. Selective Enforcement

Enforcement may disproportionately target:

- migrants;
- informal workers;
- poor districts;
- unpopular cultures;
- political opponents;
- independent venues;
- machine or alien residents;
- workers lacking recognized credentials.

The game must represent selective enforcement as an institutional pattern with evidence and consequences, not merely a villain’s dialogue.

# 12. Community Legitimacy

A group’s legitimacy varies by audience and domain.

Residents may distrust its violence while relying on its ambulance route.

Workers may value its protection while opposing its debt collection.

Officials may publicly condemn it while privately depending on its information.

Plural legitimacy prevents one reputation number from resolving the conflict.

# 13. Reform and Removal

Possible interventions include:

- legalization;
- decriminalization;
- public service replacement;
- worker cooperative transition;
- targeted prosecution;
- financial receivership;
- witness protection;
- anti-corruption reform;
- amnesty and disarmament;
- community accountability;
- infrastructure investment;
- negotiated coexistence.

Destroying a group’s leadership without replacing transport, medicine, credit, shelter, or dispute resolution can create a succession war or service collapse.

# 14. Player-Owned Vice City Risks

Player governance may itself become corrupt through:

- dependence on tax revenue;
- favored operators;
- hidden campaign finance;
- selective zoning;
- surveillance contracts;
- emergency powers;
- pressure to conceal incidents;
- personal ownership of regulated venues;
- conflicts between player profit and civic duty.

The player does not receive immunity from the systems they design.

# 15. Multiplayer Boundaries

Player organizations may participate only within server rules.

The simulation must prevent:

- real-money coercion;
- nonconsensual persistent debt between players;
- doxxing through in-world privacy systems;
- sexual harassment framed as roleplay;
- ownership of another player’s identity or body;
- punitive exclusion without moderation appeal where platform policy requires it.

# 16. Acceptance Tests

Required tests include:

- prohibition changes demand rather than deleting it;
- an informal clinic can lose legitimacy through extortion while retaining medical value;
- a licensed venue can pass inspection through corruption and later fail causally;
- enforcement can become selectively biased through actual policy and staffing;
- removing an organization without service replacement can worsen outcomes;
- corruption evidence preserves uncertainty and custody;
- organized groups contain internal factions;
- public revenue dependence can distort regulation;
- a reform can reduce harm while increasing another pressure;
- long-absence simulation preserves ownership, service, corruption, and succession changes.
