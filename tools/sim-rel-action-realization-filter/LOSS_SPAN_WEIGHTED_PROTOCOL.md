# SIM-REL-033: exact loss-span-weighted deficiency for binary channels

Status: exact finite optimizer for channels with exactly two observations each and at most 13 hidden states. This is a weighted directed deficiency specialization, not the classical worst-state Le Cam deficiency and not a generalized multi-output solver.

## 1. Objective

For finite source channel A and target channel B with the same ordered hidden-state vocabulary, prior pi and terminal loss table L(d,s), define the state-specific loss span including explicit abstention:

rho_s = max_d L(d,s) - min_d L(d,s).

The task-specific objective is:

beta_pi,L(A -> B) = min_G sum_s pi(s) rho_s TV(B(.|s), A(.|s)G),

where G is row-stochastic. This is a prior- and decision-consequence-weighted directed deficiency. It must be kept separate from the classical minimax Le Cam deficiency:

delta(A -> B) = min_G max_s TV(B(.|s), A(.|s)G).

Weighted directed Le Cam deficiency is established prior work. SIM-REL-033 does not claim to introduce weighted deficiency. It computes an exact loss-span-weighted specialization with replayable rational witnesses.

## 2. Binary-output parameterization

For two observations in each channel, set x=G(0,0), y=G(1,0), both in [0,1]. The complementary entries are 1-x and 1-y.

For hidden state s, let a_s=A(0|s) and b_s=B(0|s). Then:

r_s(x,y) = b_s - a_s x - (1-a_s)y.

For binary outputs, TV equals |r_s(x,y)| and the objective becomes:

beta_pi,L = min_(x,y in [0,1]) sum_s pi(s) rho_s |r_s(x,y)|.

## 3. Exact arrangement-vertex oracle

The objective is convex and piecewise linear on the unit square. Its sign pattern changes only on the residual-zero lines:

a_s x + (1-a_s)y = b_s,

and the four square boundaries x=0, x=1, y=0, y=1.

These lines partition the square into bounded polygonal cells. Within each cell, every residual has fixed sign, so the objective is linear. A linear function on a compact polygon attains its minimum at a vertex; if it is constant, a vertex is still a minimizer. Every arrangement vertex is an intersection of two non-parallel lines from this set.

The implementation therefore:
- enumerates all residual-zero and square-boundary lines;
- computes every exact signed-rational pair intersection;
- rejects parallel/coincident pairs and intersections outside the square;
- deduplicates feasible points using an ordered set;
- evaluates the exact weighted objective at every retained point;
- resolves ties lexicographically in (x,y);
- independently replays the chosen kernel through the SIM-REL-031 statewise TV verifier and re-computes all weighted residual terms.

The supported maximum is 13 hidden states, giving at most 17 lines and 136 pair intersections. Unsupported dimensions and rational overflow fail closed.

## 4. Receipt

The result contains:
- exact stochastic garbling kernel and per-state TV residuals;
- per-state loss spans rho_s;
- prior/loss weights pi_s rho_s;
- each weighted state residual and exact total objective;
- arrangement line count, pair-intersection count and unique feasible candidate count;
- an explicit optimality flag, set only after exhaustive arrangement-vertex evaluation and residual replay.

## 5. Conformance fixtures

- Three-state uninformative source A with A(0|s)=1/2 and target B(0|s)=(0,1/2,1), prior=(4/5,1/10,1/10), and unit loss spans: minimax Le Cam deficiency is 1/2, while this weighted objective is 3/20 and chooses the all-to-output-1 kernel.
- Uniform prior and equal spans produce the central median objective 1/3.
- Scaling every loss span by 1/10 scales the objective from 3/20 to 3/200 without changing the minimizing witness.
- Zero loss spans produce a zero objective; repeated execution verifies deterministic tie-breaking.
- State permutations preserve the objective and unpermuted witness.
- Invalid priors and non-binary channels fail closed.
- Exhaustive rational grid with three hidden states and first-output probabilities in {0,1/2,1}: 729 source/target channel pairs.

## 6. Benchmark protocol

For every case, record exact residuals, loss spans, prior/loss weights, weighted objective, witness, arrangement candidates, rational bit growth and runtime. Compare three different quantities:
1. SIM-REL-031 fixed-witness decision-transfer certificate.
2. SIM-REL-032 classical worst-state Le Cam deficiency.
3. SIM-REL-033 prior/loss-span-weighted deficiency.

Use matched channels, priors and loss tables. Measure the objective improvement of the task-weighted kernel over the minimax kernel under the same task weights. Do not compare a weighted objective to a minimax objective as though they were interchangeable scores.

## 7. Literature positioning

Prior-weighted Le Cam deficiency is established in the statistical-experiment and channel-ordering literature. A review of channel ordering defines a weighted Le Cam deficiency as the prior expectation of the statewise total-variation simulation error, and *Le Cam meets LeCun* discusses weighted directed deficiency as an L1 optimization problem. The SIM-REL-033 objective uses the unnormalized state weights pi_s rho_s, so it is a loss-span-weighted specialization of the established concept, not a new deficiency notion.

References:
- Bertschinger et al., *Unique Information Through the Lens of Channel Ordering*: https://pmc.ncbi.nlm.nih.gov/articles/PMC11764511/
- *Le Cam meets LeCun: Deficiency and Generic Feature Learning*: https://citeseerx.ist.psu.edu/document?doi=5c6c0eb230764e72e7f0b8e63146f6f01656f948&repid=rep1&type=pdf

## 8. Claim boundary

This is an exact small-domain binary-output optimizer conditional on the supplied channels, prior and terminal loss table. It does not establish statistical calibration, continuous-state physical coverage, apparatus safety, generalized multi-output optimization or SOTA superiority.