# Black-hole spectral robustness and transient-response benchmark

Status: research specification and analytic reference fixtures; no Symtropy pseudospectrum or black-hole transient result is claimed.

Canonical owner: [BH-SPECTRAL-ROBUSTNESS-001 #4138](https://github.com/Luminous-Dynamics/symtropy/issues/4138).

## Why this work exists

Black-hole ringdown cannot be qualified by QNM eigenvalues alone. The relevant perturbation generators are non-normal in important formulations; QNM eigenvectors need not be orthogonal in the physical energy product, transient plateaus and non-modal response can occur, and small operator perturbations can move some spectral features substantially. At the same time, spectral sensitivity is not synonymous with a large detector-visible waveform change.

The 2026 literature makes a clear next step possible:

- Pradhan, Ghaderi & Zeyauddin, *Error budgets from pseudospectra for black hole ringdown: a Schwarzschild baseline*, Eur. Phys. J. C 86, 853 (2026-07-22), DOI [10.1140/epjc/s10052-026-16135-8](https://doi.org/10.1140/epjc/s10052-026-16135-8): a reproducible finite-difference first-order generator for Schwarzschild scalar and Regge–Wheeler perturbations, matrix pseudospectra, pulse evolution and detector waveform. The authors explicitly scope numerical contours to their state variables, Euclidean norm, computational interval, resolution and boundary penalty; they do not identify them with continuum pseudospectra or physical uncertainty intervals.
- Wu et al., *The structured pseudospectrum of Schwarzschild black hole*, arXiv:2611.00853 (submitted 2026-10-31): an important new preprint proposing EFT- and Rezzolla–Zhidenko-motivated perturbation spaces and two structured norms. It reports substantially reduced accessible instability under physical constraints, particularly for higher overtones, and anisotropic approximately elliptical contours at small perturbation scales. This is an arXiv preprint, not yet a peer-reviewed result as of the date of this specification.
- Daghigh et al., *Sensitivity of black hole spectral instability to ultraviolet perturbations*, Phys. Rev. D 114, 084005 (2026-10-01), DOI [10.1103/lq4q-89t2](https://doi.org/10.1103/lq4q-89t2): localized UV and discontinuity perturbations of the Regge–Wheeler potential, including perturbation-width / integrated-strength effects and trajectories of the fundamental mode.
- Besson et al., *Transients in black hole perturbation theory*, Frontiers in Physics 13 (2025), DOI [10.3389/fphy.2025.1638583](https://doi.org/10.3389/fphy.2025.1638583): review of non-normality, non-orthogonal QNMs, transient plateaus, transient growth and pseudospectral caveats.
- Su et al., *Decomposition of the Schwarzschild Green's function*, Phys. Rev. D 113, 104013 (2026-05-08), DOI [10.1103/3n8r-8kbb](https://doi.org/10.1103/3n8r-8kbb): separates branch-cut direct response, QNM-pole response and late-time tail, validated against independent time-domain Regge–Wheeler simulations.
- Rosato et al., *Excitation factors for horizonless compact objects: Long-lived modes, echoes, and greybody factors*, Phys. Rev. D 113, 084002 (2026-04-01), DOI [10.1103/v7pv-y3qy](https://doi.org/10.1103/v7pv-y3qy): shows why a weakly damped mode may have a very small excitation factor and contribute only at very late times.
- *Unstable Chords and Destructive Resonant Excitation of Black Hole Quasinormal Modes*, Phys. Rev. Lett. 135, 031401 (2025), DOI [10.1103/ht2n-vvvh](https://doi.org/10.1103/ht2n-vvvh): a key caution that spectral sensitivity and observable waveform sensitivity can differ because mode contributions interfere.

The scientific target is therefore not “make a larger pseudospectrum plot”. It is a verifiable connection from a declared continuum equation, norm and structured physical perturbation class to source-specific time-domain response and detector-visible waveform.

## Mathematical objects that must remain separate

1. Frequency-domain operator pencil \(\mathcal L(\omega)\).
2. Declared first-order evolution generator \(\partial_t u=A u\).
3. Resolvent \((zI-A)^{-1}\), with domain, inner product and norm.
4. QNM poles/eigenvalues and left/right eigenvectors.
5. Pseudospectra under a stated norm and perturbation class.
6. Semigroup transient gain \(\|e^{tA}\|\), under a stated norm.
7. Source-specific response \(C e^{tA}B\), or equivalent retarded Green-function response.
8. Detector response and extraction-operator output.

A first-order generator is not interchangeable with the frequency-domain pencil without an explicit reduction. The Euclidean norm of a state vector is not automatically a physical energy norm. A pseudospectral radius is not automatically a physical uncertainty interval. A large unstructured matrix pseudospectrum does not prove that a physically allowed perturbation produces a large waveform residual.

## Frozen analytic matrix fixtures

The companion machine-readable manifest is [black-hole-spectral-robustness-v1.json](./black-hole-spectral-robustness-v1.json).

### Normal diagonal operator

\[
A_N=\begin{pmatrix}-1&0\\0&-2\end{pmatrix}.
\]

Under the induced Euclidean 2-norm, the exact minimum singular value is

\[
\sigma_{\min}(A_N-zI)=\min(|z+1|,|z+2|).
\]

Its \(\varepsilon\)-pseudospectrum is the union of closed \(\varepsilon\)-disks centred on -1 and -2, and \(\|e^{tA_N}\|_2=e^{-t}\) for \(t\ge0\). This is the normal-operator negative control.

### Stable Jordan operator with transient growth

\[
A_J=\begin{pmatrix}-1&4\\0&-1\end{pmatrix}.
\]

Both eigenvalues are -1, but the generator is non-normal. Put \(q=|z+1|^2\). Then

\[
\sigma_{\min}(A_J-zI)=
\sqrt{\frac{16+2q-\sqrt{256+64q}}{2}}.
\]

For \(t\ge0\),

\[
\|e^{tA_J}\|_2=
e^{-t}\frac{\sqrt{4+16t^2}+4t}{2}.
\]

The maximum occurs at \(t=\sqrt{12}/4\approx0.8660254\), with gain approximately 1.5697753, despite both eigenvalues being strictly damped. The numerical abscissa is +1. This is a known-answer software fixture, not a black-hole physical model.

These two matrix controls detect implementations that conflate spectral location with resolvent sensitivity or finite-time response.

## Analytic barrier fixture

Use the Pöschl–Teller barrier

\[
V(x)=V_0\,\operatorname{sech}^2(x/b)
\]

in the master equation

\[
\frac{d^2X}{dx^2}+[\omega^2-V(x)]X=0,
\]

with time convention \(e^{-i\omega t}\), ingoing behaviour at the left boundary and outgoing behaviour at the right boundary.

For \(V_0=2\), \(b=1\) in dimensionless units, the exact QNM frequencies are

\[
\omega_n=\pm\sqrt{2-\frac14}-i\left(n+\frac12\right)
=\pm\frac{\sqrt7}{2}-i\left(n+\frac12\right),
\quad n=0,1,2,\ldots
\]

These are pole-location fixtures only. The residue / excitation factor must be calculated separately from the declared Wronskian normalization; do not infer the amplitude from the frequency. Use this barrier as an analytic testbed for frequency-domain scattering, structured potential perturbations, and time-domain response before moving to Schwarzschild.

The exact formula is documented in the black-hole perturbation literature; one accessible reference discussion is [Spectral Instability in Modified Pöschl–Teller Effective Potential Triggered by Deterministic and Random Perturbations](https://www.mdpi.com/2218-1997/12/1/5). Pin exact parameter and sign conventions in every reproduction.

## Structured perturbation families

At minimum compare:
- unstructured matrix perturbations (diagnostic upper bound only);
- EFT-motivated effective-potential deformations;
- Rezzolla–Zhidenko metric / potential deformations;
- localized UV perturbations, thin-shell / jump perturbations and smooth compact-support perturbations;
- inner-boundary reflection / impedance perturbations;
- source / initial-data perturbations;
- discretization, finite-domain, state-scaling and boundary-penalty changes;
- measurement / extraction-operator perturbations.

For every family specify support, smoothness, normalization, norm, admissibility constraints, and how it maps into the physical generator. Keep pole movement, residue movement, source excitation and waveform changes separate.

## Execution sequence

1. Reproduce the exact normal and Jordan matrix fixtures and the known semigroup / singular-value formulae.
2. Reproduce the analytic Pöschl–Teller QNM pole locations; independently compute Wronskians and residues.
3. Reproduce the published July 2026 Schwarzschild scalar and Regge–Wheeler matrix-level pseudospectral results with exact state scaling, norm, interval, boundary enforcement, resolution and scan.
4. Reproduce the structured EFT and RZ perturbation spaces in arXiv:2611.00853 when exact conventions / data are pinned. Treat the preprint's quantitative results as replication targets, not assumed facts.
5. Compare smooth, localized UV, thin-shell and jump perturbations using the published October 2026 baseline.
6. Compare eigenvalue movement, eigenvector conditioning, resolvent norms and transient semigroup gain.
7. Map each perturbation through source-specific response and detector waveform; test destructive interference.
8. Compare pole + branch-cut reconstruction against independent time-domain evolution, consuming Symtropy #4052 and #4109.
9. Propagate measured structured-response discrepancy into Symthaea #10944 and the identifiability atlas #11021.
10. Run blinded null, positive and operator-only controls; report confidence intervals and exact source/data digests.

## Required benchmark reporting

Every result must report:
- continuum equation and first-order reduction;
- state variables, units and scaling;
- physical inner product and any alternative norm;
- operator domain, interval and boundary condition / penalty;
- discretization, resolution and precision;
- complex scan region and sample density;
- perturbation family, norm and admissibility map;
- eigenvalue and left/right eigenvector diagnostics;
- resolvent / \(\sigma_{\min}\) contour convergence;
- semigroup gain and specified-pulse response;
- pole / residue / branch-cut contributions where available;
- waveform mismatch, phase/amplitude discrepancy and detector projection;
- runtime and memory at matched accuracy;
- exact source revision, configuration and input digest.

Do not aggregate spectral sensitivity, transient gain, waveform mismatch and posterior coverage into one score.

## Cross-repository ownership

- Symtropy #4138: operator, structured pseudospectrum and transient-response diagnostics.
- Symtropy #4052: independent time-domain perturbation reference.
- Symtropy #4109: scattering, Green function, boundary response, poles, residues and branch cuts.
- Symtropy #4073: chart identity, state scaling and numerical conditioning.
- Symthaea #11002: extraction operator and residual attribution.
- Symthaea #10944: correlated waveform discrepancy and posterior calibration.
- Symthaea #11021: geometry-to-observation identifiability and active experiment design.
- Mycelix #5903: provenance and replication receipts.
- Sol Atlas #586: distinct spectrum, pseudospectrum, physical response, extracted signal and posterior visualization.

## Claim ceiling

No claim of a continuum pseudospectrum, physical uncertainty bound, transient waveform amplification, or improved ringdown inference until the relevant norm, formulation, domain, physical perturbation class, source, detector map and convergence are specified and independently validated.
