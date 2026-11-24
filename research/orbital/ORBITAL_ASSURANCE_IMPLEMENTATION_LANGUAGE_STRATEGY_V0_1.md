# Orbital Assurance Implementation Language Strategy v0.1

**Status:** engineering policy and benchmark guidance.  
**Applies to:** Symtropy's incumbent-decision, assurance-composition and SOTA-reproduction work.  
**Production claim ceiling:** Rust remains the canonical executable implementation unless a specific integration requires another runtime.

## 1. Decision

Use a deliberately polyglot verification arrangement:

- **Rust:** production decision logic, canonical state machines, receipt generation, CLI tools, deterministic replay and performance-critical evaluators.
- **JavaScript / Node:** retain the existing independent challenger implementation where the existing repository already uses it; use BigInt for exact integer comparisons, never Number for exact loss/alpha arithmetic.
- **Python:** add a separate standard-library-only reference oracle for exact rational semantics and use Python for research reproduction, statistical analysis and comparisons where the published method's ecosystem requires it.
- **JSON and explicit canonicalization:** interchange format only. Every digest must specify UTF-8, field set, array order, numeric bounds and serialization rules.

This is not a decision to move the production system to Python or to rewrite the existing JavaScript harness.

## 2. Why Python is valuable here

Python provides:
- unbounded-precision integers;
- fractions.Fraction for normalized exact rational arithmetic;
- hashlib, json, datetime and unittest in the standard library;
- a large scientific, statistical and optimization ecosystem used by published decision-focused learning work.

The current bridge uses exact rational campaign-alpha checks. Python's Fraction gives an independently expressed arithmetic path, while the Node oracle currently uses BigInt cross-multiplication. This is useful diversity: it can uncover accidental reliance on JavaScript number coercion, Python's bool-is-an-int behavior, JSON canonicalization mismatches, timezone parsing differences and malformed-value crashes.

Python is not automatically more correct. A second implementation written by the same developer from the same mistaken specification can reproduce the same conceptual bug. Independence requires separately authored code, frozen known-answer cases, mutation tests, and no sharing of the candidate's internal logic.

The new tests/anytime_world_set_bridge_oracle.py is intentionally standard-library-only. It uses Fraction for budget relations and replays the frozen negative controls. It is a research/test oracle, not a production service.

## 3. Why JavaScript should not be removed yet

The JavaScript oracle already provides a separate runtime and exact BigInt path. Replacing it immediately with Python would reduce the number of independent implementations and could make the comparison less adversarial.

Keep both challengers until:
1. the same frozen case IDs and expected outcomes run in Rust, Node and Python;
2. canonical receipts are compared field-by-field, not merely by a shared pass flag;
3. generated rational boundary cases and malformed JSON shapes are tested;
4. any mismatch is minimized to a reproducible counterexample;
5. the maintenance burden is justified by measured defect detection, not language count.

If the third implementation creates more duplicated semantics than assurance value, keep Python for benchmark reproduction and offline analysis, and consolidate the receipt challenger after a documented decision.

## 4. Production and security boundary

Do not introduce Python into the production decision or authority path merely to gain convenient libraries. Production inputs must remain typed, bounded, source-pinned and fail-closed in the Rust evaluator. No Python/JavaScript oracle may issue authority, amend source evidence, or choose/execute a replacement action.

Python scripts used in CI must:
- run with an explicitly declared interpreter version;
- use the standard library for contract oracles wherever practical;
- avoid implicit float conversion in exact comparisons;
- reject bool values where integer fields are required;
- enforce canonical serialization and numeric bounds;
- preserve an exact exit status and machine-readable report;
- never silently treat dependency installation, skipped tests or queued workflows as success.

Published research baselines may require external packages, GPUs, solver licenses or proprietary optimizers. Run those in a separate reproducibility environment with locked dependencies, documented solver versions/license status, immutable data splits and raw results. Do not add those packages to the production crate or the stdlib receipt oracle.

## 5. Benchmark implementation-language choices

| Workload | Preferred tool | Reason |
|---|---|---|
| Production incumbent gate / evidence graph / receipts | Rust | Typed contracts, deterministic execution, deployment control |
| Independent challenger for integer receipt logic | Node BigInt and Python Fraction | Distinct runtime and arithmetic implementation |
| Exact rational test generation / known-answer reference | Python Fraction + unbounded int | Transparent rational arithmetic and compact research tooling |
| Statistical baseline reproduction | Python where upstream uses it | Avoid changing the published method's implementation language |
| Heavy optimization experiments | Pinned Python environment plus declared solver | Match PEAR / robust DFL methods and solver assumptions |
| Large-scale production replay | Rust | Reproducible operational artifact and performance measurement |

## 6. Reproduction comparators and environment caveats

The official PEAR implementation for Decision-Focused Learning via Tangent-Space Projection of Prediction Error is Python-based and documents a Gurobi license requirement. The UAI 2026 robust-DFL implementation is also Python-based; its README specifies Python 3.10+ and Poetry, and says its optimization oracles require Gurobi. This makes Python the practical reproduction language for those baselines, but it does not imply the production decision kernel should be Python.

For open, independently reproducible comparisons, add a solver-neutral LP/QP slice with HiGHS or another declared open solver, and keep a separately labelled author-faithful run when licensed Gurobi is available. Never silently substitute solvers and claim exact paper reproduction.

References:
- Python fractions.Fraction: https://docs.python.org/3/library/fractions.html
- Python integer precision: https://docs.python.org/3/library/stdtypes.html#numeric-types-int-float-complex
- JavaScript BigInt: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Data_structures#bigint_type
- PEAR official code: https://github.com/FinJun/PEAR
- UAI 2026 robust-DFL official code: https://github.com/isct-nakatalab/UAI2026_Robust_Decision-Focused_Learning_via_Worst-Case_Regret_Minimization

## 7. Promotion gate

Do not remove one of the independent challenger implementations until a recorded ablation demonstrates that it adds no useful defect-detection coverage, no additional numeric edge-case coverage, and no reproducibility value. Keep the Rust production implementation and its claim ceiling independent of all challengers.
