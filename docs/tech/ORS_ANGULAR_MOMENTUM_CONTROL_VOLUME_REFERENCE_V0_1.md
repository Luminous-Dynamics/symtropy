# ORS fixed-origin angular-momentum control-volume reference — v0.1

**Status:** bounded analytical/software reference; no physical qualification  
**Tracking:** Symtropy [ORS-ANGULAR-MOMENTUM-CONTROL-VOLUME-001 #5877](https://github.com/Luminous-Dynamics/symtropy/issues/5877)  
**Companion:** [linear momentum control-volume reference #5872](https://github.com/Luminous-Dynamics/symtropy/issues/5872)  
**Parent:** [ORS-DYNAMIC-SUPPORT-001 #5387](https://github.com/Luminous-Dynamics/symtropy/issues/5387)

## 1. Why this increment exists

The linear control-volume reference #5872 accounts for mass accumulation and advective linear momentum flux. It does not compute the moment of that momentum about an origin. Offset skyhook, rider and support interfaces require an angular-momentum balance with explicit port moment arms.

This increment keeps rotational bookkeeping separate rather than deriving torque after the fact from a scalar resultant and a guessed lever arm.

## 2. Fixed-origin balance and conventions

Choose one declared inertial frame, evaluation epoch and fixed inertial origin O. Each port binds its position relative to O, absolute inertial velocity, non-negative mass-flow magnitude and inflow/outflow orientation.

The port's specific angular momentum about O is:

~~~text
h_O = r_O x v
~~~

The advective angular-momentum flux is:

~~~text
Phi_H,O = sum_out(mdot * (r_O x v))
          - sum_in(mdot * (r_O x v))
~~~

The open-system balance is:

~~~text
tau_required_about_O = dH_O,CV/dt + Phi_H,O

tau_support_on_stream_about_O =
    tau_required_about_O - tau_other_external_about_O

tau_stream_on_support_about_O = -tau_support_on_stream_about_O
~~~

The code also computes linear momentum flux and required force so that the change of origin can be tested consistently. The “other” force and torque vectors are explicit, already-derived inputs; the crate does not infer gravity, EM force, stress, contact or structure terms.

## 3. Correct origin-translation identity

Torque about an origin is not invariant. Let O' = O + a, where a is a fixed vector in the same inertial frame. Then:

~~~text
r_O' = r_O - a
H_O' = H_O - a x P

dH_O'/dt = dH_O/dt - a x dP/dt

Phi_H,O' = Phi_H,O - a x Phi_P

tau_required_about_O' =
    tau_required_about_O - a x F_required

tau_other_about_O' = tau_other_about_O - a x F_other

tau_support_about_O' =
    tau_support_about_O - a x F_support
~~~

The frozen unsteady test shifts every port position by -a, transforms the stored angular-momentum derivative by -a x dP/dt and transforms the other torque by -a x F_other. The computed required torque, support torque and equal/opposite reaction must satisfy the corresponding identity.

This tests a nontrivial and frequently mishandled property: claiming that a torque vector is origin-invariant would be wrong. An arbitrary shift in origin also cannot be applied to the angular-momentum accumulation without the linear momentum derivative term.

## 4. Independent cases and metrics

The six-case, fourteen-port corpus covers radial-through flow, tangential flow, an off-axis turn, a 3D multi-port arrangement, unsteady storage and explicit external torque isolation.

Report separately:
- mass accumulation and in/out rates, plus mass residual (kg/s);
- linear momentum flux and required force vectors (N);
- angular-momentum flux and accumulation vectors (N·m);
- required external torque, support torque and equal/opposite reaction (N·m);
- origin-translation identity residual (N·m);
- orthogonal-rotation covariance residual;
- independent known-answer component and norm errors;
- deterministic receipt and port-order permutation results.

No composite “physics score” is used. Negative controls remain in the test denominator.

## 5. Composition and claim ceiling

- #5872 owns linear momentum and mass accumulation/flux.
- #5877 owns fixed-origin angular-momentum flux and torque.
- #5387 owns the eventual moving-stream/support transient qualification.
- #5496 owns model-family discrimination and experiment design.

This reference does not implement pressure/stress surface tractions, electromagnetic field angular momentum, Maxwell stress, structural force/torque transmission, flexible-ring gyroscopic response, moving control surfaces, moving origins, orbital equilibrium, stability or failure survivability. Omitted terms are not asserted to be physically zero.

A pass establishes only exact advective angular-momentum bookkeeping for the named fixed-origin mathematical cases. It does not validate a physical support law or architecture.
