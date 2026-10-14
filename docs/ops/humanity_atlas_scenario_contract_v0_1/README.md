# Humanity Atlas scenario contract v0.1

**Status:** draft contract plus a fail-closed adapter prototype and synthetic regression fixtures. The adapter consumes captured simulator output but has not yet been qualified against a real run in this execution environment. It does not imply empirical calibration or claim that a calibrated civilization simulator exists.

## Why Symtropy belongs in this system

Symtropy can be the deterministic, replayable simulation substrate for questions with explicit state-transition models: energy balance, physical limits, storage and production throughput, resource constraints, transport, and infrastructure evolution. The existing symtropy-world bridge already separates simulation snapshots from presentation, and its world-scale abstraction gives us a natural place to define simulation boundaries. For the first integration, keep that runtime architecture intact: add an adapter that accepts a versioned scenario input and emits a versioned run envelope. Do not make Sol-Atlas depend directly on private simulator internals.

This contract separates roles:

- **Symtropy:** execute supported, explicit dynamics; preserve seeded reproducibility where promised; report model and run provenance.
- **Symthaea:** propose candidate causal structures, search scenario space, and formulate hypotheses. Suggestions remain proposals until tested against identified models and evidence.
- **Mycelix:** preserve source attribution, claim history, contestation, review and governance receipts. Consensus is not a substitute for empirical validation.
- **Sol-Atlas:** render graph, location, time, branch comparisons and scenario outputs while preserving epistemic classification.

## Contract files

- **schemas/scenario-v0.1.schema.json** — input scenario envelope.
- **schemas/run-v0.1.schema.json** — output envelope.
- **fixtures/earlier_storage_rollout.scenario.json** — a design-study fixture.
- **fixtures/synthetic_grid_baseline.json** and **fixtures/synthetic_grid_model.json** — explicitly synthetic, digest-bound inputs.
- **fixtures/synthetic_smoke_run.v0.1.json** — expected deterministic output.
- **validate_contract.py** — standard-library structural and semantic smoke test; optionally performs full JSON Schema Draft 2020-12 validation when Python's jsonschema package is installed.

## Run the process gate locally

From this directory, run **python3 validate_contract.py**. To exercise fail-closed behavior, run **python3 test_contract_fail_closed.py**. To print the generated result envelope, run **python3 validate_contract.py --emit-run**.

The smoke test verifies JSON parseability, top-level schema envelope shape, exact-byte SHA-256 bindings, valid horizon and intervention ranges, unique IDs, finite outputs, and deterministic replay by running the fixture twice and comparing results. This gate can run independently of GitHub Actions.

The smoke run is purposefully **not** the Symtropy engine. It uses a tiny transparent toy equation solely to prove that an input can be bound to a model/data digest and produce a replayable output envelope. The result is marked **synthetic_fixture**; nothing in it should be cited as real energy-system behavior.

## Trust and validation boundaries

Keep three state dimensions distinct:

1. **Data provenance:** what dataset and exact bytes were used?
2. **Model validity:** what equations, assumptions, units and ranges were used, and against what observations were they tested?
3. **Outcome uncertainty:** how variable are outputs across stochastic draws or model alternatives?

A deterministic run proves repeatability for specified inputs, not that the model represents reality. One run cannot establish uncertainty bounds. A scenario output is not a calibrated probability. **prospective_forecast** should only be emitted by a separately governed forecast workflow with a resolved outcome definition, deadline, calibration history, scoring rule and archived prediction receipt. Historical counterfactuals need sensitivity analysis across alternative causal models; their unseen outcomes are not directly verifiable.

### Evidence-gated classification

The detailed intended-use, verification, validation, uncertainty, retrospective-evaluation and forecast-receipt process is documented in **MODEL_CREDIBILITY_GATE_V0_1.md**. This is a proposed internal gate, not a certification.

A status string alone must not promote a run to `calibrated_simulation` or `prospective_forecast`. The run schema and local validator require:

- `calibrated_simulation`: `retrospective` or `external_review` status and a hashed `retrospective_evaluation` artifact. An independent review may supplement but does not replace retrospective evaluation.
- `prospective_forecast`: `prospective` or `external_review` status and a hashed `prospective_forecast` receipt.

