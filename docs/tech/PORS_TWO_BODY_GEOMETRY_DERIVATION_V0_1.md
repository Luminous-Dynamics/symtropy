# PORS Two-Body Paired-Terminal Geometry — Derivation v0.1

**Status:** analytic reference derivation; not a PORS system design  
**Implementation:** `tools/space-economy-bench/examples/pors_geometry_reference.rs`  
**Golden fixtures:** `docs/tech/pors-two-body-geometry-corpus-v0.1.json`  
**Architecture issue:** [ORBITAL-TERMINAL-GEOMETRY-001 #3247](https://github.com/Luminous-Dynamics/symtropy/issues/3247)

## 1. Scope and interpretation

This note derives a symmetric equatorial two-body conic segment between two points on a spherical Earth. The segment reaches an apex height (H), where radial velocity is zero, and returns to the same geocentric radius (R) after a central-angle displacement (2\alpha).

This is a **kinematic reference problem only**. It is not a complete PORS model, not a launch-loop model, and not a structural or propulsion model. It assumes:
- spherical Earth with constant gravitational parameter \(\mu\);
- equatorial plane;
- central inverse-square gravity;
- no atmospheric drag, lift, winds, J2, third-body force or guidance force;
- a symmetric ballistic segment;
- a fixed Earth rotation rate \(\omega_E\);
- terminal sites attached to the rotating Earth.

The osculating ellipse's mathematical continuation beyond the selected exterior arc would pass through Earth. That continuation is not part of the system: the segment starts and ends at the surface terminals. This is still a valid way to derive the exterior conic arc and endpoint state.

Birch's PORS concept uses a moving particle stream or continuous cable along a sub-orbital arc, paired with endpoint reversal and a return stream. The equations here do not reproduce the mechanical stream, cable or terminal design. They isolate the idealized two-body geometry and the consequences for terminal separation and velocity. See Birch, *Orbital Ring Systems and Jacob's Ladders — I* (1982): https://nss.org/wp-content/uploads/Orbital-Rings.pdf.

## 2. Geometry and conic

Let:
- \(R\) be the reference Earth radius;
- \(H>0\) be the arc apex altitude;
- \(r_a=R+H\) be the apex geocentric radius;
- \(\alpha\in(0,90^\circ)\) be the half central angle from apex to either ground terminal;
- \(e\) be conic eccentricity;
- \(a\) be semi-major axis;
- \(p=a(1-e^2)\) be semi-latus rectum.

Set true anomaly \(f=\pi\) at the apex, so the apex is the apoapsis. The conic equation is

\[
r(f)=\frac{p}{1+e\cos f}.
\]

At the apex, \(r_a=p/(1-e)\). At either endpoint, \(f=\pi\pm\alpha\), so \(R=p/(1-e\cos\alpha)\).

Equating these two expressions gives

\[
e=\frac{r_a-R}{r_a-R\cos\alpha}
 =\frac{H}{H+R(1-\cos\alpha)}.
\]

Then

\[
p=r_a(1-e),\qquad
a=\frac{r_a}{1+e}.
\]

For \(H>0\) and \(0<\alpha<90^\circ\), this reference produces \(0<e<1\), i.e. an elliptic conic segment. The code independently verifies that the endpoint radius recovered from \(p/(1-e\cos\alpha)\) equals \(R\) within tolerance.

## 3. Speed at apex and endpoints

Using the vis-viva equation,

\[
v^2=\mu\left(\frac{2}{r}-\frac{1}{a}\right),
\]

the apex speed is

\[
v_a=\sqrt{\frac{\mu(1-e)}{r_a}},
\]

and the endpoint inertial speed magnitude is

\[
v_g=\sqrt{\mu\left(\frac{2}{R}-\frac{1}{a}\right)}.
\]

The flight-path angle magnitude at a ground endpoint, relative to the local horizontal, is

\[
\gamma=\operatorname{atan2}
\left(e\sin\alpha,1-e\cos\alpha\right).
\]

At the arrival endpoint, the radial velocity is inward:

\[
v_r=-v_g\sin\gamma.
\]

The inertial tangential velocity is \(s v_g\cos\gamma\), where \(s=+1\) for prograde and \(s=-1\) for retrograde.

## 4. Keplerian transfer time

At the first endpoint, the true anomaly is \(f_1=\pi-\alpha\). Convert true anomaly to eccentric anomaly:

\[
\cos E_1=\frac{e+\cos f_1}{1+e\cos f_1},\qquad
\sin E_1=\frac{\sqrt{1-e^2}\sin f_1}{1+e\cos f_1}.
\]

The symmetric second endpoint has \(E_2=2\pi-E_1\). The mean anomaly is

\[
M=E-e\sin E,
\]

and mean motion is

\[
n=\sqrt{\frac{\mu}{a^3}}.
\]

Thus the two-terminal flight time is

\[
\Delta t=\frac{(E_2-e\sin E_2)-(E_1-e\sin E_1)}{n}.
\]

This is a two-body time of flight. It is not a launch/reversal schedule and does not include endpoint dwell or transfer machinery.

## 5. Earth-fixed endpoint separation

The inertial central-angle displacement has signed value \(s\,2\alpha\). During the flight, Earth rotates by

\[
\Delta\lambda_E=\omega_E\Delta t.
\]

For equatorial terminals, the signed Earth-fixed longitude difference required to meet the second rotating endpoint is therefore

\[
\Delta\lambda_{\text{fixed}}=s\,2\alpha-\omega_E\Delta t.
\]

The reference reports the unwrapped signed longitude separation and the corresponding equatorial surface arc length

\[
d=R\left|\Delta\lambda_{\text{fixed}}\right|.
\]

This is why the endpoints must be selected jointly. The inertial arc angle is not the same as the ground-fixed site separation. The current fixture is equatorial; non-equatorial endpoint vectors and general orbital planes must use the frame/epoch conventions in [ORBITAL-LAUNCH-GEOMETRY-001 #3206](https://github.com/Luminous-Dynamics/symtropy/issues/3206) and [reference frames #98](https://github.com/Luminous-Dynamics/symtropy/issues/98).

## 6. Terminal-relative velocity and kinetic-energy indicator

At the equator, the surface moves eastward at \(\omega_E R\). The arrival velocity relative to the rotating ground is

\[
v_{t,\mathrm{rel}}=s v_g\cos\gamma-\omega_E R,
\]

\[
v_{r,\mathrm{rel}}=-v_g\sin\gamma.
\]

The resultant relative speed and specific kinetic energy are

\[
v_{\mathrm{rel}}=\sqrt{v_{t,\mathrm{rel}}^2+v_{r,\mathrm{rel}}^2},
\qquad
e_{k,\mathrm{rel}}=\frac12v_{\mathrm{rel}}^2.
\]

The last quantity is an **interface-energy indicator per kilogram of moving stream mass**. It is not a prediction of dissipated energy, motor power, energy recovery, cable stress or terminal thermal load. Those require mass flow, capture/reversal duration, acceleration distance, motor efficiency, elasticity, magnetic field and control models.

## 7. Conservation checks

The implementation verifies that the apex and endpoint have equal specific orbital energy:

\[
\varepsilon_a=\frac12v_a^2-\frac{\mu}{r_a},\qquad
\varepsilon_g=\frac12v_g^2-\frac{\mu}{R}.
\]

It also verifies angular momentum per unit mass:

\[
h_a=r_a v_a,\qquad
h_g=R v_g\cos\gamma.
\]

Both differences must remain within the numerical tolerances declared in the Rust reference. These are internal two-body closure checks, not validation against a physical PORS demonstrator.

## 8. Illustrative fixture

For:
- \(R=6{,}371{,}000\) m;
- \(\mu=3.986004418\times10^{14}\) m\(^3\)/s\(^2\);
- \(\omega_E=7.292115\times10^{-5}\) rad/s;
- \(H=300{,}000\) m;
- \(\alpha=5^\circ\);
- prograde motion;

the golden reference reports approximately:
- flight time: **534.253 s**;
- inertial central arc: **10.000°**;
- Earth rotation during flight: **2.232°**;
- Earth-fixed endpoint separation: **7.768°**, approximately **864 km** along the equator;
- endpoint inertial speed: **3.177 km/s**;
- terminal-relative tangential velocity: **+1.749 km/s**;
- terminal-relative inward radial velocity: **−2.280 km/s**;
- resultant terminal-relative speed: **2.873 km/s**;
- terminal-relative kinetic energy: **4.127 MJ/kg** of stream mass.

These values are analytic consequences of the stated simplified model. They are **not** a real location recommendation, an attainable PORS profile, a net energy demand, or proof of system feasibility.

## 9. Explicit next model tiers

Do not skip from this reference to geographic siting. The next qualification tiers are:
1. 3D endpoint vectors and Earth-fixed/inertial frame/epoch coupling using #98/#3206.
2. Earth oblateness/J2 and non-equatorial ballistic boundary-value cases.
3. Atmospheric boundary, winds, dynamic pressure and weather constraints along the arc and terminal approach.
4. Stream/cable/particle dynamics, continuous mass-flow and tension/deflection envelopes.
5. Evacuated-sheath integrity, support spacing, alignment, vibration and thermal environment.
6. Magnetic acceleration, guidance, capture, reversal and energy recovery; include transient and power-failure states.
7. Terminal containment, rescue/egress, fragment release, and common-mode fault analysis.
8. Environmental, airspace, maritime, sovereignty and community overlays in Sol-Atlas.
9. Full lifecycle supply chain, bootstrap mass, power and cost in #3228 / #1765.
10. Independent physical, materials and safety qualification before any real-world claim.

The reference exists to find mathematical impossibilities and hidden terminal burdens early—not to make a speculative architecture appear ready.
