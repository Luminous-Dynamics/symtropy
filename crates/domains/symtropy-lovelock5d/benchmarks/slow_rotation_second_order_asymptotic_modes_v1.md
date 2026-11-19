# 5D EGB spin-squared asymptotic homogeneous modes

**Status:** exact symbolic indicial calculation in the Einstein limit (\(\alpha=0\)). The analysis consumes the homogeneous linear operator emitted by `derive_slow_rotation_second_order.py`. It is not a finite-coupling boundary-value solution.

**Owner:** [BH-5D-EGB-SPIN2-001 #6169](https://github.com/Luminous-Dynamics/symtropy/issues/6169).

## 1. Scope and scaling

Start from the action-derived, gauge-sliced second-order system
\[
E_q^{(2)}=\mathcal L_q[B_2,F_2,H_2]+\mathcal S_q[W_1^2]=0,
\qquad q\in\{b,f,g,h\}.
\]
For the asymptotic *homogeneous* mode calculation, set the known spin source to zero and take the exact Einstein limit \(\alpha=0\). Retain all four equations, especially \(E_g^{(2)}\), which is the radial constraint.

Use the weighted power ansatz
\[
B_2(r)=b\,r^p+o(r^p),\qquad
F_2(r)=f\,r^p+o(r^p),\qquad
H_2(r)=h\,r^{p+2}+o(r^{p+2}).
\]
The two-power offset is important: \(h\) is the coefficient perturbing an angular metric function whose background grows as \(r^2\). Treating all three functions as having the same radial power loses the correct indicial system.

## 2. Leading matrix

After dividing \(E_b,E_f\) by their leading scale \(r^{p+1}\), and \(E_g,E_h\) by \(r^{p-1}\), the four leading rows are
\[
M(p)\begin{pmatrix}b\\f\\h\end{pmatrix}=0,
\]
with
\[
M(p)=\begin{pmatrix}
0 & -\frac32(p+2) & -\frac12(p+2)(p+1)\\
\frac32p & 3 & p+1\\
-p(p+1) & -2(p+1) & -p^2-2p+2\\
-\frac12p(p+1) & -(p+1) & -3
\end{pmatrix}.
\]

The four maximal \(3\times3\) minors have common factor
\[
\boxed{p(p-2)(p+2)(p+4)}.
\]
The exact roots and one-dimensional amplitude directions are:

| Indicial power \(p\) | \((b,f,h)\) | Asymptotic interpretation |
|---:|---:|---|
| \(-4\) | \((0,1,1)\) | Decaying tail: \(F_2\sim r^{-4}\), \(H_2\sim r^{-2}\), with no \(r^{-4}\) term in \(B_2\) at this order |
| \(-2\) | \((1,1,0)\) | Mass-like mode: \(B_2\) and \(F_2\) share a \(r^{-2}\) coefficient; the leading \(H_2\sim r^0\) amplitude vanishes |
| \(0\) | \((1,0,0)\) | Constant lapse mode; removed by the asymptotic normalization \(b(\infty)=1\) |
| \(2\) | \((0,-1,1)\) | Growing mode; violates the asymptotically flat falloff |

An amplitude vector is defined only up to a nonzero overall factor.

## 3. Consequences for the finite-coupling BVP

1. The mass-like \(p=-2\) direction must be treated explicitly when distinguishing fixed-horizon-radius and fixed-ADM-energy parameterizations. It must not be silently removed by a normalization that changes the physical ensemble.
2. The decaying \(p=-4\) direction supplies an allowed homogeneous tail that must be included in the asymptotic matching data. Its relation to the asymptotic angular coefficient and physical charges must be established with the paper's charge conventions before assigning it a physical name.
3. The constant and growing directions are excluded by asymptotic normalization and flatness, respectively.
4. This is an indicial classification of the homogeneous Einstein-limit operator, not a complete Frobenius expansion. The inhomogeneous \(W_1^2\) source can generate particular falloffs at the same powers; those must be computed separately.
5. The finite-\(\alpha\) operator can change subleading coefficients and the horizon regularity problem. A finite-coupling mode calculation, followed by a coupled horizon-to-infinity BVP solve, remains required.

## 4. Reproduction

First emit the exact linear operators from the committed action derivation:

~~~sh
python3 derive_slow_rotation_second_order.py \
  --emit-equations \
  --output /tmp/slow_rotation_second_order_equations.json
~~~

Then calculate the indicial matrix and mode fixture:

~~~sh
python3 derive_slow_rotation_second_order_asymptotic_modes.py \
  --equations /tmp/slow_rotation_second_order_equations.json \
  --output /tmp/slow_rotation_second_order_asymptotic_modes_v1.json
~~~

The analyzer includes all four equations, calculates all four maximal minors, and checks the nullspace at each root. It is a research-only SymPy tool, not a production dependency.

## 5. Validation boundary

The analyzer was replayed locally with SymPy 1.14.0 against the equation JSON generated from the exact committed derivation script. The input JSON SHA-256 is recorded in `slow_rotation_second_order_asymptotic_modes_v1.json`. The exact rational matrix, maximal-minor common factor, four roots and four amplitude directions are checked by symbolic algebra.

No Rust compiler/test execution, finite-\(\alpha\) BVP, full-domain interval enclosure, or existence/uniqueness theorem is claimed.
