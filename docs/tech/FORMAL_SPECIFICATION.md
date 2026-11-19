# Experimental Metric-Coupled Agent Model: Formal Specification

**Document status:** research-model specification. The equations below describe one configurable simulation model, not an established physical theory, material constitutive law, regulatory standard or experimentally validated model of consciousness.

## Abstract

This document specifies an experimental agent-and-rigid-body model in which an operationally computed metric Φ, resource reservoirs, collision events and configurable harmony variables influence agent behavior and selected contact callbacks. It defines five software coupling channels, resource bookkeeping and hypotheses that can be tested within the model.

The model is useful for simulation research, sensitivity analysis and falsification. Its equations and numerical outcomes do not establish that Φ is a physical force, that the system is thermodynamically closed, that cooperation is universally required by thermodynamics, or that the model explains subjective experience.

## 1. Scope, quantities and assumptions

An agent (i) at model time (t) is represented by:

[
S_i(t) = (x_i, v_i, \omega_i, \Phi_i, E_i, h_i, \sigma_i, \pi_i)
]

where:
- (x_i \in \mathbb{R}^D): position in the declared simulation coordinate frame;
- (v_i \in \mathbb{R}^D): linear velocity;
- \(\omega_i \in \Lambda^2(\mathbb{R}^D)\): angular-velocity representation used by this model;
- \(\Phi_i \in [0,1]\): output of a specifically configured metric estimator;
- (E_i): model resource reservoir (treated as joules only if its source, units and conversion are calibrated);
- (h_i \in [0,1]^8): eight-component harmony parameter vector used by this experiment;
- \(\sigma_i \geq 0\): model prediction-error state;
- \(\pi_i \in [0,1]\): motor-precision factor defined by the update rule below.

The following assumptions apply:
1. Φ is a model input/state produced by a selected estimator. Do not equate a configured scalar with a universal physical field or subjective experience.
2. (h_i) is a parameter vector. The name “harmony” does not identify a standard physical quantity.
3. Numerical constants are experiment parameters until their dimensions, calibration data and validity domain are established.
4. Energy-like bookkeeping is not proof of first-law closure. Every physical source, sink, internal transfer and boundary must be identified and audited.
5. The model is deterministic only under the exact execution profile and random-seed policy used for a run.

## 2. Motor-authority mappings

One experimental four-tier mapping is:

[
g(\Phi) =
\begin{cases}
1.0 & \Phi > 0.6 \\
0.6 & 0.3 < \Phi \leq 0.6 \\
0.3 & 0.1 < \Phi \leq 0.3 \\
0.0 & \Phi \leq 0.1
\end{cases}
]

Effective motor gain:

[
G_i = g(\Phi_i)\,\pi_i
]

This is a model policy mapping, not an NRC requirement or a safety certification. The repository README records that platform demos use a separate two-level sprint/floor gate with platform-specific thresholds; the four-tier mapping must not be assumed to describe every demo or deployed platform. A threshold requires calibration and adverse-case testing for the relevant platform.

## 3. Five experimental coupling channels

### 3.1 Metric-conditioned intended force

[
F_{\mathrm{effective}} = G_i\,F_{\mathrm{intended}}
]

This equation defines a motor-command policy. It is not a claim that Φ directly generates or attenuates a natural force. If the policy is used in a physical robot, its safety and performance require an independently specified controller, actuator model, authority boundary and physical validation.

### 3.2 Resource-budget model

Movement cost:

[
\frac{dE_{\mathrm{move}}}{dt} =
-c_{\mathrm{move}}\,\lVert v_i\rVert\,\kappa_{\mathrm{sprint}}
]

where the coefficient must have units consistent with the selected distance/time convention (for example, J/m if speed is in m/s and this is an energy rate).

Metric-dependent maintenance:

[
\frac{dE_{\mathrm{maint}}}{dt} =
-c_{\mathrm{maint}}(1 + 0.5\,\Phi_i)
]

Collision-associated budget change:

[
\Delta E_{\mathrm{collision}} =
-c_{\mathrm{collision}}\,\lvert J_n\rvert
]

These expressions are experimental accounting rules. The coefficients require dimensions, calibration and source provenance before (E_i) can be interpreted as a measured SI energy reservoir. In particular, impulse is not energy; the collision coefficient must not be treated as a physically derived impact-energy relation without an explicit mechanical model.

