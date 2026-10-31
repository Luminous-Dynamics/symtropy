# Algorithm-level optimizer qualification protocol v0.1

## Status and evidence ceiling

This lane executes three actual optimization policies on analytic multi-objective functions with known Pareto fronts, including both connected and disconnected front geometries. It fills a gap left by a result-only scorecard: deterministic mock scorecards are useful for testing accounting, but they do not test whether an optimizer actually searches well.

The lane is intentionally separate from aerodynamic CFD. Its objective evaluations are cheap analytic function calls. Passing it proves implementation and algorithm-qualification behaviour on ZDT1/ZDT2/ZDT3 only. It does not demonstrate airfoil performance, ShapeBench performance, CFD evaluation efficiency, or superiority to state-of-the-art aerospace optimization methods.

## Tasks and known answers

- ZDT1, 30 decision variables in [0,1], objectives: f1=x1; g=1+9*mean(x2..xn); f2=g*(1-sqrt(f1/g)). Its Pareto front has g=1 and f2=1-sqrt(f1).
- ZDT2 uses the same decision domain and g, with f2=g*(1-(f1/g)^2). Its Pareto front is g=1 and f2=1-f1^2.
- ZDT3 uses the same decision domain and g, with f2=g*(1-sqrt(f1/g)-(f1/g)*sin(10*pi*f1)). Its non-dominated front has five disconnected x intervals: [0,0.0830015349], [0.1822287280,0.2577623634], [0.4093136748,0.4538821041], [0.6183967944,0.6525117038], [0.8233317983,0.8518328654].

All three have analytically described two-objective Pareto fronts. ZDT3 is disconnected into five separated trade-off segments, testing front-preservation and coverage. The report uses reference point (1.1,1.1), the exact continuous-front hypervolume, and a 201-point deterministic reference set for IGD. For ZDT3, the hypervolume is integrated piecewise over the five front intervals with constant-height rectangles across gaps; its IGD reference set allocates 41/40/40/40/40 points across those intervals. Hypervolume is normalized by the exact continuous-front value, so a result must lie in [0,1] up to floating-point tolerance.

## Policies

1. uniform_random: independent uniform samples across the full decision cube. Every evaluated vector is retained in the evaluator trace.
2. nsga2: a compact generational NSGA-II-style implementation with nondominated sorting, crowding-distance selection, binary tournament, simulated binary crossover (SBX), and polynomial mutation. The final partial generation is clipped to the remaining evaluation budget.
3. gde3: generalized differential evolution 3 using DE/rand/1 mutation, binomial crossover, parent/trial dominance comparison and nondominated/crowding truncation for mutually nondominated parent/trial pairs.

All policies share the same seeded initial population for each task/replicate (common random numbers); their random streams diverge only after the initial design as their search operators consume randomness. All are assigned identical tasks, seed list, and objective-evaluation checkpoints. They use the same decision dimension and total objective-call budget per seed/task. Failed or invalid evaluations must not be silently counted as successful results; the current deterministic analytic suite fails the run if any objective evaluation is non-finite or out of domain.

## Metrics and fairness

At each checkpoint, the qualification runner rebuilds the nondominated archive from all evaluations observed so far for both methods, so neither method is scored on a smaller history window. It reports exact objective evaluations consumed, nondominated archive size, 2D hypervolume and fraction of the continuous Pareto-front hypervolume, inverted generational distance (IGD) to a fixed 201-point front, and mean g-value as a diagnostic of convergence toward the Pareto set.

The full trace contains the decision vector and objective values for every evaluation. The independent Python oracle recomputes every ZDT objective from the vector, then recomputes the archive, hypervolume, normalized fraction, IGD and mean g for all checkpoints.

The qualification uses 30 variables, population 40, 1,000 evaluations per policy/task/seed, checkpoints 100/250/500/1,000 and five seeds. Across ZDT1, ZDT2 and disconnected-front ZDT3, this produces 45 runs, 45,000 objective evaluations, 180 summary rows and 45,000 candidate trace rows.

## Benchmark targets

These are targets for later ShapeBench and CFD-connected work, not claims from the present analytic suite:
- independently replay quality-versus-evaluation curves across at least 8 distinct task families;
- compare random search, NSGA-II, differential evolution, CMA-ES/BO where appropriate, and ShapeBench task-applicable baselines under the same total budget;
- include multiple seeds and a blind family holdout;
- report family-level medians, worst-decile performance and paired uncertainty intervals, not only pooled means;
- charge solver setup, mesh generation, failed evaluations, surrogate training, acquisition computations, orchestration and audits to total cost;
- report validated hypervolume per high-fidelity condition call, false-feasible rate, high-fidelity prediction error and uncertainty calibration;
- target at least 20% fewer high-fidelity condition calls at equal validated Pareto quality, with no degradation in worst-family feasibility or uncertainty calibration. This is a pre-registered stretch target, not an achieved result.

## What would count as beating the field

A method should be declared superior only if it improves a predeclared validated design objective or Pareto hypervolume under matched total cost, repeats across independent seeds and multiple families/environments, preserves or improves feasibility/uncertainty calibration/OOD abstention, survives blind family/condition holdout, and has reproducible geometry/mesh/solver/configuration/run-receipt/independent-audit artifacts.

The ZDT lane is only a deterministic algorithm smoke/qualification suite. ShapeBench remains the broad external optimization comparison. DPW8 remains the physics verification/validation programme. They must not be collapsed into one score.

## References

- Deb et al., A Fast and Elitist Multiobjective Genetic Algorithm: NSGA-II, IEEE Transactions on Evolutionary Computation, 2002, DOI: 10.1109/4235.996017.
- Zitzler, Deb and Thiele, Comparison of Multiobjective Evolutionary Algorithms: Empirical Results, Evolutionary Computation, 2000, DOI: 10.1162/106365600568202.
- ShapeBench 2026: https://arxiv.org/abs/2605.20763
- AIAA DPW-8/AePW-4 test cases: https://www.aiaa-dpw.org/TestCases/testcases.html


## Machine-readable algorithm configuration receipt

Every summary row records a stable algorithm configuration ID, decision dimension, population size, maximum evaluation budget, and initial-population basis. The configuration IDs are uniform-random-sampling-v0.1, nsga2-sbx15-pm20-pc0.5-v0.1, and gde3-de-rand1-f0.5-cr0.9-v0.1. The NSGA-II-style settings are SBX distribution index 15, polynomial mutation distribution index 20, per-coordinate crossover probability 0.5, and mutation probability 1/dimension. GDE3 uses DE/rand/1 weight 0.5 and binomial crossover rate 0.9.

All policies use the same SplitMix64 seed for a task/replicate, producing identical initial population vectors before algorithm-specific search. Subsequent random streams diverge as the operators consume random values. The independent oracle checks that the initial vectors match across all policies.


## Disconnected-front qualification: ZDT3

ZDT3 uses the standard five disjoint non-dominated front intervals. The reference front is sampled at 201 points, allocated as 41/40/40/40/40 points across the five intervals. The hypervolume reference is integrated piecewise: analytic primitive along each Pareto interval, a constant-height rectangle across each disconnected gap, and a final rectangle to the declared reference point (1.1, 1.1). This exposes premature loss of entire Pareto segments that ZDT1/ZDT2's connected fronts cannot reveal.
