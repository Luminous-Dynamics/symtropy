# Coupled human–mechanics constraints and ideal ring scaling

**Status:** analytical screening and independent reproduction only. The Zhang 2026 source is a Research Square preprint, not peer-reviewed qualification evidence. None of the body-gradient or Coriolis parameters in this document is asserted to be a human safety threshold.

## 1. Source and evidence class

Yuzhan Zhang, Human constraints and scalable design of rotating artificial gravity habitats, Research Square version 1, published 8 September 2026, DOI 10.21203/rs.3.rs-10933822/v1.

The preprint's abstract states that:
- at 0.5 g, a 2% head–foot gravity-gradient limit implies at least 87.5 m radius;
- a 2 RPM reference implies 111.8 m radius;
- for a fixed moving mass, ideal spin tension is radius-independent while angular momentum scales as R^(3/2);
- when moving habitat mass scales with circumference, angular momentum scales as R^(5/2).

This work package independently reproduces the simple equations and provides executable reference functions. It does not reproduce or validate all the preprint's electromagnetic, rolling-contact, pressure-shell, finite-element or multiphysics results.

## 2. Constraint-intersection model

Let:
- a = g_target × g0 be target radial acceleration;
- omega be angular velocity in rad/s;
- N be rotation rate in RPM;
- L be the declared radial body length;
- epsilon be a caller-selected maximum ideal head-to-foot gradient fraction;
- v_perp be a declared transverse movement speed relative to the rotor;
- c be a caller-selected upper bound on Coriolis acceleration divided by target radial acceleration.

The basic rotation equations are:

\[
a = \omega^2 R,\qquad \omega = \frac{2\pi N}{60}
\]

### 2.1 Body-gradient bound

For an ideal body aligned radially, with its centre at R and radial length L:

\[
\frac{g_{feet}-g_{head}}{g_{COM}} = \frac{L}{R}
\]

Thus the design-input bound is:

\[
R_{gradient} = \frac{L}{\epsilon}
\]

This is a geometric descriptor. The selected epsilon is not a clinical threshold.

### 2.2 Maximum-RPM bound

For a declared maximum rotation rate N_max:

\[
R_{RPM} = \frac{a}{(2\pi N_{max}/60)^2}
\]

A smaller radius would exceed the specified RPM for the same target g.

### 2.3 Movement/Coriolis bound

For a velocity component perpendicular to the spin axis:

\[
|a_{Coriolis}| = 2\omega v_\perp
\]

If a user supplies the design-envelope constraint:

\[
\frac{2\omega v_\perp}{a} \le c
\]

then:

\[
\omega_{max,C} = \frac{ca}{2v_\perp},\qquad
R_{Coriolis} = \frac{4v_\perp^2}{c^2 a}
\]

For v_perp = 0, the movement-related Coriolis radius bound is inactive and represented as zero.

The c parameter is an engineering scenario knob only. It must never be presented as an evidence-based human comfort, nausea, injury or physiological tolerance limit.

### 2.4 Joint feasible radius

The three independent bounds are intersected by taking their maximum:

\[
R_{min} = \max(R_{gradient},R_{RPM},R_{Coriolis})
\]

The active constraint is reported separately. At the selected ideal radius:

\[
N_{actual} = \frac{60}{2\pi}\sqrt{\frac{a}{R_{min}}}
\]

\[
\epsilon_{actual} = \frac{L}{R_{min}},\qquad
c_{actual} = \frac{2v_\perp\sqrt{a/R_{min}}}{a}
\]

The constraints are deliberately kept separate so a Pareto search can explain which requirement sets the design size and which assumptions are driving the result.

## 3. Reproduction of the September 2026 preprint example

Inputs:
- target g = 0.5;
- body length L = 1.75 m;
- maximum ideal body gradient epsilon = 0.02;
- maximum RPM = 2.0;
- illustrative Coriolis fraction c = 0.10;
- transverse speed = 1.0 m/s.

Independent evaluation with g0 = 9.80665 m/s²:

| Constraint | Minimum radius |
|---|---:|
| Body gradient, 1.75 / 0.02 | 87.500000 m |
| 2 RPM at 0.5 g | 111.782406 m |
| Coriolis, c = 0.10 and v = 1 m/s | 81.577297 m |
| **Joint minimum** | **111.782406 m** |

The 2 RPM constraint is active. The resulting ideal body gradient is 1.56554%, and the resulting Coriolis ratio is 8.54275% of target radial acceleration. The 10% Coriolis value is merely the explicit input scenario used to exercise the function; it is not a human limit.

