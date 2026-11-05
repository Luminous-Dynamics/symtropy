# Maritime service-chain contract v0.1

Dependency-free Rust types and validation for the first synthetic end-to-end
maritime service-chain case under Symtropy #3683.

This is a **scenario contract**, not another optimizer, vessel model, port
optimizer, hydrodynamics solver, lock controller, logistics event store, or
navigation authority.

## Run

~~~sh
cargo fmt --manifest-path benchmarks/maritime-service-chain/Cargo.toml -- --check
cargo test --locked --manifest-path benchmarks/maritime-service-chain/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/maritime-service-chain/Cargo.toml --all-targets -- -D warnings
~~~

## First composition case

The in-code frozen fixture represents a synthetic route:

~~~text
origin
  -> channel transit
  -> lock approach
  -> lockage
  -> lock exit
  -> port approach
  -> berth + terminal handling + shore energy
  -> railhead
  -> inland transfer
  -> destination
~~~

The case includes:
- one 10-tonne demand record, participant ID, delivery window, and opaque Mycelix commitment reference;
- two already-declared vessel/speed alternatives with different arrival intervals and propulsion-energy envelopes;
- source/model revisions and time-bounded evidence references owned by Symthaea, Symtropy, Mycelix, Sol Atlas, or an external source;
- exact binding from each arrival interval to the corresponding arrival evidence record;
- explicit channel, lockage, berth, terminal-handling, shore-power, and inland-transfer resource types, units, capacities and availability windows;
- a separate non-compensable hard-constraint vector for navigability, lock eligibility, environmental flow, water, energy, berth capacity and participant authority;
- a synthetic-only claim ceiling and no real route/operator data.

The fixture is illustrative. It does not assert that any real vessel, lock, berth,
rail connection, waterway, or terminal has these characteristics.

## Validation boundary

The crate verifies:
- case and identifier uniqueness;
- a continuous origin-to-destination route graph;
- each segment's required resource kinds;
- resource-kind/capacity-unit compatibility and non-overlapping half-open calendars;
- exact resource-calendar-to-evidence binding (every evidence snapshot must match
  the precise availability windows and capacities, not only the resource ID);
- demand origin/destination agreement with the frozen route;
- nonzero, unit-matched resource uses that cover the full declared handling and
  inland-transfer amount;
- vessel cargo capacity at least equal to declared demand and in the same service unit;
- current, non-future, in-validity evidence at the decision cutoff and within
  the case's explicit maximum evidence age;
- owner/role boundaries for engineering, operational, spatial and participant evidence;
- exact arrival-window evidence binding;
- explicit Mycelix ownership for participant authorization and cargo custody;
- rejection of unknown or blocked non-compensable constraints.

A failed validation is not repaired by relaxing a gate, imputing a capacity, or
assigning a default. Missing or stale evidence must be resolved by its owner.

## Ownership seams

- **Symthaea:** vessel engineering, vessel performance and arrival-window evidence.
- **Symtropy:** lock/operational/resource envelopes, deterministic evaluation and this contract.
- **Mycelix:** participant authorization, commitments, cargo custody and lifecycle transitions.
- **Sol Atlas:** spatial topology, navigability evidence lineage and read-only geography.
- **IAPH/DCSA adapters:** port-call event/timestamp semantics remain in #1930/#1955. This contract stores an opaque projection reference; it does not duplicate their schemas.

The validator checks references, declared ownership and applicability metadata. It
does **not** cryptographically authenticate snapshots, prove engineering
qualification, prove calibration or conditional coverage, establish data rights,
or verify that the upstream owner actually performed its promised checks.

## Deliberate limits

- Synthetic case class only. No real AIS, LPMS, port, bathymetry, water-level,
  environmental, cargo-custody or participant data is bundled.
- No voyage/lock/berth optimizer or independent schedule simulator is implemented.
- No physical hydraulic, sediment, manoeuvring, energy-generation or emissions model is implemented.
- Energy is expressed in separate declared Wh boundaries. Lifecycle emissions
  factors and well-to-tank/tank-to-wake/well-to-wake claims are intentionally absent.
- Resource calendars are validated structurally; the crate does not choose or
  reserve a time slot.
- A passing contract test is not physical qualification, operator approval,
  passage authority or permission to operate.

Next work belongs to the existing owners: compose this contract with the
lock eligibility/scheduler work (#3347/#3362), JIT/contingent planning
(#1858/#1867), channel/sediment/environmental constraints (#3279/#3302),
Mycelix lifecycle authority (#5185), and Sol Atlas evidence projection (#115).
