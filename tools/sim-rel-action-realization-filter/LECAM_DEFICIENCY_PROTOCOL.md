# SIM-REL-032: exact binary-output Le Cam deficiency oracle

Status: exact finite-channel oracle for two observations per channel and at most 13 hidden states. Not a generalized Le Cam deficiency solver.

## 1. Objective

For finite channels A and B with the same ordered hidden-state vocabulary, directed Le Cam deficiency under the convention TV = one half L1 is:

delta(A -> B) = min_G max_s TV(B(.|s), A(.|s) G),

where G is a row-stochastic post-processing kernel mapping A observations to B observations.

For channels with exactly two observations each, write x = G(0,0), y = G(1,0). Then the other garbling probabilities are 1-x and 1-y. The residual in state s is:

r_s = B(0|s) - A(0|s)x - (1-A(0|s))y.

For binary outputs, total variation equals |r_s|. Therefore deficiency is the exact linear-program optimum:

minimize epsilon
subject to -epsilon <= r_s <= epsilon for every state s,
0 <= x,y,epsilon <= 1.

This is a small-domain exact oracle. It does not solve general multi-output Le Cam deficiency.

## 2. Independent optimality qualification

The implementation requires two mathematically independent exact procedures to agree.

### 2.1 Fourier-Motzkin projection

The three-variable rational inequality system [x,y,epsilon] is projected by eliminating x and y. The resulting one-dimensional inequalities provide an exact lower endpoint for feasible epsilon. At that minimum, epsilon is substituted into the original system, and the existing exact boxed feasibility routine reconstructs x and y.

### 2.2 Independent active-set vertex enumeration

The feasible polytope is bounded in [0,1]^3. Its minimum is attained at a vertex with three linearly independent active constraints. The independent oracle enumerates every 3-constraint active set, solves each 3x3 system by signed-rational Gaussian elimination, and retains only candidates satisfying every original inequality.

The exact deficiency is accepted only when:
- the Fourier-Motzkin minimum equals the independent active-set minimum;
- the reconstructed stochastic garbling kernel passes exact row-probability checks;
- the SIM-REL-031 statewise residual verifier reports maximum residual exactly equal to the optimum.

The vertex oracle has a hard cap of 32 inequalities: six box inequalities plus two residual inequalities per hidden state, hence at most 13 hidden states. Exceeding the cap is an explicit unsupported-size error. It is never reported as infeasibility or non-dominance.

## 3. Receipt

The result retains:
- exact witness G;
- statewise TV residual vector and maximum residual;
- exact directed deficiency;
- Fourier-Motzkin pair-product count;
- active-set count and feasible-vertex count;
- independent vertex lower bound;
- Boolean agreement of both optimizers and the replayed residual.

A candidate kernel alone is only a primal upper bound. Exact optimality is asserted only after the independent vertex lower bound agrees with the projection result.

## 4. Conformance fixtures

- Perfect binary channel to symmetric 80%-accurate channel: deficiency exactly 0; the witness is the 80%-accurate garbling.
- Uninformative channel to symmetric 80%-accurate channel: deficiency exactly 3/10.
- Symmetric 70%-accurate channel to symmetric 80%-accurate channel: deficiency exactly 1/10.
- Three hidden states with binary outputs: exact zero deficiency and invariance to a permutation of the hidden-state ordering.
- Non-binary observation channels and mismatched state vocabularies: rejected.
- Every fixture requires equality of the Fourier-Motzkin and independent vertex optima.

## 5. Benchmark programme

Start with exhaustive rational grids for two-state, then three-state and up to the declared 13-state cap. Report:
- exact objective agreement between the two optimality procedures;
- witness replay and statewise residual correctness;
- count of active sets and feasible vertices;
- FME pair products and intermediate constraint counts;
- numerator/denominator growth and checked-arithmetic overflow cases;
- runtime and unsupported-size rate;
- canonical witness stability under repeated execution and state/observation permutations.

Compare the fixed-witness SIM-REL-031 residual, the minimax deficiency, and the task-specific prior/loss-weighted optimum from future SIM-REL-033 as separate quantities. Do not scalarize or conflate them.

## 6. Relationship to SIM-REL-031 and the literature

SIM-REL-031 verifies a supplied witness and converts its residual into a task-specific risk bound. SIM-REL-032 additionally optimizes the witness for the classical worst-state deficiency objective. The prior/loss-weighted objective is a distinct task-specific optimization, not Le Cam deficiency.

Le Cam deficiency and approximate comparison of statistical experiments are classical. This PR implements a bounded, exact, auditable two-output oracle and does not claim a new definition or general-purpose deficiency solver.

References:
- Torgersen, *Comparison of Statistical Experiments*: https://www.cambridge.org/core/books/comparison-of-statistical-experiments/405F64D2A2FB3A4E6A1893C1C8A4A60B
- Abraham & Nickl, *Le Cam deficiency and Le Cam distance*: https://www.statslab.cam.ac.uk/~nickl/Site/__files/MSL.pdf

## 7. Claim boundary

The solver applies only to finite channels with exactly two observations each and at most 13 hidden states. It does not provide the general multi-output solution, continuous-state guarantees, statistical calibration, physical apparatus coverage, or SOTA superiority.