### 3.3 Sanctuary-zone impulse rule

For configured conditions (h_i[7] > 0.6), \(\sum_k h_i[k] > 2.0\), and \(\Phi_i > 0.3\), a model-defined zone of radius (r_s) may be formed. Its callback rule is:

[
J_{\mathrm{dampened}} =
J\left(1 - \delta\left(1 - \frac{d}{r_s}\right)\right)
]

with:

[
\delta = 0.9\,h_i[7]\,\min(\Phi_i,1)
]

This is a synthetic impulse-modulation rule. It is not a general collision law. Any real-world analogue would need a measurable physical mechanism, a dimensional and momentum/energy account, and independent validation.

### 3.4 Harmony-field friction rule

A model field is defined by:

[
H(x) =
\sum_i
\frac{s_i}{\max(\lVert x-x_i\rVert,1)^2}\,h_i
]

The model's resonance is the cosine similarity:

[
R(a,b) =
\frac{a\cdot b}{\lVert a\rVert\,\lVert b\rVert}
]

and the callback coefficient is:

[
\mu_{\mathrm{effective}} =
\mu_{\mathrm{base}}\left(1 - 0.5\,R(H(x),h_{\mathrm{agent}})\right)
]

Zero-norm vectors require an explicit disposition; they must not yield NaN or an implicit favorable result.

The rule maps an abstract parameter similarity to a friction multiplier. It is a phenomenological research intervention, not evidence that information integration or harmony changes the friction of real materials. If used for engineering, replace it with a calibrated material/contact model appropriate to the physical interface.

### 3.5 Collision prediction-error feedback

For a collision with impulse magnitude (lvert Jvert):

[
\Delta \sigma_i = \min(0.01\,\lvert J\rvert,2.0)
]

[
\pi_i = \frac{1}{1+\sigma_i}
]

A per-tick recovery rule is:

[
\sigma_i(t+1) = \sigma_i(t)(1-\lambda)
]

with experimental parameter (lambda=0.05). The update cadence and units of impulse must be frozen as part of the scenario identity. This rule is a chosen agent-response model, not a general law of biological motor adaptation.

## 4. Resource sources, transfers and ledger closure

The experiment may model three resource mechanisms.

### 4.1 Ambient source term

[
\frac{dE_{\mathrm{ambient}}}{dt} =
c_{\mathrm{ambient}}\,R_{\mathrm{collective}}
]

where:

[
R_{\mathrm{collective}} = 0.5 + 1.5\,\Phi_{\mathrm{collective}}
]

This is an imposed source term unless a physical source and conversion mechanism are specified. If it adds energy to agent reservoirs, the corresponding source term must be recorded in the ledger.

### 4.2 Finite resource wells

[
\frac{dE_{\mathrm{well}}}{dt} =
-c_{\mathrm{well}}
\quad \text{when } \lVert x_i-x_{\mathrm{well}}\rVert < r_{\mathrm{well}}
\]

A transfer from a well to an agent must debit the well and credit the agent by equal amounts. A well cannot supply more than its remaining modeled capacity.

### 4.3 Pairwise transfer rule

For agents (i,j) within the configured interaction radius:

[
R_{ij}=R(h_i,h_j)
]

[
\frac{dE_{i\leftarrow j}}{dt} =
c_{\mathrm{reson}}\,\max(R_{ij}-0.5,0)\,2
\quad \text{when }R_{ij}>0.5
]

This rule is an internal transfer only if the source agent or reservoir is debited by the same amount. Otherwise it is an additional source, not energy conservation.

### 4.4 Ledger identity

A physically interpretable energy balance requires all boundaries and flows to be declared. For a closed accounting scope:

[
E_{\mathrm{initial}} + E_{\mathrm{in}} =
E_{\mathrm{final}} + E_{\mathrm{out}} + E_{\mathrm{dissipated}}
]

where each term has the same energy unit and time interval, internal transfers cancel pairwise, and stored energy includes every modeled reservoir. Define the residual:

[
r_E =
E_{\mathrm{initial}} + E_{\mathrm{in}}
- E_{\mathrm{final}} - E_{\mathrm{out}} - E_{\mathrm{dissipated}}
]

