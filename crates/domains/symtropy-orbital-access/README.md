# symtropy-orbital-access

A standalone, dependency-free Rust reference for the idealized orbital plane produced by horizontal insertion from a uniformly rotating spherical Earth.

This crate is intentionally separate from Symtropy's Bevy orbital demo and from the full workspace. It is an analytic geometry contract for tests and downstream research, not a launch-vehicle or trajectory simulator.

## Conventions

- Latitude and longitude are radians.
- Launch azimuth is measured clockwise from local north in the Earth-fixed frame. Negative azimuths are west of north.
- Insertion speed is the vehicle's horizontal speed relative to the rotating surface, in m/s.
- Epoch is integer TAI seconds from the caller's declared epoch.
- Earth-fixed and inertial frame IDs are explicit strings.
- Earth is spherical and rotates uniformly about the inertial +Z axis.
- Inertial position uses the supplied Earth rotation angle at the reference epoch plus the declared constant rotation rate times elapsed epoch seconds.

For azimuth A, latitude phi, relative horizontal speed V, spherical radius R and spin rate omega:

- north velocity: v_N = V cos(A)
- east inertial velocity: v_E = V sin(A) + omega R cos(phi)
- inclination is derived from the inertial angular-momentum vector h = r cross v.

The implementation computes vectors in the named inertial frame and derives inclination from h. A scalar inclination relation is used only as an independent test cross-check.

## Limitations

No oblate-Earth gravity, precession/nutation, polar motion, atmospheric drag, ascent steering, finite burn, vehicle mass/thrust, staging, launch-window or range-safety model is included. The output cannot establish launch capability, site eligibility, payload performance or flight safety.
