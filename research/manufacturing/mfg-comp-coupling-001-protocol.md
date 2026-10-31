# MFG-COMP-COUPLING-001 — Composed-twin coupling validity

**Version:** 0.1  
**Date:** 2026-10-31  
**Evidence tier:** T0 synthetic known-truth  
**Implementation:** `tools/manufacturing-assurance/composition_coupling.rs`

## 1. Research question

When individually credible subsystem twins are composed, can the integrated simulation violate physical balances or become numerically inaccurate because of interface mapping, asynchronous communication, stale values, or clock skew? What minimal evidence is necessary before a composed-twin result can be treated as valid?

## 2. Paradigm shift: from value pipes to physical exchange contracts

A conventional integration boundary often exposes a value, type, and sample time. That is insufficient for a conservation-bearing physical interaction.

The proposed interface contract treats an exchange as a first-class event with:
- identity of the exchange and both participating subsystems;
- physical quantity, dimensional unit, and sign convention;
- sender and receiver timestamps plus a clock-skew/freshness bound;
- the integrated transfer over the communication interval, not only an instantaneous point value;
- validity domain, calibration/provenance and quality flags;
- paired receipts proving that both subsystems applied the same physical transfer with opposite signs.

This is inspired by established bond-graph and energy-conservation-based co-simulation (ECCO) research. The engineering goal here is a deterministic assurance primitive that can be attached to composition, lifecycle and release claims. We are not claiming novelty for energy conservation or power bonds.

## 3. Known-truth analytical system

Two thermally coupled bodies exchange heat. Let heat capacities be (C_A,C_B), conductance (G), and temperatures (T_A,T_B):

[
C_A dot T_A = -G(T_A-T_B), qquad
C_B dot T_B = +G(T_A-T_B).
]

The total stored energy (E=C_A T_A+C_B T_B) is invariant. The weighted mean is:

[
ar T = rac{C_A T_A(0)+C_B T_B(0)}{C_A+C_B}.
]

The temperature difference decays with rate:

[
lambda=Gleft(rac{1}{C_A}+rac{1}{C_B}ight),
quad
Delta T(t)=Delta T(0)e^{-lambda t}.
]

Thus:

[
T_A(t)=ar T+rac{C_B}{C_A+C_B}Delta T(t),qquad
T_B(t)=ar T-rac{C_A}{C_A+C_B}Delta T(t).
]

Frozen parameters:
- (C_A=2) J/K; (C_B=5) J/K;
- (G=0.8) W/K;
- (T_A(0)=100) K; (T_B(0)=20) K;
- horizon 2 s.

Analytic checkpoints are frozen in `tools/manufacturing-assurance/fixtures/analytic-composition-coupling.tsv`.

## 4. Algorithms and negative controls

### Reference baseline: shared-flux Jacobi

At each communication point, compute one transfer:

[
Q_k=G(T_{A,k}-T_{B,k})h.
]

Apply ( -Q_k) to A and (+Q_k) to B. This preserves total energy to floating-point tolerance but still has explicit-Euler trajectory error. It is not expected to be exact.

### Asynchronous negative control: independent one-step-stale ports

Each subsystem computes its own transfer using the peer's previous published value. The two port transfers can differ. Their difference appears as a spurious energy source/sink in the composed simulation.

This simplified negative control isolates the failure mechanism. It is not a representation of any named industrial product.

### Paired receipt controls

A good pair is accepted only when IDs and participant identities match, units are joules, values are finite, timestamps are within the configured skew, and signed energy changes sum to zero within tolerance.

Fault injections: wrong exchange ID; wrong component/peer identity; wrong sign; unequal transfer magnitude; excessive clock skew; incompatible units; non-finite values; invalid energy tolerance.

## 5. Metrics — independent gates, never one opaque score

1. **Conservation:** maximum and final absolute energy-balance residual in joules and normalized by initial stored energy.
2. **Trajectory fidelity:** RMS and maximum absolute temperature error versus the analytic solution.
3. **Convergence:** error ratio and observed order after halving the communication step.
4. **Contract integrity:** counts for accepted valid receipts and rejected negative controls; later add sensitivity, specificity, precision, false-acceptance rate and abstention.
5. **Synchronization robustness:** error and conservation degradation versus clock skew, stale-message age, dropped/reordered messages, and step-size ratio.
6. **Performance:** wall-clock runtime, CPU/peak memory, communication messages, rejected/retried steps, and accuracy at a fixed runtime budget.
7. **Uncertainty:** prediction interval coverage, calibration, and preservation of shared/correlated nuisance variables through composition.

A run cannot pass because a composite score hides a conservation failure. Report physical invariant and trajectory gates separately.

