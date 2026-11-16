# Kitchen Workcell Benchmark V0.1

Status: draft / not run. Parent execution issue: https://github.com/Luminous-Dynamics/symtropy/issues/5618.

- `kitchen-workcell-benchmark.schema.json`: structural contract for comparator/task/metric/run manifests.
- `manifest.json`: initial comparator and task basket. All comparators are candidates, not acquired or measured. `runs` is intentionally empty.

## Use

1. Validate `manifest.json` against the local schema with a Draft 2020-12 validator.
2. Freeze the protocol, exact SKU/revision, task input, output assays, meter calibration and randomization before trials.
3. Append a run record only after retaining raw instrument records and input digests.
4. Keep failed, aborted, incomplete and unknown runs in the manifest.
5. Never change `evidence_class` from simulated/modelled to measured because a prediction matched an expected value.

## Comparator sourcing note

Consumer Reports' 2026 public test descriptions are a task-basket source, not a standard or proof of local model availability:
- Multi-cookers: https://www.consumerreports.org/appliances/multi-cookers/best-multi-cookers-of-the-year-a7136971834/
- Food processors: https://www.consumerreports.org/appliances/food-processors-choppers/best-food-processors-of-the-year-a7878155198/

The Breville Paradice 9 has a regional capacity discrepancy in public technical material (Consumer Reports describes a seven-cup model; an Australian technical sheet lists nine cups). Fix the market-specific SKU, capacity, supply voltage and accessory kit before comparison. A comparator name alone is not an adequate test identity.

## Hard boundaries

- Commercial, certified appliances only during first metrology phase.
- No pressure-vessel modification, interlock bypass or unsafe blocked-vent trial.
- Pressure, thermal cut-outs and blade guarding remain local deterministic safety functions.
- Food safety, microbiological lethality and conformity are external qualification gates.
- The current manifest contains hypotheses only; no physical results or SOTA claims.
