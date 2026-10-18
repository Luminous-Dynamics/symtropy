# Port-Call Counterfactual Value-Sharing Benchmark v0.1

**Purpose:** test whether a port-call coordination proposal creates a defensible system-level benefit and distributes it without violating declared participant acceptance thresholds.

**Status:** synthetic benchmark contract. It is not a contract engine, optimizer, payment rail, emissions verifier, or authority for operational effects.

## Why this is a separate problem

A faster or lower-emission voyage can shift financial value among shipowners, charterers, cargo interests, ports, terminals, and service providers. One participant may incur longer passage time or coordination expense while another receives the fuel savings. A locally optimal schedule can therefore be physically useful but commercially unadoptable.

The paradigm shift is to treat a proposed coordination policy as a **counterfactual with an explicit multi-party account ledger**, not merely as a schedule or a system-wide “savings” percentage. Evaluate physical outcomes, each party's net cost, transfers, emissions, and participation thresholds separately. Do not silently assume that aggregate savings mean every party benefits.

This composes with:
- [MARITIME-JIT-001 #1858](https://github.com/Luminous-Dynamics/symtropy/issues/1858), which owns joint voyage/arrival/berth/energy planning;
- [MARITIME-REC-001 #1867](https://github.com/Luminous-Dynamics/symtropy/issues/1867), which owns bounded contingent recovery;
- [MARITIME-CASCADE-PROPAGATION-BENCHMARK-V0.1](https://github.com/Luminous-Dynamics/symtropy/pull/1985), which measures downstream service impact; and
- Mycelix's participant authorization and binding-commitment workflows, which this benchmark must not replace.

The value-sharing benchmark owns cost-boundary qualification, counterfactual comparability checks, transfer-conservation checks, and the report of participant-level outcomes. It must not create a new voyage optimizer, canonical event schema, custody system, agreement authority, or payout mechanism.

## Empirical motivation and boundaries

The Global Maritime Forum's 7 October 2026 report studied 16 voyages across three ports using three different port-call optimization mechanisms. It reports different, context-specific outcomes: the available waiting time, vessel operating envelope, currents/weather, and contract structure affected savings. It also identifies split incentives and the difficulty of agreeing on a counterfactual baseline for benefit-sharing. The report itself emphasizes that the small sample should not be generalized as a universal expected saving.

Blue Visby's published description of its Blue GA mechanism benchmarks voyage accounts against a business-as-usual digital-twin scenario and considers fuel savings, passage prolongation costs, and demurrage. That is a useful documented design reference, not proof that another implementation reproduces the same method or has the same legal/commercial effects.

Research on the multi-port berth allocation problem with speed optimization has examined exact optimization and cooperative-game methods for distributing collaboration gains. Recent work continues to study cooperation and emissions in port-energy systems. These support the research direction, but no game-theoretic theorem or provider result is assumed transferable without a pinned, scope-matched implementation and test.

Research anchors:
- Global Maritime Forum, *Turning waiting time into savings: Lessons from Cargill's port call optimisation pilot* (7 Oct 2026): https://globalmaritimeforum.org/insight/turning-waiting-time-into-savings-lessons-from-cargills-port-call-optimisation-pilot/
- Blue Visby Solution, *How does it work?*: https://bluevisby.com/how-does-it-work/
- Martin-Iradi, Pacino & Ropke, *The multi-port berth allocation problem with speed optimization: Exact methods and a cooperative game analysis*: https://arxiv.org/abs/2011.15085
- Gong et al., *Cooperative game-based spatial-temporal coordination of multi-energy ports considering green hydrogen sharing* (Aug 2026): https://doi.org/10.1016/j.seta.2026.105223

These sources motivate requirements only. They are not data fixtures, independent validations, or guarantees of economic outcomes.

## Frozen counterfactual basis

Every comparison must bind:
- scenario identity and version;
- baseline and treatment receipt digests;
- a shared comparison-contract / delivered-service definition;
- a currency code and smallest currency unit (e.g. USD cents);
- a declaration that both cases deliver equivalent contracted service over the same horizon;
- a complete and symmetric cost boundary for the affected participants;
- declared participant identities and minimum acceptable benefit (reservation threshold);
- baseline and treatment fuel/emissions evidence and their scope; and
- evidence tier: synthetic, qualified historical replay, or unqualified.

The caller supplies SHA-256 digests. This v0.1 evaluator validates their syntax and carries them into its canonical report; it does not read or re-hash source bytes. An upstream receipt verifier must prove that each digest matches the referenced artifact. Even a verified hash establishes artifact identity, not truth. A synthetic fixture is qualified only for its declared arithmetic and accounting contract. Real historical or live operation data require separate rights, source lineage, coverage, model applicability, calibration, and reviewer gates. A digest alone cannot promote evidence.

The cost-boundary completeness flag is also a caller declaration in v0.1, not an independent line-item audit. Before real use, the cost boundary must be backed by a category-level manifest and complete baseline/treatment account rows.

Reject a comparison when delivered service differs, the cost boundary is incomplete, the evidence is unqualified, or receipts are malformed. Do not price carbon and then also claim the identical carbon value as an additional independent saving unless the accounting contract explicitly models the external payment and its counterparty.

## Account contract

For every participant, record:
- baseline net cost before coordination transfers;
- coordinated net cost before coordination transfers;
- transfer received, positive means the participant receives value and negative means it contributes value;
- minimum acceptable benefit, declared before comparing allocations.

For participant i:

    gross benefit(i) = baseline net cost(i) - coordinated net cost(i)

    net benefit(i) = gross benefit(i) + transfer received(i)

A proposed allocation must conserve transfers:

    sum(transfer received(i)) = 0

before separately itemized transaction costs, taxes, or external payments. Those costs must appear in the cost boundary; they must not disappear into a balancing residual.

The benchmark reports whether each participant's net benefit reaches its declared minimum. It does **not** determine that the threshold is fair, legally binding, or acceptable to the participant, and it does not generate or execute transfers. A participant's reservation threshold is an input, not inferred preference data.

Report fuel and CO₂ as physical quantities separate from monetary accounts. Monetary fuel/CO₂ valuations must have an explicit payer/receiver and must not be double-counted against the same account line.

## Comparison arms

A future full tournament should compare, on identical basis and bounded information:
- **B0 — no coordination / business-as-usual counterfactual**;
- **B1 — operator-only savings**, with other party costs visible rather than assumed zero;
- **B2 — a declared fixed-share transfer rule**;
- **B3 — a proportional-to-declared-gross-benefit rule**;
- **B4 — a negotiated / bargaining-model candidate** with all disagreement points explicit;
- **B5 — exact enumeration on small bounded transfer sets** as an independent oracle;
- **B6 — an optional Symthaea-guided policy ranking** at a matched compute/information budget, with no prior assumption of superiority.

The reference evaluator checks a proposed account allocation and now supports a **counterfactual sensitivity ensemble**: the same treatment receipt, comparison contract, participant set, treatment costs, transfer proposal, and reservation thresholds are tested against multiple distinct baseline receipts. It requires at least two distinct baseline receipts and reports worst/best total benefit, fuel reduction, CO₂ reduction, and each participant's worst/best net benefit. It does not assign probabilities to the baselines or collapse them into a fabricated expected saving. An empty ensemble, repeated baseline receipt, changed treatment receipt, changed transfer proposal, changed participant contract, or inconsistent treatment quantities rejects explicitly.

This remains a verifier, not an allocation optimizer or bargaining agent. A subsequent tournament can add independent policy generators and a category-level cost ledger after these contracts are qualified.

## Required adversarial controls

Include:
1. total gross benefit is positive but one participant loses without transfer;
2. transfers make every participant acceptable and sum to zero;
3. one-cent transfer-conservation mismatch;
4. missing participant or duplicated participant identity;
5. unequal delivered service / time horizon;
6. incomplete cost boundary, such as omitted passage prolongation or coordination costs;
7. unqualified evidence or invalid/missing artifact digests;
8. changed counterfactual baseline that flips the result;
9. treatment fuel or CO₂ exceeds baseline despite improved monetary outcome;
10. monetary carbon value supplied with no explicit transaction boundary;
11. input-row permutations;
12. a transfer allocation that meets only aggregate-system benefit but misses a participant's declared threshold;
13. numeric extremes and checked overflow;
14. costs, transfers, and reservation thresholds in incompatible currency/minor-unit bases.

## Proposed qualification gates (targets, not measured results)

- **100%** rejection of incompatible service boundaries, incomplete cost boundaries, unqualified evidence, malformed digests, duplicate parties, and non-conserving transfers.
- **100%** agreement with independent arithmetic on the frozen ledger corpus.
- **100/100** row-order permutations produce byte-identical canonical receipts.
- **Zero** unexplained transfer residuals or balancing plug entries.
- **100%** participant-level acceptance reporting; aggregate benefit must never mask a participant shortfall.
- **100%** separate reporting of physical emissions reduction and monetary value; zero double-counting in the frozen negative controls.
- **100%** of reported treatment savings identify the exact baseline, scope, delivered-service contract, and evidence tier.
- **100%** of baseline-sensitivity ensembles preserve the fixed treatment/participant contract and report worst/best participant outcomes without inventing scenario probabilities.
- For the later allocation tournament, **100%** agreement with an independent exact oracle on small bounded action sets; report Pareto feasibility, minimum normalized surplus, total surplus, allocation sensitivity, and computation cost separately.

No percentage emissions target is prescribed here. The 2026 pilot shows that waiting time and vessel envelope determine the opportunity; a case with no absorbable idle time can have no additional saving. Targets must be preregistered by a supported case family.

## Authority and claim ceiling

This is an offline cost-accounting and benchmark component. It does not negotiate, approve, sign, book, reserve, dispatch, alter a vessel plan, certify emissions, create a debt, move funds, or bind any participant. The report is a proposal analysis only. Mycelix and the relevant participants retain authority over permissions, legal terms, binding commitments and settlement.
