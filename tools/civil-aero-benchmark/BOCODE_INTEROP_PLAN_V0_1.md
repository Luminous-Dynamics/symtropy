# BOCoDe interoperability and engineering optimizer-transfer protocol v0.1

**Status:** metadata importer and benchmark protocol under review. No BOCoDe optimizer has yet been run by Symtropy, and no optimizer superiority claim is made.  
**Parent:** AERO-MDO-INTEROP-001 (#3237).  
**Implementation:** AERO-BOCODE-INTEROP-001 (#3525), draft PR.  
**External benchmark:** https://github.com/rosenyu304/BOCoDe  
**Paper:** https://arxiv.org/abs/2608.15073  
**API documentation:** https://bocode.readthedocs.io/en/latest/api.html

## Why this matters

BOCoDe addresses a real weakness in optimizer evaluation: synthetic and HPO benchmarks do not adequately represent constrained, multi-objective, high-dimensional or mixed-variable engineering design. The 2026 paper reports 307 problems: 159 engineering, 80 HPO and 68 synthetic. It compares 31 methods: 27 Bayesian-optimization algorithms and four evolutionary algorithms.

The published baseline protocol uses:
- a type-aware shared Latin-hypercube initial design of size D;
- 250 additional objective evaluations per method/problem;
- 25 independent seeds;
- best-so-far objective normalized per problem for single-objective tasks;
- fixed-reference hypervolume for multi-objective tasks;
- feasible best objective/HV and a separate feasibility rate for constrained tasks;
- median best-so-far across seeds, then per-problem ranking; Friedman/Iman-Davenport/Nemenyi statistical analysis, with Holm-corrected Wilcoxon checks for classes with fewer than 20 problems.

These are the **official reproduction targets**. Do not silently replace 25 seeds with three, reduce the 250-call budget, or change the objective-class metric and then label the result a BOCoDe reproduction.

The upstream API maximizes objectives; constraints are feasible when `g(x) <= 0`. Any conversion to Symtropy's minimization convention must occur in an explicit downstream adapter and retain original values and sign.

## Important upstream version and taxonomy caveats

The current public docs inspected on 4 November 2026 identify BOCoDe 1.0.2.dev2. The repository's README says it is still under construction. Therefore pin the exact Git commit, installed package version, environment lock, optional extras and metadata catalog used by every run.

The paper and upstream README define five comparison strata:
1. SO-Unconstrained
2. SO-Constrained
3. MO-Unconstrained
4. MO-Constrained
5. SO-Mixed-Variable

These are **not** the Cartesian product of objective count, constraints and variable type. The exact `input_type` (continuous / discrete / mixed) is a separate metadata axis. Discrete-only tasks remain in the appropriate SO/MO unconstrained/constrained stratum while retaining `variable_type_class=discrete`. The single-objective mixed-variable stratum includes both constrained and unconstrained tasks: current `PressureVessel` has 1 objective, 4 constraints and `input_type=mixed`; `Car` has 1 objective, 10 constraints and `input_type=mixed`. Multi-objective mixed-variable records, if present, are retained as an explicit extension rather than forced into one of the five official strata.

Additional metadata caveats:
- `Truss10D`: source citation is empty in its current metadata.
- `Mazda`: 222 dimensions and 5 objectives, but `bounds=null` and no `variable_types` array.
- Discrete examples include `HOIP`, `GearTrain` and `MaxSAT`.

The importer preserves the five official strata and records exact variable type independently. It also records missing source citations, unavailable bounds and unavailable variable-type arrays as explicit states. Missing metadata is not replaced by inferred defaults. A task whose source is missing is not ready for a sourced benchmark claim until the source is separately admitted.

The upstream API's `list_metadata()` returns registered metadata resources. Synthetic functions from `list_synthetic()` are exposed separately and are not automatically included in this metadata catalog. Consequently, this importer does **not** claim to have imported all 307 paper tasks. The raw metadata catalog scope and task count are recorded in the export manifest. A future synthetic-function adapter needs its own explicit schema.

## Rust-first architecture

The external exporter is:
`scripts/export-bocode-catalog.py`

It runs inside an independently pinned BOCoDe environment, calls `bocode.list_metadata()`, and reads the exact package resource bytes under `bocode.opt_problems_metadata/*.json`. It does not instantiate or evaluate any task and does not download optional datasets.

It emits:
- normalized one-row-per-task CSV;
- a JSON context manifest containing the upstream commit, installed package version, exact raw metadata catalog digest, normalized CSV digest, membership-list digest and counts of missing/unavailable metadata.

The Rust importer is:
`src/bocode_catalog.rs`

It verifies:
- exact 40-character source commit and catalog SHA-256;
- exact normalized CSV schema;
- sorted and unique task identifiers;
- positive dimensions/objective counts and non-negative constraint counts;
- the objective/constraint/variable-type class tuple;
- explicit benchmark membership;
- SHA-256 digests for exact raw metadata, canonical bounds and canonical variable types;
- explicit source citation, bounds and variable-types availability states;
- explicit optional dependency name or `none`.

The independent stdlib verifier is:
`scripts/verify-bocode-catalog-independent.py`

It replays the normalized schema, taxonomy, canonical import digest and metadata-state checks independently. Catalog import proves structural consistency only. It does not authenticate the BOCoDe release, establish that optional dependencies are installed, clear data terms, prove executability, or validate scientific/physical accuracy.

### Export example

Use the exact upstream commit and an explicitly reviewed benchmark-set membership file. Without that file all rows are retained but `benchmark_set_member=false`; membership is never inferred from task names.

```sh
python scripts/export-bocode-catalog.py \
  --source-commit <40-character-upstream-commit> \
  --benchmark-set-ids bocode-benchmark-set-ids.txt \
  --output bocode-catalog.csv \
  --manifest bocode-catalog-manifest.json
```

The CSV can then be passed to the Rust catalog importer with `BoCoDeCatalogContext` populated from the manifest. Keep the raw JSON metadata resources and the manifest beside the normalized CSV in the retained benchmark bundle.

## Sentinel intake proposal

Freeze the exact IDs from the pinned export and the paper's task list before running anything. The following are candidate examples, not claims of already-admitted benchmark membership:

| Family | Candidate | Why it matters | Intake caution |
|---|---|---|---|
| Structural sizing | `ThreeTruss`, `Truss10D` | engineering constraints and structural design | `Truss10D` currently has no source citation and requires the truss extra |
| Mechanical design | `PressureVessel` | mixed-variable constraints | extension class, not an assumed unconstrained mixed-variable task |
| Vehicle design | `Car`, `CarSideImpact` | mixed variables and several constraints/objectives | distinguish objective counts and exact variable-type metadata |
| Multi-objective design | `RE21`, `CRE21` | constrained/unconstrained engineering multi-objective comparison | keep true constraint state and maximization sign convention |
| Aerodynamic design | `MiniAeroWing`, `Wing` | closest connection to aero design | not a CFD benchmark; model class and reference source remain separate |
| High-dimensional system design | `Mazda` | high-dimensional, multi-objective task | bounds and variable types unavailable in current metadata; optional extra and external data need separate review |

Do not freeze the sentinel list until exact task membership, metadata, sources, optional extras, data dependencies and benchmark license terms have been reviewed. The task set should span at least five independent engineering families and include a mix of the supported problem classes. Include the BOCoDe HPO and synthetic tracks only as separate controls; do not use them as engineering-domain proxies.

## Two comparison tracks

### Track A — exact BOCoDe reproduction

Run the applicable upstream algorithms on their supported classes with shared initial designs, D + 250 objective calls, 25 seeds and the official per-class metric. Use the exact 31 methods where applicable; record each unavailable method/task pair as an explicit compatibility failure rather than silently excluding it.

Produce the paper-comparable complete-case ranking and disclose:
- intended task/method matrix size;
- complete-case task count per class/domain;
- failed, unavailable and invalid task/method combinations;
- the reason each task was excluded from the complete-case statistical panel.

### Track B — Symtropy full-engineering-cost extension

Keep the official objective-call score intact, then report a distinct full-cost view that charges:
- package startup, dependency installation and task initialization;
- objective calls, failed/rejected/timed-out calls and retries;
- surrogate training, acquisition optimization and hyperparameter tuning;
- preprocessing, data loading and cache construction;
- CPU/GPU time, wall time, peak memory and measured energy;
- orchestration, agent tokens/tool calls and human intervention where applicable;
- post-processing and independent verification.

Use BOCoDe's applicable GP-UCB/GP-LogEI, TuRBO, BAxUS, CEI/SCBO, qNEHVI/qNParEGO, NSGA-II/SPEA2, PR/Bounce/Casmopolitan/HEBO/BODi baselines where the exact task class is supported. Add random and type-aware LHS controls. Add a Symthaea/Symtropy proposal policy as a separate participant with an immutable version, not as an unpinned hidden decision layer.

For every task report:
- best feasible objective or fixed-reference HV;
- feasibility attainment rate across all seeds;
- time-to-first-feasible and evaluations-to-first-feasible;
- incumbent quality versus objective calls and versus total cost;
- CPU/GPU/wall/memory/energy (unknown stays unknown);
- failure and abstention coverage;
- paired family-level deltas and uncertainty.

Report BOCoDe-compatible complete-case rankings **and** the all-attempt coverage table. Complete-case ranking is required for paper comparison, but cannot be the only system-quality report: a method that fails on difficult problems should not get its failure hidden by a shrinking denominator.

Use the existing family-clustered aggregation layer for paired across-family analysis. Do not treat hundreds of iterates or multiple seeds from one task as hundreds of independent engineering domains.

## Performance and superiority claims

The first target is reproduction, not superiority. The next target is better feasible-decision quality per total engineering cost on held-out engineering families, with no worsening of feasibility failure or evidence coverage.

Useful candidate metrics:
- best feasible objective and normalized best-so-far;
- normalized HV/IGD+ where a defensible reference front exists;
- feasible attainment rate and time-to-first-feasible;
- regret when a true/reference optimum or trusted bound exists;
- anytime area-under-the-best-so-far curve;
- cost-quality curves and cost-to-target;
- family-clustered paired effect sizes and confidence intervals;
- coverage-adjusted failure rate and worst-family performance.

Do not use mean ranks alone as the system metric. Rank methods within task/class for BOCoDe comparability, but retain effect sizes, raw outcomes, uncertainty, all-attempt coverage and absolute costs. BOCoDe itself notes that algorithm rankings vary by domain and that feasibility can deteriorate on engineering problems; this is why engineering-domain controls matter.

A Symtropy superiority claim requires at least five independent engineering families, 25 seeds for exact BOCoDe reproduction, equal tuning opportunity, matched call budgets, full-cost accounting for the extension, preregistered primary metrics, and independent replay. If resources cannot support 25 seeds for a new extension, label it exploratory and do not present it as exact baseline reproduction.

## External reference summary

- BOCoDe paper: https://arxiv.org/abs/2608.15073
- BOCoDe GitHub: https://github.com/rosenyu304/BOCoDe
- BOCoDe API: https://bocode.readthedocs.io/en/latest/api.html
- BOCoDe metadata source function: https://bocode.readthedocs.io/en/latest/_modules/bocode/registry.html
- OpenMDAO current docs: https://openmdao.org/docs/
- ShapeBench paper: https://arxiv.org/abs/2605.20763
- AIAA DPW-8: https://www.aiaa-dpw.org/TestCases/testcases.html
- NASA Turbulence Modeling Resource: https://tmbwg.github.io/turbmodels/
