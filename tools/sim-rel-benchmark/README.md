# SIM-REL-001 isolated reference harness

This package is intentionally isolated with its own empty workspace and has **no third-party dependencies**. It does not modify or weaken Symtropy's main workspace.

## What it currently proves

The first vertical is a reference-parity smoke test only. In addition to the 5-case smoke corpus, the qualification candidate corpus contains 42 deterministic cases (6 train, 6 validation, 30 held-out) across three nominal difficulty tiers. It is coverage for the reference simulator, not yet a qualified model-discovery benchmark. It reads the synthetic TSV corpus and simulates each linear spring-mass scenario using two independently implemented force calculations:

1. direct edgewise spring-force accumulation;
2. a stiffness matrix assembled from the same declared spring graph.

Both use the same declared semi-implicit Euler integrator. The harness checks their complete state traces against a fixed absolute tolerance, validates core scenario fields, rejects duplicate scenario IDs and malformed records, hashes the exact corpus bytes with an in-process SHA-256 implementation, and binds the receipt to the exact checked-out Git HEAD.

**It does not implement a discovery agent, infer a grammar, compare model quality to SOTA, validate physical measurements, or prove Chain Theory.** The two force paths solve the same known equations; parity is an implementation check, not a scientific discovery result. The five-case smoke corpus is a quick smoke test. The 42-case qualification candidate corpus meets the protocol's count/split target, but it remains **unqualified until its tests and receipts are executed on the exact head**; it still checks solver parity, not grammar-discovery quality.

## Commands

Run from the Symtropy repository root on the exact commit to qualify:

```bash
cargo fmt --manifest-path tools/sim-rel-benchmark/Cargo.toml -- --check
cargo test --locked --manifest-path tools/sim-rel-benchmark/Cargo.toml
EXPECTED_HEAD="$(git rev-parse HEAD)"
cargo run --locked --manifest-path tools/sim-rel-benchmark/Cargo.toml --bin sim-rel-reference -- \
  --corpus tools/sim-rel-benchmark/fixtures/spring-mass-v1.tsv \
  --expected-head "$EXPECTED_HEAD" \
  --out /tmp/sim-rel-reference-receipt.json
cargo run --locked --manifest-path tools/sim-rel-benchmark/Cargo.toml --bin sim-rel-reference -- \
  --corpus tools/sim-rel-benchmark/fixtures/spring-mass-qualification-v1.tsv \
  --expected-head "$EXPECTED_HEAD" \
  --out /tmp/sim-rel-qualification-receipt.json
```

The executable rejects a mismatch between `--expected-head` and `git rev-parse HEAD`. To bind the run to a known reviewed SHA, set `EXPECTED_HEAD` to that exact SHA rather than deriving it from the checkout.

For a quick view, pass `--out -` to emit the JSON receipt on standard output. Failed parity cases write the receipt (if the destination is writable) and exit non-zero.

## TSV contract

One record per line, UTF-8 text:

`id|split|difficulty|masses_csv|x0_csv|v0_csv|damping_csv|dt|steps|springs_semicolon|forces_semicolon|observation_stride`

- Lines beginning with `#` and blank lines are ignored.
- `id` accepts only ASCII alphanumeric characters, underscore and hyphen; IDs must be unique.
- `split` is `train`, `validation`, or `heldout`; `difficulty` is `easy`, `medium`, or `hard`.
- Vector fields contain finite comma-separated numbers; masses must be positive, damping non-negative, and vector lengths equal.
- Each spring is `node_a,node_b,stiffness`; node indices are zero-based and `node_b=-1` denotes an immovable, zero-displacement anchor.
- Each force pulse is `node,start_step,end_step,amplitude`; the start is inclusive and the end exclusive. The literal `none` means no springs/pulses (scenarios must still declare at least one spring).
- `dt` must be finite and positive; steps are bounded to 1..=100000 and observation stride is positive. Parity is checked at every integration step; RMSE is reported only at the declared observation stride (always including the initial and final states).
- Scenario generation is deterministic and currently noise-free. Noise, repeated trial/participant structures, active intervention selection, and grammar inference are future increments. The 42-case corpus includes 30 held-out compositions, but it is not qualified until the exact-head tests and receipts have actually run.

