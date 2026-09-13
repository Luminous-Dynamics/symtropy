# ECON-02B — Atomic monetary settlement v0.1

## Status

This contract defines the first Symtropy authority that couples one balanced ECON-02 journal transaction to one monetary-supply issuance or retirement event atomically.

It strengthens ECON-02 without replacing it. Raw `FinancialBook` remains the lower-level accounting/supply representation. `MonetarySettlementLedger` is the stronger authority for state that claims journal/supply atomicity.

## Governing theorem

For every admitted settlement:

```text
balanced journal consequence
    + exact monetary supply event
    + exact currency/amount/cause binding
    + designated settlement account binding
    + registered authority identity binding

=> both histories commit together or neither history commits
```

The implementation stages both ECON-02 mutations on a cloned candidate book. The live settlement book changes only after both operations succeed and the resulting candidate financial book validates.

## Frozen base theorem

A settlement ledger begins from one complete validated ECON-02 `FinancialBook` called the base book.

The settlement history is the only transition path after that base within `MonetarySettlementLedger`.

```text
current settlement financial state
    == replay(base financial book, complete canonical settlement history)
```

The base book remains immutable inside this authority.

## Settlement identity

Each settlement has a unique bounded `SettlementId`.

Settlement history is one-based, canonical, gap-free, append-only, and bounded to 4,096 settlements in v0.1.

Duplicate settlement IDs fail closed.

## Exact coupling rules

V0.1 requires:

1. journal transaction currency == monetary-event currency;
2. journal transaction cause ID == monetary-event cause ID;
3. authorization monetary-authority ID == monetary-event authority ID;
4. authorization authority ID == the currency's registered monetary authority ID;
5. authorization actor == the currency's registered monetary-authority actor;
6. one explicit settlement account in the same currency;
7. settlement account class == `Asset`;
8. settlement account owner == issuance beneficiary or retirement source actor;
9. issuance settlement account posting == exact Debit of event amount;
10. retirement settlement account posting == exact Credit of event amount;
11. total transaction debits == event amount;
12. total transaction credits == event amount.

Rules 9-12 prevent a monetary event from being paired with an unrelated or larger balanced journal transaction.

## Retirement possession rule

Before retirement, the designated Asset account must have a current net Debit balance at least equal to the amount being retired.

This v0.1 rule prevents the stronger settlement path from retiring currency claims that the source account does not currently represent as held.

It is intentionally narrower than a general credit/overdraft model. Future banking policy may authorize other account structures through separate typed theorems.

## Atomicity mechanism

The settlement ledger never mutates the live financial book by sequentially posting a transaction and then mutating supply.

Instead:

1. validate settlement semantics against the current book;
2. clone current `FinancialBook` into a candidate;
3. post the exact journal transaction to the candidate;
4. apply the exact monetary-supply event to the candidate;
5. validate the complete candidate financial book;
6. append the settlement entry;
7. replace live state only with the completed candidate.

Any error before step 7 leaves the live financial book and settlement history unchanged.

## Authorization trust boundary

`SettlementAuthorizationRef` binds:

- exact monetary-authority ID;
- exact monetary-authority actor ID;
- external authorization evidence ID.

V0.1 verifies identity agreement with the registered currency definition.

It does **not** verify:

- digital signatures;
- key possession;
- capability-token authenticity;
- legal mandate;
- remote-authority liveness;
- external evidence truth.

A future Xenia/Mycelix bridge may satisfy this external trust edge with authenticated capabilities/signatures while preserving the settlement theorem.

## Replay theorem

`MonetarySettlementLedger::from_history` begins from the exact base financial book and executes every canonical settlement again.

`validate()` independently replays the entire settlement sequence from the frozen base and requires the reconstructed financial book to exactly equal current state.

Thus a locally valid ECON-02 journal/supply pair is not sufficient for ECON-02B authority unless it is also present in the canonical settlement history.

## Explicit non-claims

ECON-02B v0.1 does not establish:

- cryptographic authentication;
- legal payment finality;
- bank deposits as monetary aggregates;
- commercial-bank money creation;
- loans, interest, collateral, reserves, or capital requirements;
- escrow or delivery-versus-payment;
- contracts or invoices;
- FX conversion;
- taxes;
- market pricing;
- monetary policy;
- inflation or velocity models;
- universal correspondence between every ECON-02 account and aggregate currency supply;
- simulation-LOD conservation.

It establishes atomic coupling only for explicitly admitted issue/retire settlements through the stronger settlement authority.

## Successor boundaries

- **ECON-03** — simulation-tier promotion/demotion conservation for physical stock, financial claims, and settlement lineage.
- **ECON-04** — commodity units, grades, batches, quality, and serialized capital identity.
- **ECON-05+** — logistics, contracts, markets, households, firms, banking, taxation, institutions, externalities, technology, and macro observability.

Authenticated Xenia/Mycelix authorization should be added as a separate bridge/capability layer, not by weakening this contract into an implicit trust assumption.
