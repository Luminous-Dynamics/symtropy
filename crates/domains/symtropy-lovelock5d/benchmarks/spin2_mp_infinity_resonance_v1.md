# Einstein-limit infinity recurrence: the first resonance is compatible

**Status:** exact result for the frozen-coefficient Einstein-limit asymptotic system through \(x^2\), \(x=1/r\). This advances the leading indicial polynomial to a first recurrence compatibility check.

## 1. Asymptotic equation

Use
\[
\tau=\log r,\qquad x=e^{-\tau}=1/r,
\qquad V=(\widehat B_2,r\widehat B_2',\widehat H_2,r\widehat H_2')^T.
\]
The local action-specialized reduction gives
\[
-x\frac{dV}{dx}=(A_0+xA_1+x^2A_2+O(x^3))V+
(g_0+xg_1+x^2g_2+O(x^3)),
\]
where
\[
A_0=\begin{pmatrix}
0&1&0&0\\0&-2&0&0\\0&0&0&1\\0&0&8&2
\end{pmatrix},\quad A_1=0,
\]
\[
A_2=\begin{pmatrix}
0&0&0&0\\0&0&0&0\\0&0&0&0\\0&0&12&-2
\end{pmatrix},\qquad g_0=g_1=g_2=0.
\]
The characteristic polynomial is \(\sigma(\sigma-4)(\sigma+2)^2\): two \(r^{-2}\) modes, one constant mode, and one \(r^4\) growing mode. The repeated \(-2\) eigenvalue is semisimple at leading order.

## 2. Recurrence and resonance

For \(V=\sum_{n\ge0}v_nx^n\),
\[
(-nI-A_0)v_n=\sum_{k=1}^nA_kv_{n-k}+g_n.
\]

A bounded branch begins with the constant eigenmode \(v_0=c e_B\), \(e_B=(1,0,0,0)^T\). At \(n=1\), \(-I-A_0\) is invertible (determinant \(-5\)) and the right-hand side vanishes, so \(v_1=0\).

At \(n=2\), \(-2I-A_0\) is singular, with rank and nullity both two. This is the first resonance with the two decaying \(r^{-2}\) modes. However,
\[
A_2e_B+g_2=0.
\]
The compatibility conditions are therefore satisfied exactly. No logarithm is forced at this order. The coefficient \(v_2\) can contain both independent decaying modes:
\[
v_2=c_B(1,-2,0,0)^T+c_H(0,0,1,-2)^T.
\]

Thus, at this order, the bounded local asymptotic family has one constant-mode amplitude and two independent decay amplitudes. After fixing the constant \(Bhat2\) normalization, two decaying coefficients remain.

## 3. Consequence for the global BVP

The horizon analysis gives two free regular Einstein-limit horizon data after \(Bhat2(1)=0\). The asymptotic analysis gives two decaying coefficients after removing the constant normalization and growing \(r^4\) mode.

This balanced \(2\)-versus-\(2\) count motivates a \(2\times2\) global boundary-data map. It does **not** prove that the map has full rank, that a solution exists for arbitrary data, or that the BVP is unique. Those remain global questions.

The full asymptotic recurrence beyond \(x^2\), source/response conventions, the fixed-charge versus fixed-horizon ensemble, finite-coupling infinity modes, and Fredholm index remain open.

## 4. Evidence boundary

The local action-specialized reduction was checked against the exact Myers–Perry solution over the full radial domain. The committed fixture/verifier independently check the \(n=0,1,2\) recurrences, the two-dimensional nullspace, and the bounded-branch compatibility using exact rational arithmetic. This is not a complete asymptotic solution, a global BVP theorem, an interval certificate or a finite-coupling result.
