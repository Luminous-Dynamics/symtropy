# LBE Kernel

A dependency-free Rust prototype for two foundational mechanisms in the Love-Based Economy benchmark programme:

1. Rights-neutral intake and explicit scarcity allocation for essential needs.
2. Double-entry mutual-credit accounting with deterministic pooled demurrage.

The crate is deliberately isolated from the parent Symtropy Cargo workspace. It has no third-party dependencies and can be checked without Bevy, Holochain, GPU support, Symthaea, a conductor, or a network.

## Run

From the Symtropy repository root:

```sh
cargo fmt --manifest-path crates/lbe-kernel/Cargo.toml --all -- --check
cargo test --manifest-path crates/lbe-kernel/Cargo.toml --locked
cargo clippy --manifest-path crates/lbe-kernel/Cargo.toml --all-targets --locked -- -D warnings
```

## Need intake

The public request type contains only:
- a bounded category;
- an urgency declaration;
- an integer quantity.

The caller separately supplies an `AuthenticatedSubject` produced by an identity adapter. The adapter must derive the subject from an authenticated principal; it must never trust a requester identifier in an untrusted form. The kernel does not authenticate people itself.

The API has no fields or parameters for:
- wealth, income, collateral, or token balances;
- MYCEL / reputation / social trust scores;
- SAP or TEND history;
- Phi, consciousness, cognitive, or psychological measurements;
- prior contribution, civic participation, or reciprocity offers.

One open need per subject/category prevents simple queue-splitting in this first model. A completed case may be followed by a new request. More elaborate recurrence, household, delegated and assisted-intake semantics are deliberately not guessed here. Category-specific canonical units and realistic maximum quantities remain to be defined; the current `u64` quantity type is not a production validation policy.

## Allocation policies

- `PriorityThenOldest`: urgency first, then trusted receipt timestamp, then stable case ID.
- `OldestFirst`: first accepted, first served.
- `ProRata`: proportional to outstanding quantity, using largest-remainder rounding and a stable identifier tie-break.

These are comparative policy arms, not universal fairness claims. A real service must publish its scarcity rules and provide correction and appeal. The kernel only guarantees that committed allocations do not exceed the supplied physical quantity.

## Mutual-credit ledger

All balances are signed integer units. Every transfer debits one account and credits another by exactly the same amount. Issuance and redemption are explicit events against a system issuance account. Redemption can retire only an existing positive member balance; it cannot create debt.

Demurrage is a **pooled community levy** on aggregate positive member balances:
- fractional accrual is held once at the ledger level;
- the aggregate levy is independent of how a positive balance is split across member accounts;
- whole-unit charges are distributed by largest remainder with a stable account-ID tie-break;
- the commons treasury receives the charge;
- negative balances are not automatically decayed or forgiven.

Pooled carry has an explicit incidence trade-off: a fraction accrued in one interval can be collected from the current positive-balance population in a later interval. This avoids account-fragmentation changes in total levy, but it is not an individual account's exact time-weighted tax liability. That policy needs evaluation before any real use.

Debt relief is separate. A reserve must first be funded by an explicit transfer. Forgiveness then moves funds from the reserve to a debtor, reducing that member's negative balance while preserving the aggregate ledger.

## Bilateral consent

The `consent` module is a pure state machine for a match or service exchange:

- proposal terms are represented by a fixed 32-byte digest;
- the host adapter authenticates the caller and recomputes the digest from canonical immutable terms;
- two distinct parties must independently accept that same digest;
- one acceptance is not enough to settle;
- changed terms require a new proposal digest and new acceptances;
- a party can decline or revoke before settlement;
- pending proposals expire;
- settlement is available only after both acceptances and is idempotent in the model.

The module does **not** cryptographically authenticate an actor, verify a digest, or prove that agreement was free of duress. It is unsafe to trust caller-supplied subject IDs or digest bytes. Holochain adapters must use the authenticated `action.author`, verify the referenced Need/Offer actors, and hash a canonical representation of all material terms. A separate, private refusal/revocation and coercion-review path is required where dependency or power imbalance exists. External settlement must be journaled and idempotent. Gratitude, ratings, reciprocity and public testimony are not part of consent.

## Dissonance and paradox

The `dissonance` module records empirical disagreements, normative conflicts, resource trade-offs, temporal tensions, structural paradoxes and interpretive differences without assigning people a moral or psychological score.

It preserves position records, source attribution, rationale and evidence references. Process safeguards (faithful representation, accessible affected-party voice, dissent retention, evidence review, appeal and power disclosure) are explicit. Corrosive hazards such as retaliation, coercion, essential-access conditioning, evidence suppression, misrepresentation, intimidation and surveillance retain their finding state: reported, corroborated, substantiated, refuted or unresolved.

The model distinguishes dispositions:
- hold a tension open;
- integrate all recorded positions;
- provisionally prioritize one position while retaining all others;
- sequence two positions with explicit retention and a rollback trigger;
- run a bounded experiment against named positions;
- contextualize every position with an explicit context rule;
- resolve an empirical disagreement through a reviewed evidence bundle.

Normative conflict cannot be marked as empirically resolved. Consequential action requires all process safeguards and no active or unresolved corrosive hazard.

**Process changes are now event-sourced in the kernel prototype.** Safeguard snapshots, hazard reports, hazard-finding transitions and dispositions each append an attributed event with a sequence number, monotonic timestamp, predecessor receipt reference and current receipt reference. Hazard finding changes require new evidence references, and the prior finding remains in the event history. The record can deterministically replay a complete event stream from its initial position set and reject broken receipt links, repeated receipts and backdated events.

The kernel does not authenticate actor IDs or verify cryptographic receipts. The host must bind the genesis position set and each canonical event to signed/content-addressed receipts, verify the authenticated principal, enforce authorization and preserve the append-only store. The sequence and receipt-link checks in this in-memory model are not, by themselves, tamper-proof storage.

This remains a pure model. Safeguard booleans require external evidence, and the model is not an ethics review, truth oracle, facilitation tool, legal authorization mechanism or proof that consent is free of coercion. Strong anger, protest or minority dissent are not automatically corrosive.

See [LBE-BENCH-003](https://github.com/Luminous-Dynamics/symtropy/issues/3110) and its [research specification](https://github.com/Luminous-Dynamics/symtropy/pull/3111).

## Current claim ceiling

Passing this crate's tests would establish properties of this small in-memory model only. It would not establish:
- production-grade authentication or identity recovery;
- privacy-preserving storage or a secure public DHT design;
- legal suitability of any currency or cooperative instrument;
- economic superiority over public provision, markets or cash transfers;
- empirical service outcomes or real-world policy effectiveness;
- security of the Holochain adapters or canonical Mycelix TEND integration.

Those need separate implementation, governance, security review and empirical evaluation.
