# Synthetic known-truth reference oracle v0

**Status:** mathematical test fixture only; not a pavement, materials, FEM or WPT hardware model.  
**Parent:** [#3726 WPT-ASSET-HEALTH-001](https://github.com/Luminous-Dynamics/symtropy/issues/3726).

## Purpose and strict limitation

This file defines deterministic, hand-recomputable relationships for testing asset-observation ingestion, time-series linkage, drift detection, evidence propagation and uncertainty accounting.

The equations are deliberately low-order. They are not claimed to represent the true constitutive behaviour of asphalt, concrete, copper under all conditions, ferrite, insulation, water ingress, pavement fatigue or a specific WPT coil. Their parameters are declared fixture constants. A passing test establishes only that the evaluator handles the declared synthetic mapping correctly.

The model must never be used to forecast a real asset, authorize energization, derive safety limits or extrapolate service life.

## 1. Geometry-to-coupling fixture

Set:
- primary inductance (L_p=100\,\mu H);
- secondary inductance (L_s=100\,\mu H);
- baseline coupling coefficient (k_0=0.2);
- displacement scale (d_c=10\,mm);
- baseline relative displacement vector δ = (0,0,0) mm.

The synthetic displacement magnitude is:

\[
d=\sqrt{\delta_x^2+\delta_y^2+\delta_z^2}
\]

For this fixture only:

\[
k(d)=k_0\exp\left[-\frac{1}{2}\left(\frac{d}{d_c}\right)^2\right]
\]

\[
M(d)=k(d)\sqrt{L_pL_s}
\]

This produces:
- (d=0\,mm): (k=0.2), (M=20\,\mu H);
- (d=10\,mm): (k=0.1213061319), (M=12.13061319\,\mu H);
- (d=20\,mm): (k=0.0270670566), (M=2.70670566\,\mu H).

**Negative controls:** coordinate-frame mismatch, unit substitution (metres interpreted as millimetres), stale baseline geometry digest, and missing displacement uncertainty must be rejected or marked NOT_EVALUABLE. Do not use this Gaussian mapping as a universal physical coupling law.

## 2. Temperature-to-resistance fixture

Declare a synthetic copper-like conductor temperature coefficient α = 0.00393 per °C, reference temperature (T_0=20\,°C), and reference resistance (R_0=100\,m\Omega).

\[
R(T)=R_0[1+\alpha(T-T_0)]
\]

At (T=70\,°C), (R=119.6500\,m\Omega).

For a declared fixture frequency (f=85\,kHz) and fixed (L=100\,\mu H):

\[
Q(T)=\frac{2\pi fL}{R(T)}
\]

- (Q(20\,°C)=534.070751);
- (Q(70\,°C)=446.360845).

This fixture changes conductor resistance and Q only. It holds inductance and capacitance fixed, so it does not claim a thermal resonance-frequency shift. A separate measured or declared (L(T))/(C(T)) law is required to test resonant-frequency drift.

## 3. Moisture-to-insulation fixture

Let normalized synthetic moisture state (w\in[0,1]), with (R_{iso,0}=100\,M\Omega) and β=3:

\[
R_{iso}(w)=R_{iso,0}e^{-\beta w}
\]

- (w=0): (R_{iso}=100\,M\Omega);
- (w=1): (R_{iso}=4.97870684\,M\Omega).

This is a synthetic monotone response. The variable (w) is not a real moisture measurement and the output is not an insulation safety threshold.

## 4. Ferrite-loss fixture

For normalized synthetic damage (D\in[0,1]), define:

\[
Q_{ferrite}(D)=Q_{healthy}(1-0.5D)
\]

At (D=0.2), the declared synthetic Q is (0.9Q_{healthy}). This tests whether the evaluator preserves damage, Q and uncertainty as separate fields; it does not assert a measured ferrite degradation law.

## 5. Censoring and sensor-fault fixtures

### Right censoring

Define an observation end time (t_c) before a declared failure threshold is crossed. The record is RIGHT_CENSORED at (t_c); it is not a failure at (t_c). RUL scoring must use a censoring-aware method or report NOT_EVALUABLE where the method cannot support the claim.

### Sensor offset

The latent synthetic truth remains fixed while one sensor reports:

\[
y_{sensor}=y_{truth}+b
\]

with declared bias (b=+2\,\mu\epsilon). An independent reference channel remains unbiased. A correct evaluator flags the sensor disagreement; it must not infer pavement damage solely from the biased sensor.

### Missing calibration

Remove the calibration receipt for one measurement channel. The affected derived metric is NOT_EVALUABLE even when the raw numeric value appears plausible.

### Asset leakage

Create repeated time records for one asset and deliberately distribute them across train/test partitions. The split validator must reject the manifest.

## 6. Deterministic tests

A first evaluator should prove all of the following:
1. Recompute the displacement-to-(k)/(M) points to floating-point tolerance.
2. Recompute (R(T)) and (Q(T)) from the frozen constants.
3. Recompute the declared moisture and ferrite fixtures.
4. Detect inconsistent units, impossible (k\notin[0,1]), negative resistance and stale baseline identity.
5. Distinguish physical drift from a known sensor bias using the independent reference record.
6. Preserve missing calibration as NOT_EVALUABLE.
7. Preserve right censoring as censored rather than failure.
8. Reject asset/material-lot leakage across data partitions.
9. Keep all synthetic predictions tagged SYNTHETIC_KNOWN_TRUTH and prevent them from entering a measured/field evidence aggregate.

## Claim ceiling

These deterministic equations are evaluator controls only. They do not validate a material law, pavement lifetime model, coupling model, RUL model or energized WPT asset. Any transition from synthetic to physical inference requires independent material characterization, calibrated geometry/electrical measurements, held-out physical assets and dependence-aware uncertainty evaluation.
