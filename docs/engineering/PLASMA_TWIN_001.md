# PLASMA-TWIN-001 — Solver-neutral plasma scenario contract

**Status:** Design contract for review; not a solver implementation or validation claim.  
**Parent:** [ENG-SYNTH #1172](https://github.com/Luminous-Dynamics/symtropy/issues/1172)  
**Twin boundary:** [ENG-SYNTH-TWIN #1467](https://github.com/Luminous-Dynamics/symtropy/issues/1467)  
**Capability/credibility work:** [ENG-SYNTH-CRED #1173](https://github.com/Luminous-Dynamics/symtropy/issues/1173)  
**Symthaea benchmark:** [PLASMA-VV-001 #7588](https://github.com/Luminous-Dynamics/symthaea/issues/7588)

## 1. Decision

Treat plasma as an externally solved, regime-qualified engineering domain. Symtropy's role is initially scenario identity, execution orchestration, replay, deterministic artifact handling, and downstream synthetic-system coupling. A specialist solver owns its supported plasma equations and numerical result.

This contract does **not** claim that Symtropy currently solves plasma electromagnetic fields, plasma chemistry, thermal arcs, kinetic particle dynamics, or MHD. No plasma equations are added to the rigid-body contact solver as part of this contract.

## 2. The scientific invariant

Keep these claims distinct:

- A candidate design is a proposal, not a simulation result.
- A completed solver process is not proof of numerical convergence.
- Numerical convergence is not physical validation.
- Cross-code agreement is useful only when equations, boundary conditions, observables, and physical assumptions are comparable.
- A replay receipt proves only the properties it explicitly checks.
- Synthetic output is never a laboratory measurement or hardware authorization.

Each result must carry its evidence class and claim ceiling through serialization, replay, and visualization. A renderer may not promote claim strength.

## 3. Regime qualification

A run must name one explicit, versioned regime profile. Initial profiles may include:

- cold atmospheric plasma;
- low-pressure fluid RF discharge;
- kinetic / particle-in-cell;
- thermal arc or jet;
- magnetized fluid / MHD;
- fusion plasma;
- laser-plasma interaction;
- other, only with an explicitly qualified profile.

These profiles are not aliases. Passing a low-pressure RF fluid case does not qualify a thermal torch, an atmospheric plasma jet, a fusion operating point, or a kinetic case.

The profile declares the model class, supported species and reactions, boundary/initial-condition requirements, validated parameter envelope, known omissions, and required observables. Unknown profile, unsupported capability, or out-of-envelope inputs produce an explicit unsupported/incomplete disposition.

## 4. Versioned data contract

Implement as small, serializable Rust DTOs in an integration/domain layer after the initial schema review. Names below are conceptual.

### PlasmaScenarioSpec

Contains:
- schema version and stable scenario ID;
- geometry/configuration identity and source;
- declared design variables, SI units, bounds, provenance, and uncertainty where known;
- gas/species/reaction dataset identifiers and content hashes, with citation and license/redistribution metadata;
- boundary and initial conditions;
- named regime/model profile;
- required observables;
- constraints and explicit non-goals.

Never fill missing values with physically meaningful-looking defaults. Reject unknown units, dimension mismatches, non-finite values, contradictory constraints, and missing required fields.

### SolverCapabilityProfile

Contains:
- solver name, exact version/tag/commit, and source/license identity;
- supported regimes and model formulations;
- supported species/reaction capabilities;
- accepted input schema version;
- declared validity envelope and known model limitations;
- dependency/toolchain/environment identity;
- required solver diagnostics.

Solver capability must be declared from inspected upstream behavior or qualified evidence, not inferred from the executable name.

### SolverRunReceipt

Contains:
- immutable scenario and solver-profile identities;
- hashes of normalized request bytes, all referenced input data, configuration, and raw outputs;
- exact invocation metadata and environment identity;
- start/end timestamps and resource measurements when available;
- exit status, convergence status, warnings, and error details;
- required-output inventory and normalized-result hash;
- raw artifact references and a manifest schema version;
- evidence class and claim ceiling.

Timestamps and volatile telemetry should be separated from deterministic content identity so equivalent runs can be compared without incorrectly claiming byte-identical execution when an output includes time-varying metadata.

### Observable

Every normalized value identifies:
- quantity name and SI unit;
- scalar/vector/field shape and coordinate frame;
- sampling location and simulation time/phase where applicable;
- uncertainty or interval when provided by the source;
- derivation, source artifact, and normalization version;
- status: present, estimated, unavailable, invalid, or outside-profile.

Raw solver outputs remain immutable. Normalization never erases warnings, failed fields, or discrepancies.

### RunDisposition

Every attempted run receives exactly one explicit terminal disposition, e.g.:

- CompletedWithinDeclaredProfile
- CompletedButNotConverged
- UnsupportedRegimeOrCapability
- InvalidInput
- SolverFailed
- IncompleteRequiredOutput
- OutsideQualifiedEnvelope
- Cancelled

A zero process exit status alone can never produce CompletedWithinDeclaredProfile. Unknown fields and unknown enum values must not silently deserialize into a success path.

## 5. Run lifecycle

1. Validate schema, dimensions/units, data provenance, profile capabilities, bounds, and required fields.
2. Resolve all external references to immutable versioned identities before execution.
3. Construct an immutable request identity and retain the exact bytes.
4. Invoke the selected external solver only through an explicit opt-in runner with bounded resources and captured stdout/stderr.
5. Preserve exit status, warnings, convergence diagnostics, raw output, and all produced artifacts.
6. Normalize only declared observables; retain raw-to-normalized lineage.
7. Generate the run receipt.
8. Independently verify manifest completeness, hashes, required fields, units, disposition logic, and claim ceiling.
9. Project results into Symtropy as synthetic/research output with source identity and applicability labels.
10. Keep any future physical observation in a separate typed channel with its own instrument, calibration, operator, time, and raw-observation provenance.

The independent verifier must not import the runner's success decision as its oracle. It checks the artifacts and declared acceptance rules independently.

## 6. Reference case selection

Start with one source-pinned low-temperature RF capacitively coupled plasma (CCP) case supported by the exact chosen solver revision. Zapdos is a candidate because its published verification/validation work covers 1D/2D RF CCP models, manufactured-solution convergence, code-to-code comparisons, and selected experimental comparisons: https://doi.org/10.1016/j.cpc.2023.108837

Use the NIST GEC RF reference cell only where geometry, pressure, gas, excitation, diagnostic, and boundary conditions match the source data: https://www.nist.gov/publications/gaseous-electronics-conference-radio-frequency-reference-cell-defined-parallel-plate

LXCat data may support low-temperature plasma models, but each contributor dataset is versioned/provenanced independently; preserve the dataset's citation, conditions, redistribution terms, and exact content hash: https://us.lxcat.net/notes/index.php?download=lxcat_team1

WarpX is a possible later kinetic/PIC reference for cases that match its supported model and published test conditions: https://warpx.readthedocs.io/en/latest/usage/examples.html

The GEC/Zapdos benchmark qualifies the evidence and adapter path; it is not by itself proof that an atmospheric plasma jet or hot-plasma torch has been designed successfully.

## 7. Acceptance tests before a physics-facing adapter can qualify

Contract-level tests:
1. Missing required pressure, composition, excitation, geometry, or boundary condition cannot become a completed run.
2. Incompatible units, unknown units, NaN/Inf, invalid coordinate frame, and reversed bounds fail closed.
3. Changed input bytes or dataset identity produce a different request identity.
4. Same display name but different solver versions produce distinct capability/run identities.
5. Unsupported species/reaction/model option cannot produce a completed-within-profile status.
6. Exit code 0 with missing convergence evidence is not treated as success.
7. Missing required observable yields incomplete output.
8. Warnings and failed candidates remain present after normalization and replay.
9. Changed geometry prevents reuse of a receipt tied to the prior geometry.
10. Solver output cannot deserialize as a physical measurement or a hardware-authorizing state.
11. A visualization smoothing/transform operation has separate derived identity and cannot mutate the raw artifact.
12. A tampered manifest, stale hash, missing artifact, or false status is caught by the independent verifier.

Research/physics tests are separate:
- numerical verification (including mesh/time-step convergence where applicable);
- code-to-code benchmarking only under compatible assumptions;
- experimental validation with diagnostic uncertainty;
- model applicability and out-of-distribution checks.

A successful contract test is not a plasma-physics validation result.

## 8. Benchmark targets

- 100% of attempted runs retain a terminal disposition, including failures and non-convergence.
- 100% of accepted receipts pass independent identity, completeness, required-field, and disposition checks.
- 0 unsupported-profile or synthetic-to-measured promotions accepted by adversarial tests.
- Every claimed physical observable has a unit, source artifact, and explicit provenance.
- Reproducibility is reported at the level actually supported: exact deterministic content, numerically equivalent output within a predeclared tolerance, or nondeterministic/unqualified—not a blanket reproducibility claim.
- Do not set numerical tolerances after viewing a favorable result. Derive them from solver documentation, manufactured-solution expectations, reference measurement uncertainty, and the intended comparison.

## 9. Explicit non-goals and safety boundary

No direct high-voltage, RF, laser, gas-flow, pressure, thermal, or plasma power-source actuation. No lab procedures, hardware build authorization, safety certification, or autonomous physical operation. No custom plasma PDE solver in Symtropy core. No use of the consciousness/Phi coupling as a substitute for an established physical constitutive law. No promotion of simulation/replay to experimental evidence.

## 10. References

- DeChant et al., “Verification and validation of the open-source plasma fluid code: Zapdos,” Computer Physics Communications 291 (2023), 108837. https://doi.org/10.1016/j.cpc.2023.108837
- NIST, “The Gaseous Electronics Conference Radio-frequency Reference Cell.” https://www.nist.gov/publications/gaseous-electronics-conference-radio-frequency-reference-cell-defined-parallel-plate
- LXCat project context and dataset provenance. https://us.lxcat.net/notes/index.php?download=lxcat_team1
- WarpX tested example catalog. https://warpx.readthedocs.io/en/latest/usage/examples.html
