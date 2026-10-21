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