Every validation evidence item has an ID, kind, artifact reference, SHA-256 digest, and description. These fields provide content-addressed traceability, **not authentication or scientific proof**. The v0.1 contract does not yet define signatures; Mycelix-backed publisher identity and independent review receipts remain a follow-up.

The failure-injection test copies the fixture set into temporary directories and verifies that changed baseline/model bytes, an out-of-horizon intervention, an unsupported parameter, an invalid time step, and evidence-free classification promotion all fail closed. Well-formed synthetic receipt shapes are accepted only to test the validator—not to qualify any real model.

Do not collapse source reliability, causal-edge confidence, forecast probability, normative preference and model completeness into one score. Do not infer that a technology is feasible solely because an unconstrained simulation reaches it: supported models must account for conservation laws, units, resource and energy budgets, manufacturing throughput, deployment lead times and physical constraints relevant to the claim.

## Planned integration sequence

1. **Contract seam (this patch):** versioned schemas, digest-bound fixtures, deterministic local smoke gate.
2. **Symtropy adapter:** translate a supported scenario into engine-native initial state/parameters and return the same run envelope. Preserve deterministic-seed and snapshot/replay semantics. Unsupported variables fail closed rather than being silently ignored.
3. **Sol-Atlas consumer:** render observed facts, estimates, scenario branches and calibrated forecasts differently; expose input/model digests in the detail panel.
4. **Symthaea analysis adapter:** propose scenario variations and causal hypotheses; serialize every intervention and model assumption for replay.
5. **Mycelix evidence workflow:** record source, attribution, dispute, and review receipts; preserve superseded claims rather than erasing their history.
6. **Qualification:** run unit checks locally, then historical hindcasts with time-cutoff data, sensitivity and stress tests, calibration checks, and independent model review for consequential claims.


## Private simulator access in GitHub Actions

The real-artifact workflow must check out `Luminous-Dynamics/luminous-dynamics`, which is a private repository. The `GITHUB_TOKEN` issued to this Symtropy workflow does not automatically grant cross-repository access.

For reproducibility, the qualification currently pins these source revisions rather than resolving mutable branches at runtime:

