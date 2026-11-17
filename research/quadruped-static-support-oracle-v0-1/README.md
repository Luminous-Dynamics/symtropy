# Quadruped Static Support Oracle v0.1

Parent: Symtropy #5744  
Task application: Symtropy #5739  
Backend comparison: Symtropy #5727  
Status: standalone analytic reference implementation; runner qualification pending.

## Purpose

Compute a transparent 2-D static support polygon from declared active foot contacts and evaluate the projected centre of mass against it, with conservative uncertainty deductions.

This is a bounded geometric reference. It is not a gait controller, dynamic stability model, friction-cone solver, foot-force allocator, viability/recovery solver or motion-authority mechanism.

## Contract

All contact points and the CoM projection are supplied in the same explicitly versioned support-plane frame. Every contact carries:
- a unique foot/contact ID;
- frame and plane identity;
- frame revision;
- position in metres;
- nonnegative radial position uncertainty.

The snapshot also binds robot identity, morphology revision, CoM projection uncertainty and a caller-supplied minimum margin.

A missing, stale, mismatched, duplicate, non-finite or otherwise invalid field returns `InvalidEvidence`. Fewer than three contacts returns `InsufficientContacts`. Collinear or zero-area support returns `DegenerateSupport`.

## Reference calculation

1. Build a deterministic 2-D convex hull by monotone chain.
2. Compute polygon area and signed distance from projected CoM to its boundary.
3. Subtract CoM projection uncertainty and the largest declared contact-position uncertainty from the nominal signed margin.
4. Compare the conservative result with the profile's required margin:
   - `Stable`: conservative margin >= required margin;
   - `Marginal`: conservative margin >= 0 but below required margin;
   - `Outside`: conservative margin < 0.

The uncertainty deduction is a simple conservative development rule. It is not a general set-based support-polygon calculation.

## Run

From this directory:

    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo test
    cargo run

No third-party Rust dependencies are required.

## Tests

Known-answer and adversarial cases cover centered and near-edge CoM, uncertainty deductions, outside projection, triangular support, insufficient and degenerate contacts, duplicate IDs, stale/mismatched frame identity, non-finite/negative evidence and input-order invariance.

These public synthetic fixtures verify this implementation's declared geometry. They do not validate the current quadruped demo or a physical robot.

## Claim boundary

A Stable result only means that a point projection lies within this declared convex support approximation with the supplied simple uncertainty deduction and margin threshold. It does not establish:
- dynamic balance, capture-point or centroidal stability;
- friction feasibility or tangential force closure;
- foot force distribution or actuator torque feasibility;
- impact or gait stability, recovery, terrain traversability or safety;
- actual contact, CoM or frame truth on a robot.

The evaluator emits no action commands and grants no actuation permission.