Report both absolute and normalized residuals with their boundary, interval, units and numerical uncertainty. The often-used 1% tolerance is only a proposed experiment-specific threshold; it is not a universal thermodynamic standard.

Until every source/sink and transfer is accounted for with validated units and physical boundaries, call the result **model resource bookkeeping**, not thermodynamic closure.

## 5. J/Φ diagnostic

The experimental statistic:

[
J/\Phi =
\frac{\sum_i E_{\mathrm{consumed},i}\,\Phi_i}
{\sum_i\lvert\Delta\Phi_i\rvert}
]

has the interpretation of a model-specific energy-weighted metric-change ratio only if the energy and Φ definitions, time window and aggregation are fixed. It is not a standard physical constant or established measure of the energy cost of consciousness. Its units and behavior may change under rescaling, estimator changes or a different aggregation rule. Any claim of stability must be tested under parameter variation, estimator variation, independent implementations and held-out scenarios.

## 6. Landauer principle: narrow applicability

Landauer's bound is:

[
E_{\min}=k_B T\ln 2
]

for the minimum heat dissipated when an irreversible logical operation erases one bit of information in an environment at temperature (T), under the principle's assumptions.

At (T=310\,\mathrm{K}), this is approximately (2.97\times10^{-21}\,\mathrm{J/bit}). This is **not** a universal lower bound on the energy of consciousness, integrated information, every computation or every model state. Applying it to a Symtropy experiment requires an explicit mapping from modeled logical erasures to physical bit erasures, a temperature, an implementation and an energy boundary. Without that mapping, the Landauer expression is contextual background, not a constraint enforced on the simulation's energy variables.

## 7. Falsifiable model hypotheses

The following are hypotheses about this specific configurable simulation model. They are not established universal physical predictions:

1. **Clustering under resource constraints:** for a frozen parameter family, interaction and transfer rules may produce different clustering distributions than a non-cooperative control. Test against ablations, parameter sweeps and independent seeds.
2. **Governance-like coordination:** increasing the number of agents may change resource-allocation outcomes under the selected policy. This does not show that governance is thermodynamically necessary in real societies.
3. **Topology/metric association:** a procedurally defined topology may correlate with a model-defined collective metric. This is a property of the generative rule until tested against independent data.
4. **J/Φ statistic behavior:** the ratio may converge under some substrate/parameter configurations. Test whether this persists under changes to timestep, agent count, metric estimator, resource policy and normalization.

Every result should include exact source revision, configuration digest, seed policy, sample size, exclusions, statistical analysis and all negative controls. Separate exploratory results from preregistered confirmation and independent physical validation.

## 8. Required validation before stronger claims

- **Dimensional analysis:** every energy, impulse, force, time, length, temperature and metric coefficient has defined units.
- **Ledger audit:** every external source/sink and internal transfer is represented once; no source is duplicated and no missing sink is silently ignored.
- **Sensitivity:** vary coefficients, thresholds, number of agents, timestep, solver profile and metric estimator.
- **Ablations:** remove each coupling channel in turn; include shuffled/random metric controls where appropriate.
- **Numerical verification:** check timestep refinement, numerical stability, boundedness, invariants and replay under a frozen execution profile.
- **Independent analysis:** use a second implementation or test oracle where practical; preserve disagreement.
- **Physical validation:** if a physical interpretation is proposed, use independent calibrated observations relevant to that mechanism and context of use.

Synthetic rollout counts do not increase the number of independent physical experiments. A passing code test or replay only establishes its declared software property.

## References

- Tononi, G. (2004). An information integration theory of consciousness. *BMC Neuroscience*, 5(1), 42.
- Friston, K. (2019). A Free Energy Principle for a Particular Physics. *arXiv:1906.10184*.
- Adams, R.A., Shipp, S. & Friston, K.J. (2013). Predictions not commands: active inference in the motor system. *Brain Structure and Function*, 218(3), 611–643.
- McFadden, J. (2020). Integrating information in the brain's EM field: the cemi field theory of consciousness. *Neuroscience of Consciousness*, 2020(1), niaa016.
- Landauer, R. (1961). Irreversibility and Heat Generation in the Computing Process. *IBM Journal of Research and Development*, 5(3), 183–191.
- Lundbak, M. et al. (2023). Phi fluctuates with surprisal. *PLOS Computational Biology*, 19(10), e1011517.
