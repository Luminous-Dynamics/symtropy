# Exact extremal 5D EGB near-horizon attractor: branch-endpoint reference

**Owner:** [#6368 BH-5D-EGB-EXTREMAL-ATTRACTOR-001](https://github.com/Luminous-Dynamics/symtropy/issues/6368)

**Status:** exact algebraic specialization of the published extremal near-horizon family, with a 39-point dimensionless reference grid. This is not a global black-hole solution.

## Why this is a useful new anchor

The current Akribeia reproduction [#6063](https://github.com/Luminous-Dynamics/symtropy/issues/6063) targets a non-extremal equal-spin 5D EGB sweep at fixed horizon angular velocity. Separately, Kleihaus, Kunz and Radu (2023) construct the extremal equal-spin family and an exact rotating squashed \(AdS_2\times S^3\) near-horizon attractor. The paper reports that the global domain is bounded by the static branch, the extremal branch and the Myers–Perry branch; it also constructs the local attractor algebra and compares it with global numerical solutions. This supplies an independent local horizon reference for the endpoint that is difficult to reach with a non-extremal radial solver.

Source: Kleihaus, Kunz & Radu, *Phases of Rotating Black Objects in d = 5 Einstein–Gauss–Bonnet Theory*, Universe 9 (2023) 156, DOI [10.3390/universe9040156](https://doi.org/10.3390/universe9040156), Eqs. (13), (16)–(20).

## Coupling convention: factor four is mandatory

The 2023 article uses
\[
I\propto\int\sqrt{-g}\,[R+\alpha_{NH}\mathcal L_{GB}],
\]
whereas the Symtropy/Brihaye reduced action is
\[
I\propto\int\sqrt{-g}\,[R+\alpha_{sym}\mathcal L_{GB}/4].
\]
Therefore
\[
\boxed{\alpha_{sym}=4\alpha_{NH}}.
\]

This is not just a notation assumption. The 2023 static BD lapse
\[
N(r)=1+\frac{r^2}{4\alpha_{NH}}
\left(1-\sqrt{1+\frac{8\alpha_{NH}(r_H^2+2\alpha_{NH})}{r^4}}\right)
\]
is exactly identical, after rationalization and the factor-four map, to the Symtropy lapse
\[
F(r)=1-\frac{2r_H^2+\alpha_{sym}}
{r^2\left(1+\sqrt{1+\alpha_{sym}(\alpha_{sym}+2r_H^2)/r^4}\right)}.
\]
The replay script asserts this identity symbolically.

## Exact local attractor data

The near-horizon metric is parameterized by
\[
f_0=v_1r^2,\quad f_1=v_1/r^2,\quad f_2=v_2,\quad
f_3=v_2v_3,\quad w=-kr.
\]
Here \(v_3\) is the dimensionless squashing parameter. For \(0<v_3<2\), define
\[
R=5v_3^4-34v_3^3+73v_3^2-56v_3+16,\qquad
P=2v_3^2-7v_3+4.
\]
In the Symtropy coupling convention, the source expressions reduce to
\[
V(v_3)=\frac{v_2}{\alpha_{sym}}
=\frac{\sqrt R-P}{2-v_3},
\]
\[
U_1(v_3)=\frac{v_1}{\alpha_{sym}}
=\frac{(V+4-3v_3)(3V+4-v_3)}
{2(4-v_3)(3V+8-6v_3)}.
\]
The global charges/area/entropy scale as \(\alpha_{sym}^{3/2}\):
\[
J=\pi\alpha_{sym}^{3/2}\,
\frac{Vv_3\sqrt{(4-v_3)(V+4-3v_3)}}{4},
\]
\[
A_H=2\pi^2\alpha_{sym}^{3/2}V\sqrt{Vv_3},
\]
\[
S_{ext}=\frac{\pi^2}{2}\alpha_{sym}^{3/2}
\sqrt{Vv_3}(V+4-v_3).
\]
The near-horizon rotation coefficient is
\[
k=\frac{2U_1\sqrt{4-v_3}}
{\sqrt{Vv_3(V+4-3v_3)}}.
\]
A useful independent thermodynamic identity is
\[
\boxed{\frac{S_{ext}}{A_H}=\frac{V+4-v_3}{4V}}.
\]

## Exact positivity analysis

The source's square root is real throughout the closed interval because the degree-four radicand has strictly positive Bernstein coefficients on each half:
- on \(v_3\in[0,1]\): \(16,2,1/6,2,4\);
- on \(v_3\in[1,2]\): \(4,6,49/6,7,4\).

Further,
\[
R-P^2=v_3^2(v_3-4)(v_3-2)>0
\quad (0<v_3<2),
\]
which gives \(V>0\). The factor controlling the angular-momentum square root is
\[
V+4-3v_3=
\frac{v_3^2-3v_3+4+\sqrt R}{2-v_3}>0.
\]
The denominator factor in \(U_1\) is
\[
3V+8-6v_3=
\frac{v_3+4+3\sqrt R}{2-v_3}>0.
\]
For the remaining numerator factor,
\[
3V+4-v_3=
\frac{3\sqrt R-(5v_3^2-15v_3+4)}{2-v_3}>0.
\]
The last inequality follows by splitting at \(v_*=(15-\sqrt{145})/10\): below \(v_*\), the polynomial is positive and its square is strictly less than \(9R\); above \(v_*\), that polynomial is non-positive. Thus \(v_1>0\), and all displayed \(J,A_H,S_{ext}\) radicands are positive on the open parameter interval.

At \(v_3\to0\), \(v_1/\alpha_{sym}\to1/4\), i.e. \(v_1\to\alpha_{NH}\), while \(v_2,J,A_H,S_{ext}\to0\). This is the critical zero-area attractor endpoint in the source. The \(v_3=2\) endpoint is singular in this parameterization; it must be approached through a separate limiting procedure, not evaluated by direct substitution.

## Reproduction and use

Run:
~~~sh
python3 derive_extremal_equal_spin_attractor.py --output extremal_equal_spin_attractor_v1.json
~~~
The script checks the coupling conversion, exact BD lapse match, source-to-dimensionless reductions, Bernstein positivity, algebraic sign identities, entropy/area identity and critical endpoint limits. The fixture also contains 39 reference points from \(v_3=0.05\) through \(1.95\), spaced by 0.05.

## How this becomes a global solver benchmark

1. Use the attractor values to initialize a regular extremal inner patch.
2. Continue a non-extremal global branch toward zero temperature using a horizon-relative coordinate and regular variables.
3. Match \(v_1,v_2,v_3,k,A_H,S,J\) in a controlled overlap region.
4. Move the overlap radius and increase precision/resolution; the match must stabilize.
5. Verify the complete global radial EGB equations, radial constraint, asymptotic flatness and charges independently.

This is an inner boundary anchor, not a substitute for global integration. The paper's plotted existence region is based partly on extrapolated global numerical data; Symtropy should retain the distinction between exact near-horizon algebra and global branch evidence.

## Benchmark targets

- 100% pass on exact convention, lapse and attractor identity gates.
- Zero accepted wrong-coupling/branch/endpoint mutations.
- Near-horizon residual convergence with increasing expansion order.
- Global-to-attractor mismatch decreasing under continuation toward \(T_H\to0\).
- Overlap-radius stability and cross-chart invariance of horizon invariants.
- Independent first-law/Smarr residuals where global ADM data are available.
- Separate classifications for extremality, physical characteristic degeneracy, global branch termination, and numerical failure.

## Claim ceiling

This artifact is an exact local extremal attractor specialization plus symbolic positivity checks and a deterministic reference grid. It does not prove a corresponding global asymptotically-flat extremal spacetime, match any non-extremal Symtropy candidate, establish a full-domain existence theorem, or classify hyperbolicity.
