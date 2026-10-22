# Engine-grounded PCB routing benchmark protocol v0.1

**Status:** evaluator contract and synthetic fixtures only. No PCBWorld dataset has been downloaded, no external router run has been executed, and no state-of-the-art comparison is claimed.

## Scope: copper routing, not manufacturing route economics

This benchmark evaluates the geometric task of routing PCB copper traces: connect all required nets while respecting a pinned engine's routing DRC. It does not compare local versus outsourced fabrication or assembly, cost per accepted assembly, board milling, or workcell dispatch. Those are separate Symtropy programs (#2385 and #2397).

## Reference benchmark and provenance

The proposed external reference is [PCBWorld](https://github.com/LGAI-Research/PCBWorld) and [PCBWorld-Bench (arXiv:2607.05915)](https://arxiv.org/abs/2607.05915), published July 2026 for the KDD 2026 Workshop. The paper describes grid-based and gridless synthetic tasks and 679 real open-source boards, with methods checked by native KiCad routing and DRC. It compares rule-based routers, reinforcement-learning policies and LLM agents. Its reported result is more nuanced than “learning loses to rules”: grid-action RL can collapse as grids get finer, while the segment-level native-engine PPO policy keeps performance across grid sizes, transfers zero-shot from synthetic boards, beats other methods including rule-based routers on the small real-board stratum, and ranks behind the strongest rule-based router on the larger real-board stratum. That is the key prior to reproduce: the representation/action space and feedback loop can matter more than model scale.

Do not copy headline rates into a result or compare numbers casually. First pin the same benchmark revision and split, each board's original license and provenance, KiCad and DRC engine build, router version and configuration, command line, timeout, hardware class, and DRC/connectedness policy. The published 96% completion figure in a separate DreamerV3/FreeRouting study is a different setup and is not a valid numerical comparator until those details are aligned.

## Inputs and proof boundaries

The Rust evaluator accepts an explicit dataset manifest, one attempt per pinned board, and a run report that references the manifest's SHA-256. It validates:

- exact benchmark/dataset/split identity and the manifest's raw-byte digest;
- full board coverage, no duplicate or unexpected board IDs, matching source-board digests and required-net counts;
- source/binary pins for router and DRC engine, DRC policy digest, and machine/environment declaration;
- execution status, required/connected net counts, DRC-evaluated state and reported route violation types;
- no silent removal of timeouts, router errors, incomplete nets, crossings or missing output measurements.

**A valid route completion is derived by the evaluator**, never trusted from a router-provided success label. A completion requires: tool execution finished; every required net is connected; routing DRC was evaluated and has zero routing violations; there are zero dangling tracks/vias; the output-board hash exists; and wirelength and via count were measured. Upstream non-routing DRC is reported separately.

## Initial synthetic control suite

Six artificial board IDs exercise full completion, partial net connection, connected copper crossings, timeout, router error, and a board with separately declared upstream non-routing DRC findings. The baseline fixture intentionally derives 2/6 valid completions; the synthetic treatment fixture derives 4/6. These numbers only test evaluator logic—they do not represent an actual router, measured board, benchmark score, or state-of-the-art result.

Run locally with a pinned evaluator commit:

    export PCB_ROUTE_BENCHMARK_GIT_HEAD="$(git rev-parse HEAD)"
    MANIFEST=tools/pcb-route-benchmark/fixtures/synthetic-dataset-manifest-v0_1.json
    MANIFEST_SHA=$(sha256sum "$MANIFEST" | cut -d ' ' -f 1)
    jq --arg sha "$MANIFEST_SHA" '.dataset_manifest_sha256 = $sha' \
      tools/pcb-route-benchmark/fixtures/synthetic-baseline-run-v0_1.json > /tmp/baseline.json
    jq --arg sha "$MANIFEST_SHA" '.dataset_manifest_sha256 = $sha' \
      tools/pcb-route-benchmark/fixtures/synthetic-treatment-run-v0_1.json > /tmp/treatment.json
    cargo run --manifest-path tools/pcb-route-benchmark/Cargo.toml -- validate /tmp/baseline.json "$MANIFEST"
    cargo run --manifest-path tools/pcb-route-benchmark/Cargo.toml -- validate /tmp/treatment.json "$MANIFEST"
    cargo run --manifest-path tools/pcb-route-benchmark/Cargo.toml -- compare /tmp/baseline.json /tmp/treatment.json "$MANIFEST"

## Statistics and fair comparisons

For paired comparison, the evaluator refuses to compare runs with different board IDs/input digests, benchmark split, manifest digest, DRC policy, engine pin or execution environment. Completion difference and per-board runtime delta retain the full six-attempt denominator; wirelength/via deltas are computed only on boards that are valid in both methods. A deterministic 10,000-resample paired bootstrap interval for completion and runtime mean deltas is included, with seed material bound to both run report hashes and the manifest digest.

For actual PCBWorld experiments, begin with 30 stratified, license-cleared boards for pipeline qualification, then expand to the full declared split. Use held-out source-board families, publish every attempt and timeout, and report strata by grid/gridless family and board complexity. Compare a pinned rule-based router, native KiCad PNS where appropriate, an available reproducible published baseline, and Symthaea-guided closed-loop routing. Include an ablation that uses the same actions and engine but removes the guidance.

## Proposed targets (not results)

1. **Zero false accepts:** no route completion is accepted with a crossing/short, clearance conflict, dangling item, open required net, unevaluated DRC, or absent final-board digest.
2. **Full denominator:** every declared board has exactly one result row; all timeouts and tool errors remain in the completion denominator.
3. **Statistical comparison:** same input boards and environment; paired confidence intervals and board-level results; no claim based only on successful boards.
4. **Stretch advantage:** on a frozen held-out split, target either a completion gain of at least 5 percentage points with paired uncertainty reported or a Pareto improvement at equal completion. On boards valid for both methods, target at least 10% lower median runtime without worsening validity; wirelength/via trade-offs remain explicit. If the baseline wins, report it honestly.
5. **Generalization:** no hidden collapse on larger boards; stratify results, publish failure categories and never tune on the holdout.

## Integration and licensing boundary

PCBWorld's environment is BSD-3-Clause while its separate router/engine dependency is GPLv3; board files preserve their own source licenses. Use the environment and KiCad as pinned external processes, not vendored or linked production dependencies. The Rust evaluator checks receipts/output facts and does not implement an alternative geometric router. A valid DRC route still does not prove controlled impedance, signal/power integrity, EMC, fabrication conformity, solder quality, electrical safety, or production readiness.

## References

- PCBWorld: https://github.com/LGAI-Research/PCBWorld
- PCBWorld-Bench, arXiv:2607.05915: https://arxiv.org/abs/2607.05915
- KiCad 9 CLI DRC and manufacturing export: https://docs.kicad.org/9.0/en/cli/cli.html


## Source-pin and dataset intake constraints (verified against upstream 2026-10-21)

The upstream PCBWorld README currently identifies release v1.1.1, its KiCad 9.0.8 engine, BSD-3-Clause for the environment and a separately distributed GPLv3 engine. The repository's main tree at intake was `1989a9325796b73cca793a0c8f222dd1509256e8`; its `engine/` gitlink was `30d6a68261b8631e58f0a39c915b1cd9467bf00a`; `configs/datasets/d3.json` was blob `61d86bbf6cfd8ff8d1e13089c8d98a90f6f81bb3`; and the KDD quickstart split mapping was blob `c1ef6761206c24ec6cf70e3232eba4355765793e`. Reconfirm these pins before any real run and bind them into the dataset manifest.

Upstream documentation distinguishes 679 PCBench source boards from the 677-board D3 evaluation pool; two boards are intentionally excluded for the stated slot-cap / zero-clearance reasons. D3-A is an easy 99-board subset; the published D3 pipeline is evaluation-only and its manifest's train/test keys intentionally mirror the board list. Do not pretend that those identical keys form a held-out split for this new comparison, and do not rebuild boards silently: upstream documents that regenerated files can differ in per-net widths on 40 guide boards and inferred diff-pair class parameters. For strict comparisons, use the pinned published board files and published split definitions, preserve each board's original license, and construct an additional source-family holdout only as a separate disclosed evaluation.

At the pinned PCBWorld main commit `1989a9325796b73cca793a0c8f222dd1509256e8`, the top-level README identifies v1.1.1 and names FreeRouting/KRT; the dedicated `methods/baselines/rule_based/README.md` describes the unified runner for FreeRouting, KRT and OrthoRoute. Record the actual methods and exact tool pins available in the checkout and runnable on the declared environment, rather than treating a summary list as proof that every adapter is usable. The upstream evaluator documents a staged process (rollout, post-hoc native DRC, aggregate) and metrics such as connectivity success, routability, track/via counts, wirelength, DRC violation classes and upstream board metadata. The Rust evaluator's normalized input must be produced by an adapter that faithfully maps these metric semantics and carries raw DRC reports and output-board digests; a summary CSV or router-reported success value alone is not evidence enough.

Primary upstream sources checked: [PCBWorld README](https://github.com/LGAI-Research/PCBWorld/blob/main/README.md), [dataset index](https://github.com/LGAI-Research/PCBWorld/blob/main/configs/datasets/README.md), [D3 dataset manifest](https://github.com/LGAI-Research/PCBWorld/blob/main/configs/datasets/d3.json), [evaluation metric contract](https://github.com/LGAI-Research/PCBWorld/blob/main/eval/README.md), and [KDD reproduction index](https://github.com/LGAI-Research/PCBWorld/blob/main/experiments/kdd/README.md).


## Normalization adapter contract for PCBWorld outputs

The first adapter should preserve both raw upstream reports and a normalized Rust input, because PCBWorld distinguishes several outcomes that a single completion bit would erase:

- Connectivity success: every net's pads form one connectivity group; this is not sufficient if dangling copper exists.
- Routability: use upstream's stated reduction in pad connectivity groups, with the pinned initial-board baseline, rather than recomputing a different formula.
- Physical routing proxies: engine-reported track count, via count and wirelength in millimetres.
- DRC dimensions: error-severity violations only, and errors plus the three promoted warning codes 12/13/37, with the per-violation severity, label, error code/type, coordinates, layer, net names, error/promoted flags retained.
- Board provenance: original source-board hash, matching source .kicad_pro design-rule file, DRC policy/configuration digest, router/engine version and output-board hash.

The published evaluation pipeline is staged: rollout produces artifacts; post-hoc DRC scores the saved routed board against its source project rules; aggregation computes split/method results. The adapter must retain those stage boundaries and attach the raw DRC report; a single summary CSV or a router's own success label is insufficient proof. Record the policy-specific results rather than mixing the paper recipe's scoring policy with the newer shipped default. In particular, a score under one warning-promotion / reward config is not interchangeable with a score under another.

Do not claim the output-board digest is independently verified merely because it appears in the normalized row. The actual adapter must hash the bytes of the saved PCB output, compare that to the report, and then run the pinned DRC engine against that exact saved file. The normalized evaluator validates structure/coverage/pairing; provenance of physical bytes and DRC execution comes from the adapter's raw-file receipts and external tool process logs.


## Industry-reference track: FreeRouting continuous benchmarks

FreeRouting publishes a separate live benchmark page with per-tier counts, clean-DRC completion, fully-routed rate, timeouts, failures, total time and score. On the page generated **2026-10-20** on an AMD Ryzen 5 3600 (6 cores, 31.9 GB RAM), its aggregate table reports 1,142 fixtures for FreeRouting 2.6.0-RC3: 1,002/1,142 clean (87.7%), 1,003/1,142 fully routed (87.8%), 21 timeouts (1.8%) and 5 failures (0.4%). The next row, 2.6.0-RC4, reports a 979-fixture subset with 905/979 clean (92.4%), 906/979 fully routed (92.5%), and no timeouts/failures. Because the denominator changes from 1,142 to 979, RC4's apparent improvement is **not a paired version comparison** and must not be represented as a like-for-like gain. FreeRouting's Tier A canary set has 452 fixtures and reports 100% clean/fully-routed for RC4 in that snapshot; it is explicitly an easier fast-solving two-layer subset, not representative of complex multi-layer boards.

This is a useful public industry reference, but it is not directly comparable to PCBWorld-Bench results: the fixture sets, versions, board-project conventions, engine use, DRC scoring and time budgets differ. Use these results to design sanity checks for the evaluator and stratification requirements, not to claim superiority. Our direct comparator should still be a pinned FreeRouting binary (when reproducibly runnable through the PCBWorld evaluation harness), tested on identical PCBWorld board bytes, DRC rules and time budgets.

The industry scorecard therefore has two layers:

1. **Like-for-like research comparison:** same 30-board pilot / eventual full pinned PCBWorld split, router outputs checked through the same KiCad engine, matched source hashes, same host limits and all attempts included; route validity first, then completion/time/wire/via Pareto metrics.
2. **External context only:** FreeRouting's own versioned live benchmark and tier tables, clearly labelled as its own fixture suite. Compare its methodology (clean DRC, fully routed, timeouts, failures, board tiers) and then reproduce a comparable shared-board run before numerical head-to-head claims.

FreeRouting reference: https://github.com/freerouting/freerouting/blob/master/website/benchmarks.html. Snapshot values above were read from its page generated 2026-10-20; record a fresh page timestamp and exact tool version before any future publication.


### Raw evidence digests are mandatory for a valid completion

Each per-board attempt now includes `drc_report_sha256`, the SHA-256 of the raw DRC report generated for that saved output board. When the digest is absent, an otherwise connected zero-violation summary is categorized as `evidence_incomplete`, not a valid completion. A malformed digest rejects the report. A normalized row's string alone is not proof of correct provenance: the actual adapter must hash the real DRC bytes and bind the report to the exact output-board digest, DRC engine and DRC policy. The synthetic fixtures use explicitly synthetic digests and must never be presented as real KiCad measurements.


## Source-byte intake verification

External benchmark manifests now require a safe dataset-relative path for every board in addition to license, source URL, board digest and required-net count. Run `verify-dataset MANIFEST.json DATASET_ROOT` before route evaluation. The Rust verifier resolves each path under the canonical dataset root, rejects absolute paths, `..`, `.`, empty path components, backslashes, drive-prefix colons and symlink targets that canonicalize outside the root, then hashes the bytes actually read. Its receipt records each file's relative path, byte length and computed SHA-256, and binds the raw manifest digest and evaluator commit.

Example:

    export PCB_ROUTE_BENCHMARK_GIT_HEAD="$(git rev-parse HEAD)"
    cargo run --manifest-path tools/pcb-route-benchmark/Cargo.toml -- \
      verify-dataset pinned-dataset-manifest.json /path/to/immutable/PCBWorld/data

This is a separate gate from report validation: a report's digest must match the input board digest pinned by the manifest, and the intake receipt must show that the manifest digest corresponds to the actual bytes in the dataset tree. Keep the original dataset read-only throughout runs. The unit test writes temporary synthetic files to test digest tampering and path escape; those test bytes are not PCB layouts and are not benchmark evidence.


## Deterministic manifest builder from a pinned PCBWorld split

The intake path now has a first-party Rust manifest builder rather than requiring manual SHA entry. Prepare an intake specification whose board list is the *declared evaluation subset*—for example, 30 licensed boards stratified across the upstream easy/medium/hard test lists. Each row must state board ID, upstream difficulty, required-net count, verified license and an HTTPS source URL. The specification also declares the exact PCBWorld source commit, split name, benchmark/dataset/split IDs and release version.

Example intake-spec shape:

    {
      "schema": "pcb-route-intake-spec-v0.1",
      "benchmark_id": "pcbworld-bench",
      "benchmark_version": "v1.1.1",
      "dataset_id": "pcbench-d3",
      "split_id": "d3-test-pilot30-v0.1",
      "source_url": "https://github.com/LGAI-Research/PCBWorld",
      "source_split_commit_sha": "PINNED_40_HEX_UPSTREAM_COMMIT",
      "source_split_config_url": "https://github.com/LGAI-Research/PCBWorld/blob/PINNED_40_HEX_UPSTREAM_COMMIT/configs/datasets/d3.json",
      "source_split_config_git_blob_sha1": "PINNED_40_HEX_GIT_BLOB_SHA1",
      "source_split_config_relative_path": "provenance/d3.json",
      "split_name": "test",
      "boards": [
        {
          "board_id": "EXACT_ID_FROM_D3_JSON",
          "difficulty": "easy",
          "required_nets": 4,
          "license": "VERIFIED_LICENSE_IDENTIFIER",
          "source_url": "HTTPS_URL_TO_ORIGINAL_BOARD_SOURCE"
        }
      ]
    }

Place the exact upstream `d3.json` bytes inside the declared dataset evidence root and build the manifest from the same byte sequence. The builder refuses a root copy that differs from the supplied config; `verify-dataset` reopens and checks the copy again before accepting the dataset:

    mkdir -p /data/pcbworld/pcbench/exacad_sorted/provenance
    cp /pinned/PCBWorld/configs/datasets/d3.json \
      /data/pcbworld/pcbench/exacad_sorted/provenance/d3.json

    export PCB_ROUTE_BENCHMARK_GIT_HEAD="$(git rev-parse HEAD)"
    cargo run --manifest-path tools/pcb-route-benchmark/Cargo.toml -- \
      build-manifest /pinned/PCBWorld/configs/datasets/d3.json \
      intake-spec.json /data/pcbworld/pcbench/exacad_sorted \
      > pinned-dataset-manifest.json

Then verify the actual file bytes before a run:

    cargo run --manifest-path tools/pcb-route-benchmark/Cargo.toml -- \
      verify-dataset pinned-dataset-manifest.json \
      /data/pcbworld/pcbench/exacad_sorted \
      > dataset-intake-receipt.json

The builder proves each selected ID belongs to the explicitly named difficulty/split array in the supplied D3 configuration, derives the relative board path from D3's `board_filename`, hashes the board bytes it actually reads, and binds hashes of the raw split-config and intake-spec bytes plus the source Git commit into the resulting manifest. It also requires the immutable split-config URL to contain that commit, checks its Git blob SHA-1, and requires the copy under `source_split_config_relative_path` to match the supplied bytes exactly. `verify-dataset` reopens that split file and verifies both SHA-256 and Git blob SHA-1 alongside board and license-evidence files. It also requires an immutable GitHub blob URL that contains the pinned commit, requires the expected source Git blob SHA-1, and computes Git's `blob <length>\0<bytes>` identifier from the supplied split-config bytes before reading membership arrays. A modified `d3.json` cannot be relabelled with a guessed commit SHA and accepted merely because its changed bytes hash consistently. It rejects absent boards, unknown license placeholders, non-HTTPS provenance, unsafe paths, duplicate IDs and IDs outside the pinned split. Keep `d3.json`, PCBWorld commit, engine submodule, intake spec, manifest and intake receipt as the experiment's immutable inputs.

**Do not treat the two identical D3 train/test lists as a train/test holdout.** An explicitly selected 30-board pilot is a pipeline qualification subset; freeze the selection before routing methods are compared, then run the entire declared split for the final score. A future source-family holdout should be a separate disclosed split, not inferred from D3's mirrored keys.


### Frozen candidate pilot: 30 boards across all three D3 difficulty tiers

The initial candidate list is checked into `tools/pcb-route-benchmark/fixtures/pcbworld-d3-pilot-candidates-v0_1.json`. It records the upstream PCBWorld main commit `1989a9325796b73cca793a0c8f222dd1509256e8` and the D3 split JSON's Git blob ID `61d86bbf6cfd8ff8d1e13089c8d98a90f6f81bb3`, both retrieved from the upstream GitHub API on 2026-10-21. The 30 candidates are stratified as 10 easy boards sampled at deterministic evenly spaced indices from the 99-board easy test list, all 10 entries in the upstream medium test list, and all 10 entries in the upstream hard test list.

This file is intentionally a **candidate set**, not an executable intake manifest. The separate source-metadata audit has re-pinned 14 license files at immutable source commits, and records that 16 candidates remain unresolved. No entry is benchmark-authorized yet: the exact prepared `processed_v9_guide_v3.kicad_pcb` bytes and required-net counts are not locally verified, and the selected license evidence still requires a separate applicability review. The builder must reject placeholder license entries and verify the byte hash of the actual files under the declared dataset root. Required-net counts must be measured using the pinned EDA/evaluation semantics, not guessed from the board name. D3's medium/hard “test” lists are only ten boards each; this 30-board pilot spans the published difficulty labels but is not a statistically representative sample of the full 677-board D3 pool.

Git blob SHA-1 metadata identifies the upstream JSON object; the manifest builder separately calculates SHA-256 over the exact raw JSON bytes for cryptographic receipt binding. Do not substitute one for the other.


### Connectivity truth-source invariant

Every board attempt carries `unconnected_item_count` from the native DRC/ratsnest result. A valid-route disposition requires the count to be **explicitly present and zero**. If the normalized adapter says all nets are connected while the native DRC report still lists unconnected items, the evaluator classifies the case as `incomplete_nets`; if the count is missing, it is `evidence_incomplete`. A run where DRC was not evaluated must not claim an unconnected-item count or DRC-report digest. The receipt summarizes both the observed total and how many attempts lack that observation, so unknown is not silently normalized to zero.


## Current external competitive landscape (checked 2026-10-21)

These are **separate reference measurements, not a shared leaderboard**. Record the exact source revision and reproduce on matching inputs before ranking methods.

| Public source | Author-reported result | What it is useful for | Why it is not directly comparable yet |
|---|---|---|---|
| [PCBWorld-Bench / v1.1.1 source](https://github.com/LGAI-Research/PCBWorld) | 679 source boards; the shipped D3 evaluation pool has 677 after the documented exclusions; D3-A is 99 small-board cases | Primary engine-grounded challenge: same KiCad routing/DRC semantics, synthetic and real boards | Requires exact source bytes, board split, engine submodule, DRC policy, and evaluation recipe |
| [FreeRouting official benchmark page](https://github.com/freerouting/freerouting/blob/master/website/benchmarks.html) | Its page generated 2026-09-30 reports 1,157 fixtures for v1.9.0, 568/1,157 fully routed (49.1%) and 415/1,157 DRC-clean (35.9%) | Large ongoing regression fixture set; separates connectivity from DRC cleanliness | Different board corpus and rules; do not compare its aggregate percentage directly to D3-A or D3-B |
| [FastRoute project benchmarks](https://fastroute.baltavista.com/) | Project reports 2,102 s → 420 s over 20 boards in parity mode and roughly 20–300 MB versus 1–2 GB peak memory | Performance/memory baseline; test its Rust port independently | Project-authored small-suite data, parity mode is not the same as quality-maximizing output, and its DRC examples use a separate set |
| [ProtoFlow public benchmark page](https://www.protoflow.ai/benchmarks) | Vendor reports 98/99 D3-A small boards KiCad-clean; D3-B report 30/30 attempts clean across 10 boards × 3 attempts; also provides a router-only figure | A valuable recent external point of comparison on PCBWorld D3-A/B | Vendor-produced results and tool/agent stack need independent reproduction under pinned identical rules and timeouts |
| [PUNKFAB four-router case study](https://punkfab.com/blog/route-evals/) | On one 4-layer, 42-component board, the post reports different trade-offs across four methods and that none of the tested outputs was directly orderable | End-to-end DFM: routing success is not equivalent to fabrication readiness | One board, author-specific manufacturing rules, and no broad statistical generalization |

### Pinned PCBWorld RQ2 paper snapshot (historical reference only)

The published baseline table at PCBWorld commit `1989a9325796b73cca793a0c8f222dd1509256e8` (table blob `215b59030eaab5130b50a94867f8ed07ed84ea47`) reports the following D3-A / PCBench fair-95 means:

| Historical method | Routability (all boards) | DRV | Wirelength | Vias | Time |
|---|---:|---:|---:|---:|---:|
| FreeRouting | 0.98 | 0.09 | 158.82 mm | 0.72 | 3.90 s |
| OrthoRoute | 0.99 | 111.93 | 294.04 mm | 21.86 | 2.20 s |
| KiCadRoutingTools (KRT) | 0.94 | 1.01 | 156.01 mm | 3.74 | 0.65 s |

Source: [pinned RQ2 table](https://github.com/LGAI-Research/PCBWorld/blob/1989a9325796b73cca793a0c8f222dd1509256e8/methods/baselines/rule_based/eval/RQ2_TABLE.md). In that table, routability is averaged over the full set, while DRV/wirelength/vias are averaged only on boards with routability exactly 1.0; FreeRouting uses four seeds and sample standard deviation, and time comes from a per-tool wall-clock CSV. These are **author-reported historical figures**, not measurements from this PR and not the same as our strict valid-completion rate.

There is a concrete reproducibility trap: the current pinned `methods/baselines/rule_based/README.md` says its OrthoRoute commit lays routing tracks on inner layers only, so a two-layer board ends at `No edges` with an error row. Consequently the historical `0.99` D3-A mean must not be treated as a current runnable baseline until the matching historical engine/source and data recipe are restored and verified. For the current tree, count each method as available only when an exact-head smoke run demonstrably parses inputs, produces a loadable routed board, and passes the same post-hoc DRC evaluator. A timeout/error remains in the denominator.
Sources: FreeRouting's page separates "fully routed" from "clean (0 DRC)" and records its generation date; FastRoute describes a 20-board parity-mode timing comparison; ProtoFlow labels its public benchmark metrics and attempt counts; PUNKFAB describes its single-board four-router experiment. These details are reported by each source, not independently reproduced by this project.

### Strongest next benchmark, in order

1. **D3-A reproduction**: all 99 held-out small real boards; use the upstream-pinned split/engine/policy. Reproduce the pinned PCBWorld rule-based baseline and a reference agent before modifying the algorithm. Run each method on byte-identical boards with equal resource/time limits.
2. **D3-B repeated outcomes**: 10 medium boards, three attempts each where that matches the referenced protocol. Preserve every attempt; report board-clustered uncertainty so three tries on the same board are not treated as three independent board families.
3. **D3-C hard-board stress**: report per-board outcomes and complexity strata; never let a gain on easy boards hide a collapse on large boards.
4. **Independent router baselines**: add FreeRouting and FastRoute only where an adapter can preserve exact inputs, rule transfer, router version/configuration, output bytes and post-hoc native DRC. Keep parity mode and best-quality mode as distinct experiment arms.
5. **Orderable-output gate**: separate copper-router validity from downstream CAD/CAM/manufacturing criteria: netlist parity, copper-zone refill, drill/plating/annular-ring constraints, trace/via manufacturing limits, mask/clearance, board outline, critical-net constraints, and target supplier DFM. A "DRC-clean" board is not automatically orderable.

### Stronger statistical unit

The board—not each random attempt—is the generalization unit. For multi-attempt stochastic methods, report (a) attempt-level rate as an operational metric, (b) per-board completion frequency/range across attempts, and (c) paired board-level treatment deltas. Bootstrap board IDs as clusters, carrying the board's repeated attempts together; do not treat multiple runs on the same board as independent board samples. The CLI now has `compare-series BASELINE_RUN_DIR TREATMENT_RUN_DIR MANIFEST.json`: it requires the same declared seed set in both methods, validates each run against identical board bytes/rules/engine/environment, and emits paired per-board rates plus 10,000-resample board-clustered intervals. The simpler `compare` command remains for one paired attempt per board. The series comparison was added after the earlier v0.1 single-run contract; treat its receipt schema (`pcb-route-replicated-comparison-v0.1`) as a separate, explicit evaluation output.


## Actual-byte run artifact verification (new CLI gate)

The Rust CLI now also exposes `verify-run-artifacts REPORT.json MANIFEST.json RUN_ROOT`. It validates the report against the pinned dataset manifest, then canonicalizes every completed attempt's output-board and raw DRC-report path under the declared run root, reads those bytes, computes SHA-256, and checks them against the hashes in the run report. It rejects missing paths/digests for completed attempts, path traversal, root-escaping symlinks, duplicate relative paths/canonical files, and digest mismatches. Its receipt binds the report/manifest hashes and exact evaluator commit.

That closes an integrity gap between "the normalized JSON says a board was clean" and "the native output board and DRC report that the claim refers to are actually present and hash-matched." It **still does not prove the normalized metric fields were extracted faithfully from the raw DRC JSON**; the eventual PCBWorld adapter must parse the pinned engine report, cross-check counts/types and preserve the raw DRC output and invocation metadata. The synthetic tests exercise filesystem/digest contract only, never an actual router.


## Outcome and observability are separate axes

The CSV adapter now reports both `success_rate_over_all_rows` / `clean_pass_rate_over_all_rows` and `success_rate_over_metric_reporting_rows` / `clean_pass_rate_over_metric_reporting_rows`, together with their respective observed-row counts. This is intentional:

- **Outcome over all attempts** keeps crashes, timeouts, skipped runs and missing measurements visible in the attempt denominator. It is the conservative operational-yield view.
- **Outcome among metric-reporting rows** describes only records that actually expose the metric. It must always be accompanied by measurement coverage, because it can look artificially strong if failed attempts fail to emit metrics.
- Never overwrite missing values with a synthetic false metric. Preserve the raw status/metric absence, show both views and publish the full denominator.

The new regression fixture includes an explicit crashed attempt with no `success` or `clean_pass` report: the conservative rate changes from 2/3 to 2/4, while the observed-metric rate remains 2/3 and reported-metric coverage falls to 3/4. This distinguishes algorithm quality from the benchmark adapter's observability rather than conflating them.


## Paired significance check for completion changes

In addition to the paired bootstrap confidence interval, the Rust evaluator now reports a two-sided **exact paired sign-test p-value** for completion outcomes. For a single-seed comparison, only discordant board pairs contribute: boards completed by the treatment but not baseline count in one direction, and boards completed by baseline but not treatment count in the other. For replicated comparisons, the sign test uses board-clustered completion-rate deltas; repeated seeds on one board do not inflate the independent sample count.

The p-value is reported as a supplement to effect size and the bootstrap interval—not a replacement for board-level results, stratification, practical effect thresholds or independent test sets. If both methods have the same completion result on every board, the sign-test p-value is `null` (no discordant pairs), not 1.0. The six-board synthetic 2/6 versus 4/6 fixture has two treatment-only completions and zero baseline-only completions, yielding an exact two-sided (p=0.5); it is deliberately not statistically persuasive.


## Native KiCad DRC bytes are now an independent verification input

The new `tools/pcb-route-benchmark/src/kicad_drc.rs` parser uses the actual KiCad DRC report layout from the official `drc.v1.json` schema and the violation-type taxonomy extracted from the **KiCad 9.0.9 tag**, not the current development branch. Source pins checked on 2026-10-21:

- DRC JSON schema blob: `3db8d82fc807c0690fd231eea9aefdb562576ce4`, [source](https://github.com/KiCad/kicad-source-mirror/blob/9.0.9/resources/schemas/drc.v1.json).
- Violation identifier taxonomy: `f6822d5faf1ad17f859d6c8eb13eb85c917f1f61`, [source](https://github.com/KiCad/kicad-source-mirror/blob/9.0.9/pcbnew/drc/drc_item.cpp).
- Policy: `tools/pcb-route-benchmark/fixtures/kicad-drc-routing-policy-v0_1.json`, pinned to KiCad `9.0.9`; its raw bytes are hashed into each run's `drc_policy_sha256`.

The new `verify-kicad-run-artifacts` CLI wraps actual-byte run artifact verification and adds a strict raw-report check. For every completed attempt it requires report source filename and KiCad version to match the verified board and engine pin; the report must include every severity the policy claims to block; every DRC rule type must be explicitly classified by the pinned policy; normalized routing-violation types and unconnected-item counts must exactly match the raw JSON; and the normalized post-route non-routing finding count must match the raw report. It rejects an unknown future rule type instead of silently treating it as non-routing.

Usage:

    PCB_ROUTE_BENCHMARK_GIT_HEAD="$(git rev-parse HEAD)" \
      cargo run --manifest-path tools/pcb-route-benchmark/Cargo.toml -- \
      verify-kicad-run-artifacts REPORT.json MANIFEST.json RUN_ARTIFACT_ROOT \
      tools/pcb-route-benchmark/fixtures/kicad-drc-routing-policy-v0_1.json

The receipt includes the raw DRC report digest, policy digest, exact reported version/source, counts by severity/type, routing and non-routing finding types, unconnected-item count and schematic-parity count. The run verifier cross-checks those derived facts against normalized per-board rows; a fabricated normalized `routing_drc_violation_types: []` can no longer hide a real `tracks_crossing` finding when the raw artifact is present.

This is a software evidence gate against the pinned KiCad output format—not a claim that the taxonomy is sufficient for every board class, nor a substitute for source netlist parity, design signoff, physical measurement or manufacturing qualification.


## License-evidence bytes are part of dataset intake

An upstream PCBench `metadata.json` record is provenance metadata, not legal clearance on its own. For every external board, the intake schema now requires a non-placeholder license label, a relative path to a non-empty copy of the source license/evidence text, its source URL, an immutable source commit SHA, and the source file's Git blob SHA-1. The license URL must contain that exact commit ID. Manifest construction calculates Git's `blob <length>\0<bytes>` SHA-1 object ID from the local evidence bytes and rejects a mismatch with the declared upstream blob ID; it also computes SHA-256 as the strong content digest. `verify-dataset` reopens the file under the dataset root and checks both digests. This binds the source-board SHA-256 separately from license evidence and records the source file's original Git blob identifier.

This proves which evidence bytes were used, not that the SPDX classification is legally correct or that the license covers every board artifact. The source audit now re-fetches 14 license files at their immutable source-repository commits and verified that every observed file's Git blob SHA matches the recorded blob SHA; 16 candidates still lack verified source-license evidence. The intake schema requires the license-source commit SHA and requires the license URL to contain that exact commit. None of the 30 boards is benchmark-authorized yet because exact prepared-board bytes and required-net counts are not locally verified, and license applicability remains a separate review. Keep unlicensed or ambiguous candidates out of public benchmark execution; replace them only under a documented deterministic selection rule, never silently.


### Split-config provenance regression

The manifest builder now verifies two independent source references for the upstream split file: the exact raw-byte SHA-256 used in the run receipt, and the upstream Git blob SHA-1 plus immutable commit URL. This closes an intake flaw where a modified D3 JSON could otherwise be supplied with a caller-chosen commit ID and still pass the board-membership check. The native dataset is not authorized until both pins match the actual file bytes. The SHA-1 is only used to verify Git object identity; SHA-256 remains the content digest used in the experiment receipt.


### The split file is part of the offline evidence bundle

The dataset evidence root must retain `provenance/d3.json` (or another declared safe relative path). A manifest with plausible split hashes but no corresponding local split file fails `verify-dataset`; a modified split file fails both the SHA-256 and Git-blob checks. The split file becomes a first-class entry in the dataset verification receipt, not merely an annotation pasted into manifest metadata.



### Verified comparisons are now a single fail-closed operation

The CLI's plain `validate`, `compare`, and `compare-series` commands are now restricted to manifests explicitly marked `evidence_class: synthetic`. They are useful for testing evaluator semantics, but cannot produce a normalized-only score for an external dataset.

For an actual external pair, use `compare-verified`:

    PCB_ROUTE_BENCHMARK_GIT_HEAD="$(git rev-parse HEAD)" \\
      cargo run --manifest-path tools/pcb-route-benchmark/Cargo.toml -- \\
      compare-verified BASELINE_REPORT.json TREATMENT_REPORT.json \\
      PINNED_MANIFEST.json DATASET_ROOT BASELINE_RUN_ARTIFACT_ROOT \\
      TREATMENT_RUN_ARTIFACT_ROOT \\
      tools/pcb-route-benchmark/fixtures/kicad-drc-routing-policy-v0_1.json \\
      > verified-comparison-receipt.json

This one operation (1) reopens and hashes every dataset board, license-evidence file and pinned source split config, (2) reopens each method's output-board and DRC files, (3) checks the raw DRC JSON against the version-pinned policy and normalized report, and (4) emits the paired comparison together with all three verification receipts. If one output, DRC report, source-license file, or source split is absent or tampered, no external paired score is emitted. The raw DRC report policy digest must match both the manifest-scored run report and the policy bytes used by the verifier.

This avoids a dangerous two-command workflow in which a user could run a normalized-only comparison and forget the separate raw-artifact verifier. It still does not make any physical or manufacturing claim; it only ensures the benchmark score is bound to the actual source and result bytes and pinned KiCad evidence.

## Competitive bar updated from public results (2026-10-22)

A benchmark target must now account for public vendor-reported results published after the PCBWorld paper:

- ProtoFlow's benchmark page reports **99/99 D3-A small boards clean** and **30/30 D3-B attempts clean** with three attempts per each of 10 medium boards under a 30-minute per-attempt limit. It reports **0.929 D3-A / 0.933 D3-B at three minutes**, and router-only results of 0.919 / 0.900. See [ProtoFlow's results](https://www.protoflow.ai/benchmarks).
- DeepPCB's 28 September 2026 case study reports **92/99 D3-A and 24/30 D3-B attempts clean** on its standard model, with 95% intervals of [0.861, 0.965] and [0.627, 0.905], respectively; the small D3-B sample is too small to separate its reported 0.800 from FreeRouting's 0.64 confidently. See [DeepPCB's results and their comparison table](https://deeppcb.ai/benchmark/pcbworld-bench-deeppcb-results/).
- The paper's reproduced one-attempt baseline table, as transcribed by DeepPCB, lists D3-A 0.81 for PPO without the finish stage and 0.75 for FreeRouting; D3-B 0.22 and 0.64, respectively. Compare only after the same official evaluator, board revision, engine, scoring flags and time budget are pinned. The vendor pages are self-published run reports, not independently audited evidence from this project.

### What “beat SOTA” should mean now

The ceiling on a clean-pass rate is 1.0, so the stretch goal cannot be merely “more clean boards than 99/99.” It must be **same-validity performance at a tighter time budget, better board-level generalization, or lower copper/via cost without sacrificing clean completion**. Proposed measured gates for a fair comparison:

1. **3-minute D3-A**: match or exceed 99/99 clean attempts (1.000) on the exact same D3-A board bytes, with independently rerun KiCad DRC; then pursue a lower median/p90 time-to-first-clean completion and/or less wirelength/vias on paired clean boards.
2. **3-minute D3-B**: target at least 28/30 clean attempts (0.933) across all three attempts per each of the 10 boards, with board-clustered intervals and per-board disclosure. Then target 30/30 without raising the time ceiling.
3. **Quality frontier**: at equal clean-pass rate and identical board set, seek at least 5% lower median wirelength or 10% fewer vias without a material regression in the other metric; report the full Pareto front, not a scalar reward.
4. **Robustness**: report all 99/30 attempts, every timeout and failure; compare one-attempt and bounded multi-attempt modes separately. Never score “best of N” as if it were one attempt.
5. **Generalization**: keep source-board-family held-out data separate from the official D3 evaluation. Upstream's published train/test key mirroring is for its evaluation-only D3 flow and is not itself a held-out train/test partition for new policy tuning.

The candidate 30-board pilot in this repository remains **unauthorized for execution** until each original source project's license and exact board bytes are ingested, pinned and reviewed. Metadata says are not enough: the source audit currently marks zero of the 30 candidates benchmark-eligible. This is an intentional gate, not a reason to silently swap in unlicensed boards.


## Fixed-budget score curve semantics

The Rust PCBWorld CSV importer now emits an explicit scorecard for 30 s, 60 s, 180 s, 300 s, and 1800 s. A row receives credit at a budget only when the source's `clean_pass` is true **and** its recorded `per_board_rollout_time` is known and at or below the cap. Missing runtime remains in the all-attempt denominator and never earns time-bounded credit; a clean label with no runtime is reported as an untimed clean row. Both the all-attempt rate and a secondary timed-row diagnostic are retained.

This scorecard measures **final output completed by a recorded runtime**, not the earliest moment a route happened to become clean during an ongoing run. Reproducing a time-snapshot leaderboard such as “clean at 3 minutes” requires each method to be run under the same 3-minute cutoff (or a complete timestamped checkpoint trace); it is invalid to infer a 3-minute pass from a clean final board that took 30 minutes. The importer preserves the upstream `clean_pass` definition, which is not equivalent to the stricter raw-KiCad evidence gate in `compare-verified`.


## Anytime verified-incumbent implementation

Issue [PCB-ANYTIME-001 #2608](https://github.com/Luminous-Dynamics/symtropy/issues/2608) and the standalone API in tools/pcb-route-benchmark/src/anytime.rs now define a proof-carrying checkpoint archive:

- A candidate is evaluated from the exact native board bytes and raw KiCad DRC JSON under a pinned policy. The evaluator itself computes board/report hashes; the caller's success flag is not accepted.
- The candidate's source-board SHA-256, manifest-byte SHA-256, required-net count, board file name, KiCad version and DRC-policy digest must match the pinned manifest/policy.
- The preferred root-based API reads the named files below a canonical artifact root and rejects traversal, absolute paths, symlink escapes, missing/non-regular files, aliases and a DRC source name that does not match the output board filename.
- Only route-valid candidates enter the eligible incumbent set. Invalid/partial/late candidates stay in the audit history but cannot erase an earlier clean checkpoint.
- For each requested time budget, selection considers only eligible snapshots whose recorded elapsed time is at or below that budget; it emits a deterministic Pareto front across wirelength/vias and a selected member.
- Archive reuse is not trust-on-JSON: the record and select CLI commands re-open each board/DRC file, recompute DRC interpretation and hashes, and compare them with the stored receipt. An edited eligibility flag or changed artifact bytes must fail re-verification under the current exact evaluator head.

### CLI

From the isolated Rust crate directory/build invocation, the commands are:

- init-incumbent-archive MANIFEST.json POLICY.json BOARD_ID
- verify-checkpoint CHECKPOINT.json MANIFEST.json ARTIFACT_ROOT POLICY.json
- record-incumbent ARCHIVE.json CHECKPOINT.json MANIFEST.json ARTIFACT_ROOT POLICY.json
- select-incumbent ARCHIVE.json MANIFEST.json ARTIFACT_ROOT POLICY.json BUDGET_MS

A checkpoint metadata file supplies the checkpoint ID, board ID and source digest, manifest digest, required/connected net counts, dangling-item counts, elapsed milliseconds, wirelength, via count, relative output-board path, relative raw-DRC path, output filename and KiCad version. Run the command with PCB_ROUTE_BENCHMARK_GIT_HEAD set to the exact evaluator commit. Archive selection requires the original manifest, exact policy and retained raw artifacts so saved incumbents are revalidated instead of trusted.

The tests include a later crossing attempt that cannot displace an earlier verified clean snapshot, the Pareto trade-off between wirelength and vias, missing-net rejection, path traversal rejection, and mutation of both the serialized eligibility field and underlying saved-board bytes. These are software contract tests only; they are not a PCBWorld router result or a physical qualification.
