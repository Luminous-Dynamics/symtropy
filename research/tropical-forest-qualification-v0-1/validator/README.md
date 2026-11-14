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

# Observation-operator case manifest (validates expected dispositions, does not execute them)
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-observation-operator-negative-controls-v0.1.json

# Public model-set adequacy development manifest (not a blind test)
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-model-set-adequacy-development-v0.1.json

```

Exit status is 0 for a successful fixture-specific verification, 1 for a semantic `FAIL`, and 2 for malformed JSON, missing required fields, or invalid invocation. The receipt includes the exact input-byte SHA-256 and individual checks.

## Model-set adequacy

The public forest model-set corpus is a case-family contract only. It deliberately exposes no hidden generator equations and returns `MANIFEST_VALID`, not a model-adequacy pass. A real blind tournament requires independent custody of generator identity/outcomes and a precommitted candidate run.

## Fixture-specific result meanings

| Fixture | Successful result | What that result means |
|---|---|---|
| Biomass accounting | `PASS` | The synthetic live/deadwood transitions, declared transfers, total stocks and disconnected-control invariants are internally consistent |
| Canopy geometry | `PASS_SYNTHETIC_GEOMETRY` | The 8×8 analytical grid matches its declared LAI, four-neighbour gap topology and uncalibrated synthetic transmittance equations |
| Observation operators | `MANIFEST_VALID` | The nine expected dispositions, source-specific claim boundaries and non-execution metadata are well-formed; the negative controls themselves have **not** been run against a production observation adapter |
| Model-set adequacy | `MANIFEST_VALID` | The ten public development case families and expected dispositions are well-formed; no blind generator or independent custodian is provided |

## Dataset candidate register

See [DATASET_CANDIDATE_REGISTER_V0_1.json](../DATASET_CANDIDATE_REGISTER_V0_1.json) for five metadata-only source candidates: Amazon RAINFOR / EIDC (OGL), NASA / ORNL DAAC Brazil, DRC NFI, PNG NFI and Brazil NFI. It records spatial/temporal support, access state, permitted use and the next admission gate. No source data have been ingested by committing this register.

## Covered checks

- Fixture version, benchmark identity, synthetic claim ceiling, source scope and exact spatial IDs.
- Non-negative declared flows, parameter/effect consistency and explicit decomposition boundary.
- Live biomass, deadwood, total biomass, mortality transfer and biomass budget.
- Disconnected-control invariance using declared causal links, comparing canonical stocks separately from derived totals.
- Observation corpus case uniqueness, complete expected disposition mapping and explicit `NOT_IMPLEMENTED` / `NOT_RUN` execution fields.
- Canopy coordinate bounds, duplicate/overlap/coverage, total and mean LAI, connected gap components, synthetic transmission and uniform-baseline calculations.
- Deterministic receipt generation and adversarial mutation tests.

## Claim limits

These results are deliberately narrow:

- Biomass fixture PASS does not validate ecological growth or mortality equations.
- Canopy geometry PASS does not validate radiative transfer, crown allometry, tree physiology or recruitment.
- Observation `MANIFEST_VALID` does not mean an observation adapter rejected the nine negative controls at runtime.
- None establishes a real forest forecast, field calibration, regional transfer, SOTA superiority or authority to intervene.

The validator is a separate research workspace to keep dependency and compile scope small. It does not change production `symtropy-basin` APIs or canonical Living World ownership.
