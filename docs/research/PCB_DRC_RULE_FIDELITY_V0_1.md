# PCB Design-Rule Fidelity Contract v0.1

**Status: synthetic contract tests only.** This module has not yet been run against real PCBWorld board bytes or real source KiCad .kicad_pro files. A passing synthetic test is not a claim that the PCBWorld rule reconstruction is source-equivalent.

## Problem

A router may pass a compatibility DRC after missing rules have been inferred, rules relaxed, warnings suppressed, or settings altered. The resulting score can look better while the task became easier. Route completion and constraint fidelity are independent questions:

1. Does the routed board pass the explicitly pinned compatibility DRC?
2. Were the constraints used for that claim preserved from the source design, or was each migration/exception separately documented and verified?
3. Does the same output-board byte sequence pass the declared strict-source DRC?

Never use a positive answer to (1) as a substitute for answers to (2) and (3).

## Rule-by-rule contract

Every rule listed in a RuleFidelityPlan has one disposition:

- **source_preserved**: a JSON pointer into the byte-pinned source KiCad .kicad_pro file resolves to the declared source scalar; the evaluated scalar equals it.
- **version_migrated_equivalent**: a JSON pointer resolves to the claimed source scalar, while a separate mapping-evidence file is required and its actual bytes must match SHA-256. This records a reviewed equivalence claim; it is not a mathematical proof of equivalence.
- **explicitly_waived**: the source value must be source-pointer-backed and a pinned waiver-evidence file must exist. A waiver prevents the overall rule set from being labelled strictly source-authoritative.
- **inferred**, **relaxed**, **unavailable**: the disposition is preserved in the receipt and prevents strict-source clean-pass credit.

The source profile, input board and routed output board bytes are separately digest-checked. Every compatibility/strict DRC run declares the exact output-board digest it evaluated; that digest must equal the actual output bytes supplied to the evaluator. Policy bytes and KiCad versions are pinned too. The policy must block errors and warnings, unconnected items, and schematic-parity violations. DRC JSON must contain both violations and unconnected_items; unknown report shapes are rejected.

Mapping, waiver and strict-policy derivation evidence are loaded under an explicit evidence root. Absolute paths, traversal, symlink escape, path aliases, empty evidence, unreferenced evidence and digest mismatch are rejected. The receipt lists the exact evidence paths, byte lengths and SHA-256 values.

## CLI

After building the isolated crate, set the evaluator commit and supply paths to the plan, source board, output board, source rules (or - if absent), compatibility policy and raw DRC report, optional strict policy/report (or -), and the root containing cited evidence files:

    export PCB_ROUTE_BENCHMARK_GIT_HEAD="$(git rev-parse HEAD)"
    cargo run --manifest-path tools/pcb-route-benchmark/Cargo.toml -- \
      rule-fidelity PLAN.json SOURCE.kicad_pcb ROUTED.kicad_pcb SOURCE.kicad_pro \
      COMPAT_POLICY.json COMPAT_DRC.json STRICT_POLICY.json STRICT_DRC.json EVIDENCE_ROOT

When source rules or strict-source analysis are unavailable, supply - for the corresponding optional inputs and ensure the plan marks the associated paths/digests absent. This yields an unresolved compatibility-only result, never an implicit strict pass.

## Benchmark outputs

Keep compatibility and strict-source results in separate columns and retain every board in the denominator. Required aggregate outputs for a real pilot are:

- compatibility-policy clean-pass rate;
- strict-source-authoritative clean-pass rate;
- fraction of boards with inferred/relaxed/waived/unavailable rules;
- rule-classification matrix and policy-difference digest per board;
- board-level cases that pass compatibility DRC but fail strict-source DRC;
- route completion, wirelength, vias, runtime and failures, paired only under identical input/policy/engine pins.

For a route to count as improved, validity must not decline and constraint fidelity must remain equal or improve. Do not turn unknown source constraints into guessed numerical defaults.

## Limitations / next gate

The module mechanically checks pinned JSON paths, board bytes, DRC report shape, evidence-file digests and policy identity. Migration equivalence and waiver legitimacy remain evidence-backed engineering judgements, not decisions the evaluator can infer from a digest alone. The integration adapter must capture the real KiCad invocation, binary/container digest, exact input board, raw report, exit code and version. Next, run it against a deliberately small set of source-authorized board projects where exact source project/rule bytes are available, independently review each rule mapping, and include a negative control whose compatibility policy passes while its strict-source policy fails.

Neither compatibility nor strict-source DRC establishes signal/power integrity, EMC, fabrication conformity, physical board quality, electrical safety or production-release readiness.