The first two rows reproduce the preprint's stated 87.5 m and 111.8 m intersection example. The third row is an additional Luminous parameterized screen, not a claim from that paper.

Reference fixture: fixtures/codesign_constraint_intersection.csv.

## 4. Ideal thin-ring scaling families

For a uniformly distributed, self-supporting thin circular ring with fixed total moving mass M, target acceleration a and radius R:

\[
\omega=\sqrt{a/R},\quad v=\sqrt{aR}
\]

\[
H = M R v = M\sqrt{a}R^{3/2}
\]

\[
E_{rot} = \frac{1}{2}Mv^2 = \frac{1}{2}MaR
\]

Using the ideal thin-ring membrane relation, tension at a single cut is:

\[
T = \frac{Ma}{2\pi}
\]

Consequently, at fixed M and a:
- angular momentum H scales as R^(3/2);
- rotational energy E scales as R;
- ideal hoop tension T is independent of R.

For fixed linear mass density lambda (kg/m), the total moving mass is M = 2 pi R lambda. Consequently:
- H scales as R^(5/2);
- E scales as R²;
- T scales as R.

These are distinct assumptions. A fixed moving mass and a fixed line density are not interchangeable. Neither scaling law provides a full structure mass, a stress allowable or a safety margin. The ideal equations omit pressure shells, habitation module mass, radiation shielding, joints, spokes, nonuniform equipment loads, bending, dynamics, fatigue, buckling, thermal effects, defects and manufacturing tolerances.

The executable reference functions return omega, tangential speed, angular momentum, spin energy and ideal hoop tension for the two separate mass bases.

Reference fixture: fixtures/ring_scaling_laws.csv.

The example receipt at fixtures/codesign_intersection_manifest.example.json records the input bounds and the active RPM constraint. The versioned AG_RUN_MANIFEST.schema.json includes typed design_constraints and a structured design_decision so a result can be reconstructed from its assumptions rather than only from its output geometry.

## 5. Reproducible executables

~~~bash
cargo run --release --manifest-path research/artificial-gravity-v0/Cargo.toml --bin coupled_design_space
cargo run --release --manifest-path research/artificial-gravity-v0/Cargo.toml --bin ring_scaling_matrix
~~~

The coupled design-space generator covers:
- target gravity: 0.16, 0.38, 0.5 and 1.0 g;
- ideal radial body gradient limits: 1%, 2% and 3%;
- maximum RPM assumptions: 1.6, 2.0 and 3.5;
- caller-selected Coriolis design fractions: 5% and 10%;
- transverse movement speeds: 0, 0.5, 1.0 and 1.5 m/s.

This is a 288-case deterministic assumption grid, not 288 human experiments or validated tolerance points.

The ring-scaling matrix evaluates fixed-total-mass and fixed-line-density families across four target gravity levels, four radii and two mass/density values.

## 6. Next modelling level

The next physical model must add the features that invalidate the thin-ring approximation:
- full ring/cylinder/annulus shell and structural member geometry;
- nonuniform pressure vessel and habitat module masses;
- equipment/cargo/crew mass maps and worst-case imbalance;
- discrete joints, spoke/truss/cable load paths and boundary conditions;
- shell membrane, bending, shear, buckling, fatigue and fracture;
- dynamic modes, damping, spin-up/down loads and transient torques;
- bearings, rotary interfaces, fluid/electrical/data transfer;
- radiation shielding, thermal systems, life support and fire compartments;
- assembly sequence, material processes, NDE, inspection and repair;
- uncertainty in geometry, materials, mass growth, joints and solver parameters.

The analytical oracle remains a limiting reference for the numerical model. It does not become a structural solver or certification tool.

## 7. Evidence and cross-project ownership

- Symtropy owns equations, reference functions, structural dynamics and simulation-to-test residuals.
- Symthaea may search parameterized designs, but must treat epsilon and c as user-supplied assumptions and abstain from inventing human tolerance data.
- Mycelix records exact source versions, code/model digests, fixtures, reproduction procedures and independent critique in its existing DeSci event kernel.
- Sol-Atlas provides occupied geometry, body/room layout, useful-volume definitions, construction sequence and maintainability constraints.

Cross-links: Symtropy #5279, #5280, #5289, #5291, #5292, #5295, #5314; Symthaea #12222; Mycelix #6204.
