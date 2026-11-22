# Integrated navigation assurance kernel

**Status:** four standalone Rust research modules combined on one integration branch; combined exact-head CI pending. No physical MILUV payload is admitted, and none of the modules claims device or navigation safety certification.

**Base:** verified DS-TWR adapter work from [PR #6815](https://github.com/Luminous-Dynamics/symtropy/pull/6815).

## 1. Why integrate the modules

Separate green branches show that each module can pass in isolation. They do not prove that the shared crate compiles and tests when clock reconstruction, cryptographic session evidence, posterior auditing and active acquisition are present together.

This branch combines the four increments without merging or changing the original draft PRs.

## 2. Assurance flow

```text
Original UWB events
    |
    +--> Per-radio clock stream reconstruction
    |       - raw uint32 counter
    |       - host acquisition-time bounds
    |       - oscillator-rate tolerance
    |       - boot/reset epoch and event semantics
    |
    +--> External cryptographic verifier receipt
    |       - peer/session/sub-session binding
    |       - STS mode, key epoch, nonce, freshness
    |       - session and radio boot epochs
    |
    +--> DS-TWR / measurement integrity
    |       - event-to-duration mapping
    |       - clock/timestamp interval
    |       - antenna and power calibration
    |       - positive NLOS bias only when justified
    |
    +--> Front-end estimator
    |       - EKF / factor graph / VIO / cooperative UWB
    |       - exact observation IDs and dependency groups
    |
    +--> Independent posterior audit
    |       - measured range-set compatibility
    |       - anchor and candidate position bounds
    |       - 3D directional geometry by dependency group
    |
    +--> Protection level / mission contract
    |
    +--> Active evidence acquisition
            - synchronize / CIR / new anchor / observer / viewpoint
            - conditional PL reduction vs latency and energy
            - no authority grant
            |
            +--> new observation receipt -> requalify -> re-audit
```

## 3. Module boundaries

### Clock stream reconstruction

`miluv_clock_stream` enumerates every uint32 wrap count consistent with a host-time interval and oscillator-rate interval, and accepts only a unique result. It requires a separate stable event semantic, host clock domain and externally established boot epoch.

Still unqualified for real MILUV:
- hard event-to-host timestamp latency bounds;
- oscillator rate tolerance over operating temperature and RF conditions;
- proof of radio boot/reset epoch;
- joining one event stream's absolute wrap history to every DS-TWR transaction.

### Secure ranging session evidence

`secure_session_evidence` checks an external cryptographic verifier receipt for peer identity, radio boot epochs, protocol profile, STS mode, session validity, key epoch, sequence and nonce, then enforces replay and monotonicity rules.

It does not implement STS, AES, key derivation or packet parsing. Its replay state is in-memory; production needs durable monotonic state or hardware epoch. Cryptographic acceptance does not establish LOS, correct calibration or true distance.

### Independent posterior audit

`independent_posterior_audit` checks whether the front-end candidate and its declared absolute position bound are geometrically compatible with qualified range sets and anchor position bounds. It uses dependency-group-balanced direction matrices to prevent duplicate rows in a declared common-cause group from inflating directional support.

It is not a state estimator, covariance, protection level or proof of truth. Input bounds and dependency labels remain external assumptions.

### Active evidence acquisition

`active_evidence_acquisition` computes the Pareto frontier among conditional protection-level reductions, worst-case latency and energy. It uses a declared lexicographic rule rather than arbitrary scalar weights.

Its proposed post-action bound is conditional on successful acquisition and qualification. It always declares that the plan cannot authorize a navigation authority transition. New evidence, posterior re-audit and mission-contract evaluation are required.

## 4. Controller/evaluator and source completeness

The integration branch does not close [issue #6813](https://github.com/Luminous-Dynamics/symtropy/issues/6813). The publisher's processed MILUV preprocessing chain filters by Vicon-derived range bias before export. Therefore, processed CSV row presence is truth-conditioned. A source-complete original observation stream and row-identity negative control remain admission blockers.

Official source DOI: [MILUV Figshare dataset](https://doi.org/10.25452/figshare.plus.28386041.v1). The publisher reports a 173 GB source archive; select and freeze one source-complete experiment after inspecting the archive inventory rather than downloading everything uncritically.

## 5. Integrated validation plan

- Exact-head format and Clippy.
- All unit tests from the existing navigation kernels plus all four new modules.
- Existing protection-level, mission-risk, temporal-alignment and DS-TWR demos.
- Existing schema, generalization and rights-gated admission validators.
- Negative controls for truth-conditioned row selection and verifier/session receipt mismatch.
- Separate physical payload admission and hardware conformance after source completeness and rights review.

## 6. Claim boundary

This integration branch is software architecture and deterministic research kernels. It does not yet parse or score original MILUV payloads, establish hard ROS timestamp-latency bounds, calibrate oscillator/NLOS uncertainty, implement cryptographic UWB, or prove navigation integrity on hardware. Combined CI, even if passed, establishes software properties only.
