# Non-anticipative robust two-stage campaign planner

**Status:** finite deterministic campaign-tree reference implementation; exact-head CI pending  
**Tracking:** [#7088](https://github.com/Luminous-Dynamics/symtropy/issues/7088) · [campaign roadmap #7059](https://github.com/Luminous-Dynamics/symtropy/issues/7059)  
**Base:** randomized robust decision oracle head `d4addbf6bac36405213f6ea9a3ef137464c1f55e`  
**Branch:** `research/cmni-robust-campaign-v0`

## 1. What is new

The existing finite-horizon campaign planner optimizes expected utility under a single campaign likelihood model. The robust decision oracle handles a hidden dependence scenario but only for one acquisition. This module connects the two for a finite two-stage campaign.

A candidate is a **complete deterministic campaign tree**:
- stop at the root with an action or abstain; or
- acquire one root option;
- after each observed root outcome, stop or select one unused second acquisition;
- after each second-stage outcome, select a terminal action or abstain.

The entire tree is chosen before the hidden dependence scenario is revealed. Every scenario evaluates the same tree. The policy observes outcome history, not scenario identity.

## 2. Objective and constraints

For latent state \\(\\theta\\), admitted hidden dependence scenarios \\(s\\), and a complete tree \\(\\tau\\):

\\[
V_s(\\tau)=
\\mathbb E_s[U(a_{\\mathrm{terminal}},\\theta)]
-
\\lambda\\,\\mathbb E_s[C_{\\mathrm{acquisitions}}].
\\]

The deterministic robust campaign is:

\\[
\\max_{\\tau\\in\\mathcal T_{\\mathrm{feasible}}}
\\min_{s\\in\\mathcal S} V_s(\\tau).
\\]

A single tree is shared across every admitted scenario. The separate clairvoyant upper bound is:

\\[
\\min_s\\max_{\\tau\\in\\mathcal T_{\\mathrm{feasible}}}V_s(\\tau).
\\]

The difference is the non-anticipativity gap. A gap is not a bug; it measures how much a scenario-aware planner could overstate decision value.

Feasibility is pathwise: root acquisition cost plus the cost of any second acquisition must fit the hard budget on every branch. Acquisition IDs are one-shot. Stop and abstain policies are enumerated alongside acquisitions.

## 3. Frozen adaptive-value control

Fixture: `research/cmni/fixtures/robust-campaign-adaptive-value-v1.json`.

There are four equiprobable scientific states and four terminal actions. Correct action utility is one and incorrect action utility is zero.

Three binary measurements each cost 0.05 utility units:
- `coarse` separates states {0,1} from {2,3};
- `detail-a` distinguishes states 0 and 1;
- `detail-b` distinguishes states 2 and 3.

The hard campaign budget is 0.10. A one-stage design reaches gross expected utility 0.50; after cost, net utility is 0.45.

The adaptive policy acquires `coarse`, then uses `detail-a` on the first branch and `detail-b` on the second branch. It identifies all four states and obtains gross utility 1.00. Total path cost is exactly 0.10, so net utility is 0.90 under both admitted scenarios. Robust campaign EVSI relative to the prior action is 0.65. The frozen search enumerates 9,080 complete trees (80 at one stage).

The test also checks the actual selected tree structure, not just its scalar value.

## 4. Hidden-scenario adversarial control

A two-state calibration observation is perfectly informative in each of two scenarios, but the outcome/state mapping reverses between scenarios. Because the source scenario is hidden, one common tree cannot decode both readings. The robust planner therefore stops at prior utility 0.50; the scenario-aware clairvoyant upper bound is 0.95 after the 0.05 acquisition cost. The non-anticipativity gap is 0.45.

This control ensures that outcome semantics are not silently conditioned on hidden source identity.

## 5. Cross-check against CMNI one-shot decisions

The one-stage campaign test constructs the same state/action/likelihood model as the existing deterministic robust decision oracle #7034. It verifies:

`campaign worst-case net utility + declared acquisition cost = robust post-information utility`

This checks that the campaign tree evaluator and the one-shot robust decision module agree when no second acquisition is available.

## 6. Implementation boundaries

The Rust reference:
- enumerates every feasible stop/root/conditional-second-stage deterministic tree;
- keeps the complete tree fixed across scenarios;
- reports per-scenario expected terminal utility, expected resource cost and net utility;
- reports worst-case net utility, robust campaign EVSI, clairvoyant upper bound and non-anticipativity gap;
- enforces a hard complete-tree cap before enumeration and fails closed when it would be exceeded;
- validates common ordered scenario IDs and dependence-scope receipt references.

The likelihood interface assumes acquisitions are conditionally independent given the expanded scientific state and dependence scenario. The campaign receipt must cover that factorization, and the expanded state must contain all shared calibration/common-cause variables. The receipt string is only a scope identifier here; it is not authenticated by the kernel.

Exactness is limited to finite scenarios, finite two-stage trees and deterministic policies using f64 arithmetic. This is not yet a stochastic mixture over complete trees, a scalable POMDP, continuous PSD ambiguity optimizer, formal numerical certificate or physical validation.

## 7. Next increment

The next step is a zero-sum game over complete campaign trees and hidden dependence scenarios, reusing the randomized finite policy game solver from #7057. The randomization must mix complete contingent trees before scenario disclosure. Later, continuous PSD/elliptope dependence bounds can be introduced only after an independently validated separation oracle exists.
