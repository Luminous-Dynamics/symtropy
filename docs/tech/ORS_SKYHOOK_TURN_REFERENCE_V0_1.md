# Orbital-ring skyhook turning-force reference — v0.1

**Status:** historical analytic-reference reproduction; no physical qualification  
**Tracking:** Symtropy [ORS-DYNAMIC-SUPPORT-001 #5387](https://github.com/Luminous-Dynamics/symtropy/issues/5387)  
**Primary source:** Paul Birch, “Orbital Ring Systems and Jacob's Ladders — I,” *Journal of the British Interplanetary Society*, vol. 35 (1982), pp. 475–497, especially §3.1, Eqs. 32–56 and Table 4. [Source PDF](https://nss.org/wp-content/uploads/Orbital-Rings.pdf)

## 1. Why this increment exists

The current orbital-ring work already has a T0 fixed-circular-stream resultant, a source-separated power audit, a fail-closed load-step manifest, a two-mass synthetic transient integrator, and a PORS ballistic endpoint reference. Those are different analytical objects.

Birch's ORS model contains a further, independently reproducible mechanical event: the moving ring stream is turned at a skyhook. The stream is otherwise treated as free-falling. The local turn therefore has a direct momentum-rate reference before any controller, magnetic constitutive model, distributed ring, or flexible-structure solver is admitted.

This reference fills that analytical gap without promoting the historical design to a validated architecture.

## 2. Source derivation

Birch describes a minimum ORS as an orbital ring with skyhooks. Between skyhooks, the ring is in free fall; at a skyhook, its track changes through an angle DeltaTheta. For a ring stream with line density m and speed V0:

~~~text
Eq. 32: mass_flow = m * V0
Eq. 33: |DeltaV| ~= V0 * DeltaTheta
Eq. 35: F_T = mass_flow * |DeltaV|
             = m * V0^2 * DeltaTheta
~~~

Birch then relates the track-turn angle to the perigee drop DeltaH, the skyhook height H, Earth radius R, and half-angle alpha:

~~~text
Eq. 44: DeltaTheta ~= 4 * DeltaH / (alpha * (R + H))
~~~

Using the conic/orbit relation in Eq. 53:

~~~text
Eq. 53: V0^2 ~= g * R^2 /
              (R + H + DeltaH * (1 - 2 / alpha^2))
~~~

Combining Eqs. 35, 44 and 53 gives:

~~~text
Eq. 54:
F_T ~= 4 * m * g * R^2 * DeltaH
       / (alpha * (R + H)
          * (R + H + DeltaH * (1 - 2 / alpha^2)))
~~~

The independent reference evaluates both the momentum-rate path (Eq. 35 after Eqs. 44 and 53) and the direct algebraic expression (Eq. 54). It also checks Birch's stated geometric bound:

~~~text
Eq. 49: DeltaTheta <= 2 * alpha
~~~

A table dash is classified only as **outside the source model's stated domain**. It is not interpreted as a proof that a physical design is impossible.

## 3. Frozen source basis and results

Table 4 covers:

- Skyhook height H = 300 km and 600 km
- pi/alpha = 2, 4, 8, 16, 32 and 64
- Perigee drop DeltaH = 3 km, 30 km and 300 km

The paper gives:

- Ring line density: m = 2.5 * pi * 10^3 kg/m
- Surface gravity: g = 9.81 m/s^2
- Rounded Earth radius for this table: R = 6,400 km

For reproducing this *historical table*, the reference uses the paper's rounded R = 6,400 km basis, rather than silently replacing it with a modern geodetic radius. Modern WGS-84 constants belong in a separate re-derived model, not in the transcription check.

The executable corpus contains all 36 cells. Results:

- 28 numeric table cells reproduced to within 2% of the printed, two-decimal values.
- 8 dashes reproduced as out-of-domain cases under the source's angle bound / speed denominator.
- Maximum relative difference against the rounded numeric table: approximately 1.38%.
- Eq. 35 and Eq. 54 agree to floating-point precision for every admitted case.

The 2% threshold covers rounded source values and approximations in the derivation only; it is not a physical uncertainty budget.

## 4. Interpretation and next gate

This gives an independently inspectable load scale for one idealized skyhook turn. It does not supply the complete force path through a skyhook, its suspended ladder, the support/rider network, field momentum, or the structural ring. Nor does it give transient response, stability, or failure survivability.

The next model must keep separate:

1. the moving-stream momentum change at the turn;
2. the skyhook's own acceleration and load paths;
3. the equal-and-opposite interaction force and its electromagnetic implementation;
4. ring and support/rider momentum and energy boundaries;
5. local force distribution, flexure, and multi-skyhook/common-cause effects.

The 8 km/s figure used elsewhere in Birch's paper for drag estimates is not inserted as V0 in Table 4; the table's V0 is geometry-derived by Eq. 53. Likewise, this document does not fold the eddy-current and cryostat power numbers into the turning-force result.

**Claim ceiling:** arithmetic reproduction of Birch (1982) Eqs. 35/44/53/54 and Table 4. No claim is made about the accuracy of the 1982 model, the feasibility of an orbital ring, or the performance of any magnetic/electromagnetic support system.

## 5. Reproduction

See [the standalone Rust reference](../../tools/ors-skyhook-reference/README.md), frozen 36-case corpus, and focused workflow. The crate has no third-party dependencies and emits a deterministic receipt bound to the source commit and corpus SHA-256.