## 6. Target levels

These are proposed engineering targets, not current measured results or claims of state of the art.

| Axis | T0 target | T2/T3 target before field claim |
|---|---|---|
| Analytic fixture | all checkpoint residuals within (10^{-8}) K | independent implementation reproduces results |
| Closed-system energy | max residual (le 10^{-9}) J for shared-flux baseline | scale-normalized invariant bound tied to metrology and solver error |
| Trajectory | monotone improvement under step refinement; observed order consistent with integrator | held-out measured trajectories with uncertainty coverage |
| Contract fault detection | reject all frozen negative controls, accept valid pair | false acceptance (le 10^{-4}) with confidence bound and representative fault mix |
| Timing | reject skew beyond configured bound | bounded end-to-end timestamp uncertainty under deployed clock architecture |
| Replay | deterministic report on same build/inputs | signed provenance, toolchain and run-environment receipts |

The false-acceptance target requires a large, independent, pre-registered negative-control panel. It must not be inferred from a handful of unit tests.

## 7. Fair comparison with standards and industry

### Standards baseline

FMI 3.0.2 defines Model Exchange, Co-Simulation and Scheduled Execution interfaces and adds mechanisms including clocks, early return and intermediate updates. The co-simulation algorithm is left to the importer. Therefore the comparison must distinguish:
- FMI packaging/API conformance;
- master algorithm behavior;
- physical conservation;
- numerical error/stability;
- timing/fault response;
- uncertainty and validity claims.

FMI support is necessary for interoperability in many workflows but is not, by itself, a physical-validity certificate. Add SSP/system-structure packaging and cross-tool exchange in a later phase; do not label this reference FMI-compliant merely because it can later import/export an FMU.

### Established numerical research

ECCO and power-bond residual methods are prior art for measuring and controlling energy errors induced by loose coupling. They are a mandatory comparison lane, not an opponent to dismiss. Compare shared-flux, fixed-step Jacobi, Gauss-Seidel, rollback/retry, extrapolation/intermediate-update, and energy-residual-aware algorithms under identical models and tolerances.

### Commercial tool lanes

Run version-pinned empirical comparisons rather than making broad product claims:
- Siemens Simcenter system simulation and its documented FMI/cosimulation integrations;
- Ansys Twin Builder FMU coupling, with explicit recording of the documented version, FMU version and sample-time behavior;
- Modelon Impact FMU export/import, with FMI version and solver/compiler configuration captured.

For each product, record exact release, platform, supported FMI version and interface types, solver, master algorithm (if exposed), communication-step policy, rollback/early-return behavior, output tolerances, runtime, conservation residual, trajectory error and failure behavior. A feature checklist from vendor documentation is not benchmark evidence.

### Comparison protocol

- Use the same FMU/model equations, initial conditions, horizon, host hardware and reference tolerances.
- Split calibration and test by model configuration, not individual adjacent time points.
- Publish raw traces, model/fixture hashes, command lines, tool versions and configuration.
- Include monolithic analytic/high-accuracy reference, shared-flux baseline, one-step-stale negative control, and ECCO/power-bond reference where available.
- Report median and tail runtime across repeated runs, not only the best run.
- Keep solver accuracy separate from physical conservation and interface fault rejection.

## 8. Roadmap to public and industrial evidence

1. **T0:** current analytic two-body reference and fault controls.
2. **T1:** independently authored second implementation, property-based test generation, adversarial clock/unit/sign/freshness mutations.
3. **T2:** real public measured heat/process data. Candidate data programmes include NIST AM-Bench for additive-manufacturing physics and NIST Smart Manufacturing Systems Test Bed interoperability streams. Data admission must include source URL, licence, units, calibration/uncertainty, sample timing, transformation history and immutable digest. No external dataset is considered admitted until bytes and provenance are actually captured.
4. **T3:** independent laboratory replication using traceable sensors and controlled perturbations.
5. **T4:** industrial pilot with read-only shadow deployment first; no actuation authority is implied.

NIST's 2026 digital-twin workshop report highlights interoperability, VVUQ, cybersecurity and workforce readiness as central challenges. This work directly targets the first two while preserving provenance and security boundaries.

## 9. Exit criteria and claim ceiling

T0 exit requires clean warning-denied compilation, all tests, deterministic report replay, independent arithmetic review, and exact-head CI evidence. A queued workflow is not a pass. Before T2, results remain synthetic. Before T3, do not claim physical validity. Before T4, do not claim industrial effectiveness. No “better than SOTA” claim is justified until an independently replicated, fair comparison supports it.

This reference never recalibrates a model, changes validity envelopes, authorizes production actions or actuates hardware.
