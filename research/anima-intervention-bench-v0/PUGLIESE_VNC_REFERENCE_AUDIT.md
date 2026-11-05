# Pugliese VNC Rate-Network Reference Audit

**Profile:** \`pugliese-vnc-half-tanh-rate-network\`  
**Pinned upstream code commit:** \`10e7661bf414ba7b4c2edf795cd36d0f878c17c0\`  
**Status:** source audited; external data artifact identities unresolved.

This is a source-derived implementation profile for the Symtropy / Symthaea reproduction programme. It is not a local reproduction, a reimplementation of the biological model, or an empirical result.

## Version-of-record and provenance

The peer-reviewed version of record is *Connectome simulations identify a central pattern generator for fly walking*, Nature, published 28 October 2026, DOI \`10.1038/s41586-026-11195-y\`.

The public code repository README still points to the 2025 bioRxiv v2 preprint and describes the project as a work in progress. It points to Zenodo record **22260924**. The Nature version-of-record data availability points to **22260923** for simulation outputs and Anipose data.

These record IDs must not be assumed equivalent. Neither is selected as the universal canonical data record here. Materialize both record manifests, compare files/checksums/roles, and map each data artifact to its cited source.

## Mathematical reference model

For rate state \(R(t)\), the inspected source implements:

\[
u(t) = I(t) + W_{\mathrm{eff}}R(t)
\]

\[
A_i(t) = \max\left(
f_{\mathrm{cap},i}
\tanh\left(
\frac{a_i}{f_{\mathrm{cap},i}}
(u_i(t)-\theta_i)
\right),0
\right)
\]

\[
\frac{dR_i(t)}{dt}=\frac{A_i(t)-R_i(t)}{\tau_i}
\]

with:

\[
W_{\mathrm{eff}} =
m_E\max(W^T,0) + m_I\min(W^T,0)
\]

and:

\[
I_i(t)=I_{i,0}\,\mathbf 1[t_{\mathrm{pulse,start}}\leq t\leq
t_{\mathrm{pulse,end}}],\qquad R_i(0)=0.
\]

The transposition, sign split and separate multipliers are material model semantics, not implementation details to be guessed or silently reordered.

## Pinned DNg100 baseline configuration

| Parameter | Source setting |
|---|---:|
| Stimulated neuron index | 31 |
| Stimulus current | 250 |
| Parameter replicates | 1,024 |
| Seed | 1 |
| Total simulation time | 2.0 s |
| Requested initial integration step | 0.001 s |
| Pulse start | 0.020 s |
| Pulse end | 1.999 s |
| Diffrax solver | Dopri5 |
| Relative tolerance | \(2\times10^{-6}\) |
| Absolute tolerance | \(5\times10^{-9}\) |
| Maximum solver steps | 100,000 |
| Save grid | 2,001 points at 1 ms intervals |

The value \`dt=0.001\` is the requested initial step for an adaptive solver, not proof that every accepted integration step is 1 ms. The actual accepted step sequence and termination state are part of the reproduction evidence.

The parameter profile is sampled from truncated-normal distributions, then the activation slope and threshold are scaled by normalized neuron size. Defaults are \(\tau=0.02\pm0.002\) s, \(a=1.0\pm0.1\), threshold \(7.5\pm0.6\), firing-rate cap \(200\pm10\) Hz, and excitatory/inhibitory multipliers 0.03. Do not substitute a constant-parameter run for this distributional experiment and call it an exact reproduction.

## Two source-level numerical findings

### 1. NaN/Inf event is declared but not attached

The source defines a NaN/Inf event function, but the \`Event(...)\` attachment is commented out in the \`diffeqsolve\` call. The call also uses \`throw=False\` and \`max_steps=100000\`.

After solving, the output pipeline replaces NaN/Inf state values with zero and clips all firing rates into [0, 1000] Hz. The returned array is therefore post-processed; it is not necessarily a value-for-value representation of the raw Diffrax solution.

A successful reproduction must capture raw \`solution.ys\`, Diffrax result/termination state, accepted/rejected steps and full save-grid completion **before** replacing or clipping anything. Any transformation must have its own receipt and must not be mistaken for raw model behaviour.

The benchmark crate now provides \`RawTrajectoryAudit::validate_unmodified\`. It refuses an incomplete save grid, a non-completed solver, raw non-finite values, any non-finite replacement, any clipping, a dimension mismatch or a missing raw-state digest.

### 2. The configured input-noise parameter is inactive in this RHS

The RHS receives \`noise_stdv\`, but the random input-noise term is commented out. Consequently the inspected \`run_single_simulation\` path does not use this parameter to perturb input current. The separate weight-noise path changes the connectivity matrix and is a different intervention.

Do not claim to have reproduced input-noise robustness using a run where this parameter is merely set to a nonzero value. Input-noise intervention must either be implemented in a separately versioned experimental fork or explicitly classified as inactive in the reference implementation.

## Connectome identity and data custody

The default MANC connectivity and neuron-table paths are external local files and are not included in the public code repository. Matrix axes, neuron-table row identities, the stimulated index and any retained mini-circuit must be joined by exact stable neuron IDs before mapping integer indices across exports.

The \`DNg100_Stim_CoreCPG\` configuration uses \`keepOnly=[31,277,617,1167]\`. This is an index set only. Do not assert which three entries are E1/E2/I1 until the exact matching annotation table and connectivity matrix from that materialization have been identity-checked.

## Claim ladder

1. Source/config audit (this document).
2. Exact data artifact and neuron-identity resolution.
3. Software-level reproduction of a single reference simulation, including raw solver status.
4. Distributional reproduction across the declared parameter replicates.
5. Reproduction of the paper's perturbation and pruning results.
6. Held-out intervention prediction against competing mechanisms.
7. Biological generalization across datasets, individuals and species.

Do not skip levels. A source audit does not prove the model is correct; a synthetic software test does not prove an animal result; a post-processed output does not prove a raw numerical trajectory.
