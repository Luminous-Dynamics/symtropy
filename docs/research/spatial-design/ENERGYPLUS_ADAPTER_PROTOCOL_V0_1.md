# EnergyPlus Adapter Qualification Protocol v0.1 — A0

## Purpose and claim boundary

This is the first qualification stage for [ENERGYPLUS-QUAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/1900). It pins a maintained external building-energy engine and defines what a run receipt must prove before the output can even be considered for a reference comparison. It is not a building simulation, thermal certification, accessibility review, code-compliance verdict or professional engineering sign-off.

The official [EnergyPlus v26.2.0 release](https://github.com/NatLabRockies/EnergyPlus/releases/tag/v26.2.0) was published on 30 September 2026. The A0 manifest freezes the release tag and source commit, a platform-specific Ubuntu 24.04 x86_64 installer, its published SHA-256 and exact byte size. The package has not been downloaded or executed by this change.

Official [EnergyPlus 26.2.0 documentation](https://energyplus.readthedocs.io/en/v26.2.0/essentials/essentials.html) documents both IDF and epJSON input formats and states that the numerical inputs use SI units. The [Input/Output Reference](https://energyplus.readthedocs.io/en/v26.2.0/guides/input-output-reference/index.html) documents the versioned input schema, output-variable and meter reporting, and output files. Every actual run receipt binds the exact schema file digest captured from the pinned installation rather than leaving a floating `latest` reference.

## Receipt gate

The run receipt includes engine version/tag/source commit/package digest/platform; model and authoritative geometry revisions/digests; weather, schedules and configuration identities/digests; schema revision and digest; unit system; timestep/run dates/warm-up setting; execution exit/convergence/severe/fatal/partial state; log digest; output variable/meter identities, units, frequencies, content digests and sample counts; and a claimed gate/disposition.

The validator independently recomputes that gate. It does not trust a caller-supplied `passed` flag. An eligible receipt requires all referenced files to exist under a caller-provided evidence root, with safe relative paths and exact SHA-256 matches. Symlinks resolving outside the evidence root, path traversal, oversized/missing files and digest mismatch are rejected. The public receipt validation command requires an evidence root: without inspecting artifact bytes, a receipt is not eligible.

### States

- **Eligible for reference comparison** only means the input/output/run envelope is internally consistent and hash-bound. The result still needs independent software-capability and physical-reference qualification.
- **Blocked** means execution failed, did not converge, had severe/fatal errors or emitted partial output.
- **Unknown** means the engine/schema/input provenance is mismatched or cannot be verified, or a required output/artifact is absent or outside the declared envelope.

No absent value is recoded as zero; partial and non-converged output is never admitted as a successful run.

## Frozen synthetic negative controls

The ten receipt fixtures include:
1. a structurally complete, hash-shaped receipt;
2. a package digest that differs from the pin;
3. an input-schema revision mismatch;
4. non-convergence;
5. a nonzero process exit;
6. partial output;
7. a missing temperature time series;
8. wrong energy-meter units;
9. a severe solver error;
10. a self-declared gate/blocker set that contradicts computed evidence.

These are synthetic receipt records, not EnergyPlus runs. They test only that the adapter gate responds deterministically to the specified input envelope and failure modes.

## Acceptance targets

| Gate | A0 acceptance target |
|---|---|
| Package identity | Exact tag, source commit, platform, asset digest and byte size are pinned; 100% rejection of altered digest/size |
| Input provenance | Every accepted receipt carries the exact engine/schema revision and hashes for model, geometry, weather, schedules and configuration |
| Artifact integrity | 100% of seeded digest mismatch, missing artifact, path traversal and symlink escape controls are rejected |
| Process state | 100% of nonzero exit, non-convergence, severe/fatal and partial-result controls are blocked |
| Required outputs | 100% detection of missing outputs and wrong units/report frequency |
| Claim integrity | 100% rejection of a self-reported gate/blocker set that differs from recomputed evidence |
| Determinism | Canonical blocker codes and gate state are invariant to irrelevant field ordering |
| Physical qualification | **No claim in A0.** Applicable Standard 140 tests, analytical/reference comparisons and held-out calibration come later |

ASHRAE/IBPSA Standard 140-2023 is a capability-test layer, not universal project validation. Select the applicable test subset only after checking rights/licence requirements; preserve an index to the exact tests and expected tolerances, not unlicensed content copied wholesale into the repository. Subsequent A1/A2/A3 stages remain separate from A0 receipt validity.

## Staged next work

- **A1 — Capability/input controls:** rights-checked Standard 140 subset; versioned weather interpretation tests; seeded missing-unit/input/schema/unsupported-feature controls.
- **A2 — Trajectory integration:** one small fixed single-zone reference package with frozen epJSON, weather and schedules; time-series output; separate trajectory oracle for threshold-crossing and recovery timing.
- **A3 — Decision-use gate:** compare against suitable independent reference/measured trajectories. Freeze calibration and held-out sets separately, predeclare application-specific tolerances, report bias/residuals, interval coverage when intervals are used and out-of-domain abstention.

No blanket temperature or energy error threshold is chosen until the dataset, intended decision, reference method and consequence are specified.
