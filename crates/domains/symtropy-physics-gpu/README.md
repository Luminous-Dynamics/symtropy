# symtropy-physics-gpu

Experimental Bevy/WGPU compute and rendering scaffolding for Symtropy physics research.

## Current capability boundary

**This crate is not yet a dispatched or qualified GPU physics backend.**

The current implementation:
- uploads selected collider/social/instance state;
- queues Bevy compute pipeline descriptors;
- creates storage buffers and a render-material/instancing scaffold;
- exposes an initial fluid-voxel data structure.

It does **not** currently:
- dispatch the queued `count_and_scatter`, `integrate`, or `apply_social_decay` pipelines;
- execute the `broadphase` entry point (the shader function is a stub);
- produce a canonical collision-candidate list;
- read back collision pairs or qualify them against a CPU oracle;
- perform general rigid-body contact resolution, constraint solving or XPBD dynamics.

The current `integrate` WGSL entry point is a bounded prototype with fixed gravity and a hard-coded y=0 bounce plane. It is not a general-purpose integrator.

## First qualification milestone

See [PHYS-GPU-DISPATCH-001 #4482](https://github.com/Luminous-Dynamics/symtropy/issues/4482), under parent [PHYS-GPU-001 #3866](https://github.com/Luminous-Dynamics/symtropy/issues/3866).

The first claim-bearing milestone is narrow: dispatch a GPU AABB broadphase, canonicalize its candidate pairs, and prove exact candidate-set equivalence against an independent CPU AABB oracle over frozen fixtures. It must preserve:
- zero false negatives;
- no self-pairs or duplicates after canonicalization;
- explicit output overflow and capacity status;
- stable canonical pair ordering despite atomic append scheduling;
- paired CPU/GPU receipts bound to exact input, shader, device and execution profile.

This is only broadphase candidate-set qualification. It does not validate contact solving or physical trajectories.

## Research policy

Do not advertise this crate as GPU physics acceleration until real compute dispatch and end-to-end measurements exist. Report upload, dispatch, readback, CPU fallback and synchronization costs separately. Compare with the native LBVH/CPU baseline at matched correctness, not just kernel-only time.

The Bevy 0.20 workspace migration updates the crate's Bevy dependencies and ShaderBuffer callsites. That compatibility work does not itself complete GPU dispatch or establish GPU physics correctness.
