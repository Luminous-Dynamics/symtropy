# Tropical forest fixture qualifier

This is a small, offline Rust verifier for the synthetic qualification fixtures in the adjacent `cases/` directory. It dispatches by the manifest identity and emits a deterministic SHA-256-bound JSON receipt.

## Run

From the repository root:

```bash
cargo test --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml

# Synthetic biomass / canonical-stock accounting fixture
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-patch-accounting-synthetic-001.json

# Analytical canopy topology / synthetic light geometry fixture
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/canopy-gap-representation-synthetic-v0.1.json

# Observation-operator synthetic rule corpus (does not run a production adapter)
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-observation-operator-negative-controls-v0.1.json

# Public model-set adequacy development manifest (not a blind test)
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-model-set-adequacy-development-v0.1.json

# Public forest decision-value development corpus (uses synthetic arithmetic only)
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-decision-value-development-v0.1.json

```

Exit status is 0 for a successful fixture-specific verification (`PASS`, `PASS_SYNTHETIC_RULES`, `PASS_SYNTHETIC_GEOMETRY` or `MANIFEST_VALID`), 1 for a semantic `FAIL`, and 2 for malformed JSON, missing required fields, or invalid invocation. The receipt includes the exact input-byte SHA-256 and individual checks.

## CI integration

The standalone package has its own Cargo workspace boundary, so root `cargo fmt --all`, `cargo clippy --workspace` and root workspace checks do not include it. The repository CI workflow now has explicit steps for this manifest: `cargo fmt --manifest-path ... -- --check`, `cargo clippy --manifest-path ... --all-targets --all-features -- -D warnings`, and `cargo test --manifest-path ... --all-targets --all-features`. A queued workflow is not evidence that these steps passed.

## Headless canopy kernel

The exported `canopy_kernel` module provides a minimal spatial cohort-to-cell LAI representation, explicit one-way cohort mortality, derived Beer–Lambert transmittance, four-neighbour gap components and an optical understory eligibility mask. It is a synthetic analytical kernel only; it does not implement crown allometry, multiple scattering, species physiology, recruitment, succession or a calibrated radiation model. Its eight unit tests are included in the library test suite.

## Decision-value corpus

The public forest decision-value corpus defines twenty case families and checks two worked synthetic examples. The arithmetic oracle verifies that high information can have zero decision value when the loss matrix is state-invariant, and independently recomputes the positive-EVSI example (gross 8.8 synthetic loss units; net 5.8 after cost). This is not execution of the existing EVSI policy kernel or a field-survey recommendation.

## Model-set adequacy

The public forest model-set corpus is a case-family contract only. It deliberately exposes no hidden generator equations and returns `MANIFEST_VALID`, not a model-adequacy pass. A real blind tournament requires independent custody of generator identity/outcomes and a precommitted candidate run.

## Fixture-specific result meanings

| Fixture | Successful result | What that result means |
|---|---|---|
| Biomass accounting | `PASS` | The synthetic live/deadwood transitions, declared transfers, total stocks and disconnected-control invariants are internally consistent |
| Canopy geometry | `PASS_SYNTHETIC_GEOMETRY` | The 8×8 analytical grid matches its declared LAI, four-neighbour gap topology and uncalibrated synthetic transmittance equations; ten public challenge-case IDs and expected dispositions are valid. The challenge processes are not executed |
| Observation operators | `PASS_SYNTHETIC_RULES` | The nine synthetic input descriptors are run through the Rust rule engine and produce their declared missingness/support/semantics dispositions; no external data or production adapter is used |
| Model-set adequacy | `MANIFEST_VALID` | The ten public development case families and expected dispositions are well-formed; no blind generator or independent custodian is provided |
| Decision value | `MANIFEST_VALID` | Twenty public development cases, the zero-EVSI counterexample, and the positive-EVSI worked arithmetic are coherent; the existing EVSI policy kernel is not executed by this validator |

## Dataset candidate register

See [DATASET_CANDIDATE_REGISTER_V0_1.json](../DATASET_CANDIDATE_REGISTER_V0_1.json) for five metadata-only source candidates: Amazon RAINFOR / EIDC (OGL), NASA / ORNL DAAC Brazil, DRC NFI, PNG NFI and Brazil NFI. It records spatial/temporal support, access state, permitted use and the next admission gate. No source data have been ingested by committing this register.

## Covered checks

- Fixture version, benchmark identity, synthetic claim ceiling, source scope and exact spatial IDs.
- Non-negative declared flows, parameter/effect consistency and explicit decomposition boundary.
- Live biomass, deadwood, total biomass, mortality transfer and biomass budget.
- Disconnected-control invariance using declared causal links, comparing canonical stocks separately from derived totals.
- Observation corpus rule execution for support mismatch, no acquisition, label-semantic mismatch, geographic out-of-domain, cloud/no-data, plot leakage, inaccessible DRC units, PNG security/logistics omissions and single-census longitudinal overclaim; production adapter and rights execution remain explicitly not implemented.
- Decision-value corpus coverage, public-not-blind boundary, zero-EVSI loss-invariance case and positive-EVSI arithmetic from the declared binary likelihood/loss model.
- Canopy coordinate bounds, duplicate/overlap/coverage, total and mean LAI, connected gap components, synthetic transmission, uniform-baseline calculations, and ten unique named challenge-family dispositions.
- Deterministic receipt generation and adversarial mutation tests.

## Claim limits

These results are deliberately narrow:

- Biomass fixture PASS does not validate ecological growth or mortality equations.
- Headless canopy-kernel tests validate the declared synthetic cohort-support operations only; they do not validate biological crown geometry, radiative transfer or recruitment.
- Canopy geometry PASS does not validate radiative transfer, crown allometry, tree physiology or recruitment; the ten challenge-case dispositions are contract checks, not executed canopy processes.
- Observation `PASS_SYNTHETIC_RULES` means only that the nine machine-readable synthetic input descriptors triggered the intended Rust rule dispositions; it does not mean a production observation adapter rejected real-world invalid claims.
- Decision-value manifest validity does not measure an actual policy's EVSI performance or authorize a field visit.
- None establishes a real forest forecast, field calibration, regional transfer, SOTA superiority or authority to intervene.

The validator currently contains 26 Rust unit tests across the accounting verifier, synthetic observation rules, canopy manifest checks, model-set manifest checks and headless cohort-canopy kernel. They are authored but must still pass exact-head Cargo execution. The validator is a separate research workspace to keep dependency and compile scope small. It does not change production `symtropy-basin` APIs or canonical Living World ownership.
