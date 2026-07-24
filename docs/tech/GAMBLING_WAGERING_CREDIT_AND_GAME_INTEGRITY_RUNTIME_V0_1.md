---
title: Gambling, Wagering, Credit, and Game Integrity Runtime
version: 0.1
status: implementation-spec
scope: wagering, games of chance and skill, odds, payout reserves, credit, debt, self-exclusion, integrity, cheating, advertising, taxation, and social consequence
owner: design/economy/simulation/security/safety
related:
  - ../canon/VICE_ECONOMIES_PLEASURE_CITIES_AND_NIGHTLIFE_CONTRACT_V0_1.md
  - ECONOMIC_LEDGER_MARKET_AND_INTEGRITY_RUNTIME_V0_1.md
  - SOCIAL_SIGNAL_RUMOR_REPUTATION_AND_PUBLIC_OPINION_RUNTIME_V0_1.md
  - ../canon/JUSTICE_HARM_ACCOUNTABILITY_AND_REPAIR_CONTRACT_V0_1.md
---

# Gambling, Wagering, Credit, and Game Integrity Runtime

## Purpose

This runtime supports recreational play, professional competition, public lotteries, private betting, casinos, informal wagers, and criminal markets without creating money from nothing or reducing harmful gambling to a morality icon.

## Core Principle

> **A wager transfers risk among real participants. It does not manufacture value, consent, or legitimacy.**

# 1. Wager Contract

Every wager has:

```text
wager_id
operator or peer set
game or event reference
rules version
stake assets
custody state
odds or payout function
acceptance time
settlement trigger
cancellation and dispute rules
jurisdiction
participant capacity checks
privacy scope
```

A wager is authoritative only after valid acceptance and custody reservation.

# 2. Game Classes

The runtime distinguishes:

- pure chance;
- mixed chance and skill;
- skill contests;
- sports and races;
- prediction markets;
- lotteries;
- pooled games;
- peer-to-peer wagers;
- house-banked games;
- informal social bets;
- prohibited wagers.

Different classes require different integrity evidence.

# 3. Stakes

Permitted stakes may include:

- currency;
- licensed tokens;
- goods in escrow;
- tournament entry;
- symbolic or social stakes explicitly consented to.

The runtime must reject wagers involving unauthorized claims on:

- another person's body;
- labor not voluntarily contracted;
- identity or source chain;
- citizenship;
- essential medication;
- housing or life support where loss would constitute coercion;
- minors;
- nonconsenting third parties;
- unique assets without valid custody authority.

# 4. House Operations

A house-banked operator stores:

```text
reserve assets
outstanding liability
maximum exposure
revenue and payout history
rules and odds versions
license state
auditor identity
worker and manager permissions
self-exclusion registry interface
complaint state
```

A venue may fail financially.

The game must support:

- payout suspension;
- insolvency;
- emergency reserve use;
- operator fraud;
- public receivership;
- worker takeover;
- negotiated settlement;
- reputational collapse.

# 5. Randomness and Outcome Integrity

Chance games require:

- deterministic seeded replay for authoritative simulation;
- concealed live seed or equivalent anti-prediction mechanism;
- signed rules version;
- auditable draw or device evidence;
- tamper events;
- post-settlement verification.

Skill games require:

- participant identity and eligibility;
- equipment state;
- officiating or sensor evidence;
- anti-collusion checks;
- appeal.

Randomness certification does not establish fair advertising or responsible credit.

# 6. Odds and Disclosure

Operators must define:

- house advantage or fee;
- payout table;
- variance and loss distribution where relevant;
- jackpot funding;
- rule changes;
- promotional conditions;
- withdrawal conditions.

IRIS may explain published odds and inconsistencies. It may not predict a fair random result or imply that a player is “due” to win.

# 7. Credit and Debt

Credit state includes:

```text
principal
interest or fee
collateral
affordability evidence
issuer
collection rights
repayment schedule
jurisdiction
hardship and dispute paths
```

The system distinguishes:

- ordinary consumer credit;
- professional bankroll financing;
- predatory credit;
- informal debt;
- coerced debt;
- fraudulent debt;
- debt secured by prohibited essentials.

Debt collection may create social and criminal consequences but cannot magically transfer unauthorized property.

# 8. Player and NPC Gambling State

The game may track:

- spending and loss history;
- time and session length;
- available funds;
- self-defined limits;
- current intoxication or distress;
- excluded venues or products;
- credit exposure;
- household obligations known to the person;
- personal meaning and goals.

The runtime does not assign a hidden moral worth or diagnose addiction automatically.

# 9. Self-Exclusion and Limits

A person may request:

- venue exclusion;
- product exclusion;
- spending limits;
- credit prohibition;
- cooling-off periods;
- trusted-contact support where consented;
- private IRIS warnings;
- withdrawal from personalized advertising.

Exclusion records are private and must not become employment, immigration, or social-ranking penalties.

# 10. Advertising and Personalization

Advertising may use only permitted information.

The runtime records:

- audience criteria;
- claims;
- incentives;
- exclusions;
- targeting provenance;
- vulnerability-sensitive restrictions;
- complaint and correction history.

Forbidden patterns include:

- concealing material odds;
- implying guaranteed recovery of losses;
- targeting a private self-exclusion state;
- using health or coercion data without authorization;
- presenting credit as winnings;
- simulating personal affection to increase spending.

# 11. Cheating, Collusion, and Advantage Play

The system distinguishes:

- device tampering;
- insider manipulation;
- collusion;
- identity fraud;
- information theft;
- rule exploitation;
- lawful skill or advantage play;
- operator retaliation against successful players.

Detection creates evidence and allegations, not automatic guilt.

# 12. Taxation and Public Dependence

Public revenue may derive from:

- operator profit;
- transaction volume;
- visitor levies;
- license fees;
- prize withholding;
- public lotteries.

The simulation tracks whether essential services become dependent on resident losses.

Political pressure may arise when effective harm reduction reduces public revenue.

# 13. Social Consequences

Wagering may affect:

- household trust;
- debt;
- employment;
- public scandal;
- professional status;
- criminal enforcement;
- treatment seeking;
- worker income;
- city reputation;
- political coalitions.

A large win is not necessarily a permanent upward trajectory. A large loss is not automatically a life-ending script.

# 14. Multiplayer

Multiplayer wagering requires:

- explicit participant acceptance;
- bounded stake types;
- escrow;
- anti-duplication;
- disconnect resolution;
- anti-harassment controls;
- no wagering of another player's persistent identity, body, or essential infrastructure rights.

Servers may disable or restrict player-to-player wagering independently from world fiction.

# 15. Acceptance Tests

Required tests:

- payouts are conserved and fully funded or enter an explicit default state;
- no wager settles before its event;
- rule changes do not retroactively alter accepted wagers;
- a participant can self-exclude without public disclosure;
- advertising cannot access forbidden private state;
- a casino can become insolvent;
- a skilled player is not automatically classified as a cheater;
- debt cannot seize unauthorized life support, identity, or body rights;
- replay reproduces outcomes from the authoritative seed and evidence;
- worldline forks do not duplicate staked unique assets.
