# World experiment fidelity lease — V0.1

An isolated Rust prototype that binds already-admitted physics and render-profile decisions to one immutable simulation observation, then checks bounded read-only view consumption.

This package does not select a physics solver, select a renderer, run a simulation, render an image, collect telemetry, authenticate upstream receipts, or authorize physical actuation. The physics selector remains with PR #4492 / issue #4487; the render selector remains with PR #4533 / issue #4486; observation and capture ownership remains with #4468, #1999 and #2008.

## Run

From this directory:

    cargo test --locked
    cargo clippy --locked --all-targets -- -D warnings
    cargo run --locked -- fixtures/machine-test-01.json

The isolated package carries its own Cargo.lock, reusing the currently resolved transitive package versions from the repository root lockfile. The exact-head workflow checks the package with Cargo's `--locked` mode.

The CLI emits a deterministic lease receipt. The fixture is synthetic software test data; its digest-shaped strings are not claims about real source artifacts, measurements, models, or hardware.

## Contract

A lease binds:
- experiment/run/scenario and exact source revision;
- initial-state, simulation-state and event-order digests;
- one simulation tick and one immutable observation-frame digest;
- an upstream physics policy input/receipt reference and selected model passport;
- one or more render policy/measurement references, each consuming the same observation digest;
- a fixed snapshot scope and explicit view/frame/memory/VRAM/optional energy budgets.

View declarations are treated as a set and canonicalized by view ID. Reordering them does not change the lease identity. Duplicate IDs, malformed digests, mismatched snapshots, and budgets wider than any declared profile envelope are rejected. Usage snapshots are also treated as a set of per-view records: `usage_snapshot_sha256` sorts them by `view_id`, hashes the complete cumulative snapshot with a separate domain, and returns that digest in both Continue and Stop decisions. Sequence 1 has no parent; each later snapshot must name the exact digest of the supplied previous snapshot.

The runtime guard checks:
- exact lease/run/snapshot/tick identity;
- only declared view IDs;
- monotonic sequence and cumulative usage;
- canonical, domain-separated usage digests that link each snapshot to its immediate predecessor;
- view-event, frame, system-memory, VRAM and frame-time limits;
- per-view runtime frame/VRAM measurements bound to their own receipts;
- lease-wide integrated GPU-board energy bound to a separate receipt (never summed from potentially overlapping view windows);
- missing energy when an energy cap is required;
- any attempted physical mutation under the read-only lease.

The lease is deliberately tied to one simulation tick. Advancing physics, changing inputs, changing the selected model, or introducing a physical intervention requires a new run/snapshot identity and a new lease. A new render profile requires a new render-policy receipt and a new lease. There is no implicit escalation or widening of an existing lease.

## Runtime enforcement boundary

V0.1 is a pure policy evaluator over caller-supplied cumulative usage records. It does not yet wrap a Bevy world, interdict ECS writes, reserve GPU/CPU resources, collect hardware telemetry, or prove that a caller cannot omit or falsify a reported mutation/measurement. The `physical_mutation_attempted` flag is an input claim that the guard evaluates, not an engine hook. A production adapter must connect authoritative write barriers and measurement receipts before this lease can be described as runtime-enforced. The claim ceiling remains finite-record policy conformance only.

## Identity and upstream boundary

Lease identity is SHA-256 over a domain-separated, deterministic JSON serialization of the typed input after sorting view declarations by ID. Usage identity uses a separate domain and canonical view ordering. The returned digest provides adjacent-record continuity and helps detect accidental splicing or modification relative to a retained parent digest; it is not a signature, authenticated append-only log, or tamper-proof storage. A trusted external store or signing authority must anchor retained receipts if adversarial rewrite resistance is required.

Upstream policy, model-passport and measurement digests are references. V0.1 validates digest syntax and binds those references, but does not fetch, authenticate, or re-evaluate their contents. A successful lease check cannot upgrade an upstream claim. Direct Rust dependencies are exact-version pinned; a generated Cargo.lock and reviewed transitive dependency snapshot remain part of exact-head qualification.

## Claim ceiling

Passing the finite tests establishes only that this package enforces its supplied lease schema, snapshot scope, identity and resource-budget semantics. It does not establish physical truth, model credibility, solver accuracy, uncertainty calibration, visual quality, hardware attestation, certification, requirement satisfaction, or actuation authority.

## Benchmark plan

Measure separately on exact source heads:
- lease compilation and verification latency (p50/p95);
- lease and usage digest throughput and receipt size;
- runtime guard overhead per view event;
- peak memory per active lease;
- maximum accepted views and event volume;
- hostile-corpus false acceptance (target 0);
- known-valid false rejection;
- view-order permutation identity (target 100%);
- presentation-only state/event invariance (target 0 violations).

These are contract-runtime metrics, not renderer or physics performance. Do not aggregate them with the benchmark scorecards in #4486 or #460.