- Producer (`Luminous-Dynamics/luminous-dynamics`): `b59b99621cdb3699eda32aa39f152bd3b693a534` ([commit](https://github.com/Luminous-Dynamics/luminous-dynamics/commit/b59b99621cdb3699eda32aa39f152bd3b693a534)).
- Consumer (`Luminous-Dynamics/sol-atlas-leptos`): `d29742028e6d8b5c9b54bac7ae53721ecee3ac13` ([commit](https://github.com/Luminous-Dynamics/sol-atlas-leptos/commit/d29742028e6d8b5c9b54bac7ae53721ecee3ac13)).
- Adapter: the exact Symtropy PR head that triggered the run.

The workflow asserts all three checked-out revisions before compilation, and the qualification packet records the repository heads. When either upstream branch advances, update its pin deliberately in the workflow and README together, then re-check the producer/consumer schema byte identity and the full qualification path. A green run against older pinned commits proves only that exact combination, not the latest upstream branch heads.

Before the real-artifact qualification can run in GitHub Actions, configure the repository or organization Actions secret `LUMINOUS_MONOREPO_READ_TOKEN` as a narrowly scoped read-only credential (fine-grained personal access token or equivalent GitHub App token) with **Contents: read** access to `Luminous-Dynamics/luminous-dynamics`. Do not paste the token into a workflow file, issue, or pull-request comment. The workflow passes the secret only to the producer checkout and fails closed when it is missing or cannot read the private repository.

When access is unavailable, the workflow records `qualification_state: not_passed` and a blocker reason. That outcome is **blocked/not qualified**, not a skipped-as-success or a pass. A successful checkout alone is also not qualification: the real simulator capture, schema validation, independent Sol-Atlas consumer, exact artifact digests and final evidence packet must all complete.

## v0.1 limitations

- Numeric scalar interventions only (set, add, multiply); categorical and graph mutations require a future schema version.
- Annual integer-year horizon and scalar output series; subannual and spatial field outputs require a future extension.
- No cryptographic signature format is specified yet. SHA-256 identifies exact bytes but does not establish who published them or whether the content is true.
- No probability field is defined in v0.1. Add event-forecast receipts only after the scoring, resolution and calibration contract is specified.
- Schema changes require a new version and fixture migration; do not silently reinterpret old payloads.


## Real simulator artifact adapter

The companion `adapt_multiworld_output.py` maps a captured
`humanity-atlas-json` artifact from `mycelix-multiworld-sim` into the generic
run v0.1 envelope. It is an adapter, not a second simulator and not a scientific
validator. It requires the producer's `technology_tree`, `run_manifest`,
`resolved_config`, and `resolved_params` fields and rejects inconsistent
source/model/input/seed/tick bindings, malformed digests, duplicate JSON keys,
non-standard numeric tokens, non-finite numbers, and source artifacts larger
than 64 MiB. The independent Sol-Atlas importer enforces the same size ceiling
before hashing and parsing.

From the canonical monorepo, first capture a real run:

```sh
cargo run --manifest-path mycelix-multiworld-sim/Cargo.toml --release --bin luminous_sim -- --years 150 --seed 42 --format humanity-atlas-json > humanity-atlas-run.json
```


The CLI's progress/status messages go to stderr, so the redirected artifact remains
JSON-only. Preserve the file unchanged after capture: the envelope's source digest
is computed over these exact bytes.

Then run the adapter from the Symtropy repository:

```sh
python3 docs/ops/humanity_atlas_scenario_contract_v0_1/adapt_multiworld_output.py /path/to/humanity-atlas-run.json --output humanity-atlas-envelope.v0.1.json
```

The default artifact reference is `urn:sha256:<digest>`, which identifies the
exact source bytes but is not itself a hosting location. When storing the source
artifact in a content-addressed store or evidence service, pass its resolvable
URI with `--artifact-ref`.

For evidence preservation, `--output` refuses to replace an existing path unless
`--overwrite` is explicit. Output is staged in the destination directory and
installed atomically; destination symlinks are replaced as directory entries
rather than followed. The source simulator artifact and any symlink or hard-link
alias of it are never valid output destinations, even with `--overwrite`.

The adapter also requires `resolved_config.seed` and
`resolved_config.total_ticks` to agree with the top-level run before it emits
an envelope. The adapter deliberately emits `unvalidated_simulation`, a null
determinism value (not independently demonstrated), and a null dataset digest
(no separate dataset identity was established by this source manifest). It
scopes `model_sha256` to the effective technology-tree definition, records
resolved config/parameter digests, and binds the exact simulator-output bytes
in `artifacts`. SHA-256 is not publisher authentication. The source revision
is preserved if present; it is never inferred from a branch name. Calibration
and forecast classifications remain forbidden without their separate evidence
receipts.

Run the independent adapter regression tests with:

```sh
cd docs/ops/humanity_atlas_scenario_contract_v0_1
python3 -m unittest -v test_adapt_multiworld_output.py
```

These tests use an explicitly synthetic source-shaped fixture to exercise
adapter invariants; they are not evidence of a real simulator execution. The
end-to-end qualification step remains exporting an actual run, validating the
artifact through the independently implemented Sol-Atlas technology-tree
importer, and preserving the exact output bytes and resulting digests.

The source artifact reference also carries JSON Pointer `/technology_tree`.
The digest binds the complete simulator-output artifact bytes; the pointer selects
its embedded tree and is not represented as a separate digest. Sol-Atlas validates
the pointer syntax and independently validates the imported graph. A URI or
content-addressed name does not prove that the bytes have been published, signed,
or scientifically qualified.

The adapter marks each metric row with `time_basis: "scenario_relative_year"`.
The numeric `year` is therefore a horizon coordinate (e.g. 150 years after
scenario start), not a calendar date. Older v0.1 rows without `time_basis`
remain parseable as unspecified; consumers must not silently infer a calendar
year from them. The adapter rejects horizons greater than 100,000 years because
that exceeds the run schema's metric-year bound.

The producer's `survived` boolean is retained as the terminal
`civilization_survived` output indicator (0 = false, 1 = true). A false value
also emits `scenario_survival_failure` in the warnings. This is separate from
`run_status`: a completed run means the computation finished, not that the
simulated civilization survived. The run schema now states this distinction
explicitly. Consumers must not use execution success as a proxy for positive
scenario outcomes.

The generic envelope's `engine_version` is taken from the producer's declared
`source_model` value (`mycelix-multiworld-sim/<package version>`). The optional
`source_revision` still records the exact build revision only when the build
environment supplies it; a package version must not be mistaken for an exact
commit identity.

## Single-artifact end-to-end qualification

The script `qualify_real_artifact.sh` performs the full local path against the
checked-out producer and standalone consumer. It first runs the
monorepo's `humanity_atlas_roundtrip` integration test, which exercises the
simulator-backed producer-to-core importer seam, then captures one simulator
output and stores its SHA-256. It validates the captured `technology_tree`
and `run_manifest` independently against both published Draft 2020-12 producer
schemas before running the Python adapter and generic run-envelope schema gate.
It then runs the Sol-Atlas core unit tests and standalone Rust example against
those exact source bytes and generated envelope. The producer round-trip test
is mandatory and its output log is included in the evidence packet; the
producer-schema validation log is hashed into the evidence metadata as well. It requires all three source
checkouts (monorepo, Symtropy, and Sol-Atlas) to be clean and records all three
HEADs, tool versions, source/envelope/report and log digests, the consumer
report, and test logs in an evidence directory. It snapshots and hashes both the simulator and
standalone core Cargo lockfiles, then uses `cargo run/test --locked` for every
build, test, and importer invocation. If either checkout lacks a lockfile, the
script resolves it once, records whether it was generated during qualification,
and preserves its exact bytes and SHA-256 in the evidence packet. It checks that
neither lockfile changes during the run, removes only lockfiles it generated,
and verifies that all three source checkouts are clean again before recording a
pass. Because GitHub's ambient `GITHUB_SHA` identifies the Symtropy workflow
commit, the script overrides the producer's compile-time revision variables with
the monorepo's recorded HEAD; it rejects the captured artifact and consumer
report unless they declare that same producer revision. The producer and consumer
compile into fresh temporary `CARGO_TARGET_DIR` directories, so a previously
cached binary cannot silently retain an old compile-time revision. Those build
directories are removed on exit and are not included in the evidence upload.
The adapter source,
its tests, the run schema, and the producer/consumer copies of both versioned
schemas (technology tree and run manifest) are snapshotted and hashed. The
script fails unless both producer/consumer schema pairs are byte-identical, and
validates the actual run envelope against the snapshotted run schema. By default
the evidence directory is under `TMPDIR` (or `/tmp`), outside source trees.

The qualification gate requires the Python `jsonschema` package; the ordinary adapter unit test can skip schema validation when run without it, but the qualification script sets `HUMANITY_ATLAS_REQUIRE_JSONSCHEMA=1` and fails if it is missing. Add `python3Packages.jsonschema` to the temporary Nix shell or install `jsonschema` in the Python environment before qualifying.

From a checkout of this Symtropy repository, set the two external repository paths (the script infers the Symtropy checkout from its own location):

```sh
LUMINOUS_MONOREPO_ROOT=/path/to/luminous-dynamics \
SOL_ATLAS_ROOT=/path/to/sol-atlas-leptos \
ATLAS_QUALIFICATION_DIR=/path/to/humanity-atlas-evidence \
bash docs/ops/humanity_atlas_scenario_contract_v0_1/qualify_real_artifact.sh
```

The example validator is
`sol-atlas-core/examples/verify_humanity_atlas_artifact.rs` in Sol-Atlas;
it verifies the envelope, checks the SHA-256 of the exact source bytes, and
validates the technology-tree artifact independently of the UI. The validation
report's `structural_contract_pass` label means only that the declared contract,
digest, provenance bindings, summary metrics, and graph checks passed. It does
not establish publisher authenticity, deterministic replay, empirical
calibration, or scientific validity.

The script intentionally fails on the first failed command/test. A missing
toolchain, compilation failure, rejected artifact, or schema mismatch must
remain a failure rather than becoming a partial success receipt. An exit trap
writes `qualification-execution-status.json` with the last phase, repository
HEADs, tool versions, and hashes of whichever artifact/log files exist—even when
a gate fails early. This status record is not a qualification pass; only a zero
exit after all gates and the separate `qualification-metadata.json` record can
indicate structural qualification success. Fresh temporary Cargo target
directories prevent cached binaries from smuggling in a previous compile-time
revision and are removed on exit. The script is provided as a qualification
path; it has **not** been executed in this editing environment. Queued CI is not
counted as a pass.
