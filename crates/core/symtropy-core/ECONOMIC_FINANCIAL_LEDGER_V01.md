# ECON-02 — Double-entry financial and monetary authority v0.1

## Status

This contract adds deterministic financial accounting above ECON-00/01 without turning accounting balance into monetary issuance authority.

## Governing separation

V0.1 freezes two independent truths:

```text
financial balances == replay(complete balanced journal history)
monetary supply    == replay(complete authorized supply-event history)
```

and explicitly rejects the shortcut:

```text
balanced journal entry == permission to create money
```

The two histories are both append-only but neither silently mutates the other.

## Currency definition

A currency binds:

- `CurrencyId`;
- exact `MonetaryAuthorityId`;
- the actor represented as the monetary authority;
- integer minor-unit exponent.

All arithmetic is integer minor-unit arithmetic. V0.1 performs no floating-point currency arithmetic and no exchange-rate conversion.

## Financial accounts

Each account binds:

- persistent `FinancialAccountId`;
- owner actor;
- exact currency;
- account class: Asset, Liability, Equity, Revenue, or Expense.

V0.1 records cumulative debit and credit totals separately. It does not collapse them into an unsigned scalar whose sign convention could be mistaken across account classes.

## Journal theorem

Every `JournalTransaction` binds:

- persistent unique transaction ID;
- causal reference;
- exactly one currency;
- two or more postings;
- canonical ascending account order;
- each account at most once after aggregation;
- strictly positive integer posting amounts.

For every admitted transaction:

```text
sum(debits) == sum(credits)
```

using checked `u128` accumulation.

Every referenced account must already exist and must be denominated in the exact transaction currency. Cross-currency journal entries therefore fail closed rather than silently embedding an exchange rate.

Duplicate transaction IDs fail closed.

## Journal persistence theorem

Journal sequence numbers are canonical, one-based, and gap-free.

Materialized per-account debit/credit totals are only a projection. `validate()` independently replays the complete journal history and requires exact equality with materialized totals.

There is no generic balance setter.

## Monetary-supply theorem

Declared aggregate supply can change only through one of two typed events:

- `Issue`;
- `Retire`.

Each event binds currency, exact monetary authority ID, integer amount, source/beneficiary actor provenance, and causal reference.

The authority ID must equal the authority registered by the exact currency definition. Zero issuance/retirement fails closed. Retirement exceeding current declared supply fails closed.

Journal posting APIs cannot change monetary supply.

Monetary-supply APIs cannot change financial account balances.

This separation is intentional. A later settlement/reconciliation theorem must bind a supply event to any corresponding account consequences atomically before Symtropy claims end-to-end monetary settlement consistency.

## Monetary persistence theorem

Monetary-event sequence numbers are canonical, one-based, and gap-free.

Materialized aggregate supply is only a projection. `validate()` independently replays complete authorized monetary history and requires exact equality with materialized supply.

There is no generic supply setter.

## Authority boundary

`MonetaryAuthorityId` in v0.1 is a simulation/evidence authority reference, not cryptographic authentication. The kernel proves exact identifier binding and replay semantics. It does not prove that a real external signer possessed a key or had lawful monetary authority.

A later Mycelix/Xenia bridge may bind authority references to authenticated signatures/capabilities without changing this accounting theorem.

## Resource bounds

V0.1 is bounded to:

- 1,024 currencies;
- 65,536 accounts;
- 262,144 journal entries;
- 262,144 monetary events;
- 256 postings per journal transaction;
- IDs up to 256 UTF-8 bytes;
- minor-unit exponent no greater than 18.

All arithmetic and sequence increments are checked and fail closed on overflow.

## Explicit non-claims

ECON-02 v0.1 does not yet establish:

- account authorization/signatures;
- overdraft or credit limits;
- bank deposits versus central-bank money semantics;
- loans, interest, collateral, maturity, default, or bankruptcy;
- escrow or contract settlement;
- foreign exchange or exchange-rate discovery;
- market pricing;
- taxes or fiscal policy;
- monetary policy rules;
- reserve requirements;
- inflation or price-level dynamics;
- reconciliation between monetary supply and account claims;
- legal validity of ownership or authority;
- financial reporting standards compliance.

## Relationship to ECON-00/01

ECON-00/01 remains authoritative for physical economic stock ownership, custody, location, and stock-event history.

ECON-02 does not mutate stock lots. A later settlement contract may atomically compose stock transfer and financial journal consequences, but neither ledger may bypass the other's typed authority boundary.

## Successors

- ECON-02B: atomic monetary-supply/account-claim reconciliation and authenticated settlement authority.
- ECON-03: simulation-tier promotion/demotion reconciliation without duplicated or lost stock/claims.
- ECON-04: commodity specifications, units, grades, batches, serialized capital assets, and quality/provenance semantics.
- Later tranches: contracts, escrow, banks/credit, firms, markets, taxation, institutions, and macro policy.
