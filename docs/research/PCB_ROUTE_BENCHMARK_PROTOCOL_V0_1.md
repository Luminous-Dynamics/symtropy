# Engine-grounded PCB routing benchmark protocol v0.1

**Status:** evaluator contract and synthetic fixtures only. No PCBWorld dataset has been downloaded, no external router run has been executed, and no state-of-the-art comparison is claimed.

## Scope: copper routing, not manufacturing route economics

This benchmark evaluates the geometric task of routing PCB copper traces: connect all required nets while respecting a pinned engine's routing DRC. It does not compare local versus outsourced fabrication or assembly, cost per accepted assembly, board milling, or workcell dispatch. Those are separate Symtropy programs (#2385 and #2397).

## Reference benchmark and provenance

The proposed external reference is [PCBWorld](https://github.com/LGAI-Research/PCBWorld) and [PCBWorld-Bench (arXiv:2607.05915)](https://arxiv.org/abs/2607.05915), published July 2026 for the KDD 2026 Workshop. The paper describes grid-based and gridless synthetic tasks and 679 real open-source boards, with methods checked by native KiCad routing and DRC. It compares rule-based routers, reinforcement-learning policies and LLM agents; it also reports that large-board routing remains difficult and learned agents may trail strong rule-based methods on hard real boards.

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

The current v1.1.1 README lists FreeRouting and KRT as rule-based baselines; the exact method availability is versioned, so the protocol must enumerate the methods actually present in the pinned checkout rather than carry forward a stale router list from an earlier release. The upstream evaluator documents a staged process (rollout, post-hoc native DRC, aggregate) and metrics such as connectivity success, routability, track/via counts, wirelength, DRC violation classes and upstream board metadata. The Rust evaluator's normalized input must be produced by an adapter that faithfully maps these metric semantics and carries raw DRC reports and output-board digests; a summary CSV or router-reported success value alone is not evidence enough.

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

Build the manifest from the exact upstream `d3.json`, intake-spec bytes, and local processed D3 root:

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

The builder proves each selected ID belongs to the explicitly named difficulty/split array in the supplied D3 configuration, derives the relative board path from D3's `board_filename`, hashes the board bytes it actually reads, and binds hashes of the raw split-config and intake-spec bytes plus the source Git commit into the resulting manifest. It rejects absent boards, unknown license placeholders, non-HTTPS provenance, unsafe paths, duplicate IDs and IDs outside the pinned split. Keep `d3.json`, PCBWorld commit, engine submodule, intake spec, manifest and intake receipt as the experiment's immutable inputs.

**Do not treat the two identical D3 train/test lists as a train/test holdout.** An explicitly selected 30-board pilot is a pipeline qualification subset; freeze the selection before routing methods are compared, then run the entire declared split for the final score. A future source-family holdout should be a separate disclosed split, not inferred from D3's mirrored keys.


### Frozen candidate pilot: 30 boards across all three D3 difficulty tiers

The initial candidate list is checked into `tools/pcb-route-benchmark/fixtures/pcbworld-d3-pilot-candidates-v0_1.json`. It records the upstream PCBWorld main commit `1989a9325796b73cca793a0c8f222dd1509256e8` and the D3 split JSON's Git blob ID `61d86bbf6cfd8ff8d1e13089c8d98a90f6f81bb3`, both retrieved from the upstream GitHub API on 2026-10-21. The 30 candidates are stratified as 10 easy boards sampled at deterministic evenly spaced indices from the 99-board easy test list, all 10 entries in the upstream medium test list, and all 10 entries in the upstream hard test list.

This file is intentionally a **candidate set**, not an executable intake manifest. Every entry states that its original source license, source URL and required-net count remain unverified/unmeasured. The builder must reject placeholder license entries, and the benchmark must not start on these boards until original project provenance/licensing and actual local board hashes are filled into a reviewed intake spec. Required-net counts must be measured using the pinned EDA/evaluation semantics, not guessed from the board name. D3's medium/hard “test” lists are only ten boards each; this 30-board pilot therefore spans the published difficulty labels but is not a statistically representative sample of the full 677-board D3 pool.

Git blob SHA-1 metadata identifies the upstream JSON object; the manifest builder separately calculates SHA-256 over the exact raw JSON bytes for cryptographic receipt binding. Do not substitute one for the other.