## Receipt fields

The stable JSON receipt contains:
- schema version and `synthetic_only: true`;
- exact Git HEAD and SHA-256 of the raw corpus bytes;
- case count and overall parity disposition;
- per-case observation stride and sampled-state count;
- per-case ID, split, step count, recorded state count, maximum absolute state delta, RMSE, and PASS/FAIL.

The receipt contains no timestamps, random seeds, or host-specific paths, so repeat output should be byte-stable for a fixed Git HEAD and corpus bytes. Float values are serialized with 17 digits after the decimal in scientific notation. SHA-256 is implemented locally and includes a known-vector unit test. Tests also check edge-order invariance, node relabeling invariance, observation-stride accounting, and fail-closed behavior for non-finite simulation results.

## Next implementation gates

1. Run the isolated format/test/CLI commands on the exact PR head and retain output/digests.
2. Execute the existing relabeling, non-finite-state, endpoint, parity, and corpus-coverage tests; add invalid-force and numerical-stability controls as gaps are found.
3. Audit whether the 30 held-out compositions provide sufficient topology diversity and whole-family separation; preregister any corpus revision before model comparison.
4. Add a conventional graph baseline that is not merely a second implementation of the exact same equations.
5. Add a truly distinct relational-grammar candidate and an explicit shared-grammar rejection path.
6. Add the same-budget intervention-selection policy comparison.
7. Compare against published methods under matched datasets/budgets before making any superiority claim.

## Active intervention-selection prototype (new, separate scorecard)

The `sim-rel-active-discovery` binary adds a first runnable *design* for the currently missing intervention-selection comparison. It is intentionally separate from the spring-mass simulator parity score.

It compares three policies over finite binary candidate models:

1. `active_greedy_max_split`: choose the available intervention that most evenly splits the remaining candidate models by predicted outcome.
2. `fixed_ascending_action`: query available interventions in increasing action order.
3. `seeded_random_action`: query in a deterministic pseudo-random order; 64 seeds per task.

All policies receive the same candidate models, initial observations and available actions. Each action is one of eight binary input assignments. The policy-selection function is not passed the target label. Evaluator truth labels are stored in a separate TSV fixture and joined only by the runner for outcome scoring.

The 24-task candidate fixture contains 6 train, 6 validation and 12 held-out tasks. Two held-out negative controls contain hypotheses with identical observable signatures; a correctly cautious policy must stop with ambiguity rather than invent a distinction. Metrics include exact model identification, ambiguity/abstention, truth-retention (false elimination), intervention count, median and p90 query count, with results by split and policy.

Run on the exact checked-out repository head:

```bash
cargo fmt --manifest-path tools/sim-rel-benchmark/Cargo.toml -- --check
cargo test --locked --manifest-path tools/sim-rel-benchmark/Cargo.toml --bin sim-rel-active-discovery
EXPECTED_HEAD="<reviewed-40-hex-commit>"
cargo run --locked --manifest-path tools/sim-rel-benchmark/Cargo.toml --bin sim-rel-active-discovery -- \
  --problems tools/sim-rel-benchmark/fixtures/active-discovery-problems-v1.tsv \
  --truth tools/sim-rel-benchmark/fixtures/active-discovery-truth-v1.tsv \
  --expected-head "$EXPECTED_HEAD" \
  --out /tmp/sim-rel-active-discovery-receipt.json
```

For a stdout receipt, use `--out -`. The receipt records the exact Git HEAD plus SHA-256 digests of both fixtures. Do not commit evaluator truth into an agent-visible problem bundle in any deployment where the agent can read the file system directly; this offline runner uses separate files and a policy function with no target-label argument, but external process isolation is still required for a real red-team evaluation.

**Claim ceiling:** This is a finite synthetic causal-model discrimination and experiment-selection prototype. It does **not** implement a relational-grammar learner, does not establish that the active policy outperforms the baselines until the exact-head runner produces a receipt, is not a directly comparable NewtonBench/SRBench score, and provides no evidence for Chain Theory's physical claims. The boolean model class is deliberately small; next requirements are noisy/partial observations, graph/grammar induction, heterogeneous-world rejection, budget-matched experiment choice, and an independent corpus.
