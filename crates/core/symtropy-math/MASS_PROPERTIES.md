# Shape Mass Properties

The collision `Shape<D>` contract does not imply a unique mass distribution. A support map and bounding sphere describe collision geometry, but cannot determine internal density or second moments. Shapes that do not implement an analytic distribution therefore return `MassPropertiesError::UnsupportedShape`.

## Tensor convention

The API stores the centroidal mass second-moment tensor

```text
C = integral (x - c)(x - c)^T dm
```

in the shape's local coordinate frame. It is symmetric positive-semidefinite and has mass-times-length-squared units. It is not the classical 3D inertia tensor. The corresponding N-dimensional angular inertia action on a skew generator `Omega` is

```text
I_C(Omega) = C Omega + Omega C
```

A proper rotation `R` maps the centroidal tensor by `C_parent = R C_local R^T`; translation changes the centroid but not the centroidal tensor.

## Exact analytic distributions implemented

For a uniform-density solid D-ball of radius `r` and declared mass `m`:

```text
C = (m r^2 / (D + 2)) I
```

For a uniform-density axis-aligned solid hyperbox with half-extents `h_i`:

```text
C_ii = m h_i^2 / 3
C_ij = 0, i != j
```

These return local-frame centroid and tensor values. The caller supplies total mass; geometry alone does not imply density.

## Compound composition

For components with declared masses `m_k`, local centroidal tensors `C_k`, local-to-assembly rotations `R_k`, and transformed centroids `r_k`:

```text
c = sum(m_k r_k) / sum(m_k)
C_total = sum(R_k C_k R_k^T + m_k (r_k - c)(r_k - c)^T)
```

The helper validates component properties and proper rigid transforms before returning the composite mass properties.

## Explicit abstention

Capsules, convex hulls, meshes and custom shapes remain unsupported in this M1 change. A future implementation must provide a derived and tested volume/mass integral, with its distribution assumptions stated. No fallback to a bounding sphere, guessed radius, collider support vertices or nominal density is permitted.

## Promotion boundary

This is a geometry/mass-property source API only. It does not change `RigidBody`'s legacy principal-inertia vector, production `merge_toolhead`, angular integration, contact effective mass or joint response. Those changes require an explicit frame migration and integration of the full operator across the physics solver.
