# Ma–Lü equal-spin Myers–Perry parameter map

**Status:** exact kinematic relations and exact-symbolic series checks.

The published PDF Eq. (10) explicitly defines chi_a=a/mu and chi_b=b/mu. The square root inserted by an intermediate extraction was incorrect. For a scale-explicit comparison, use chi*r_h.

For equal spins, set q=a/r_h and nu_h=Omega_H*r_h. The MP horizon and angular-velocity equations give
\[
\mu/r_h^2=(1+q^2)^2/2,\qquad \nu_h=q/(1+q^2),\qquad \chi r_h=2q/(1+q^2)^2.
\]
The inverse physical-spin series is
\[
q=\nu_h+\nu_h^3+2\nu_h^5+5\nu_h^7+14\nu_h^9+42\nu_h^{11}+O(\nu_h^{13}),
\]
while the paper's parameter maps as
\[
\chi r_h=2\nu_h-2\nu_h^3-2\nu_h^5-4\nu_h^7-10\nu_h^9-28\nu_h^{11}+O(\nu_h^{13}).
\]

As an independent check, set y=chi*r_h. Inverting gives q=y/2+y^3/4+9y^5/32+..., and the exact horizon relation yields
\[
\mu/r_h^2=1/2+y^2/4+9y^4/32+O(y^6),
\]
matching Eq. (50) specialized to equal spins.

The first-coupling source identity gives alpha_R=alpha_GB/4 for the Ricci-flat MP background, or alpha_R/r_h^2=lambda/4 at the same horizon radius. This is not a finite-coupling equivalence.

The committed SymPy script checks the exact horizon and spin maps, inverse series through nu_h^11, Eq. (50) through y^4, and the action coefficient map. It does not claim a corrected metric comparison or finite-coupling EGB result.
