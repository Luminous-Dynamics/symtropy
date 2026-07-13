// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Re-export of [`PlatformType`] from `symtropy-robotics-bridge-core`, plus
//! a conversion from `symthaea-core`'s [`EmbodimentPlatform`].
//!
//! This crate doesn't define its own platform enum — the canonical
//! definition lives in the permissively-licensed `-core` crate so it can be
//! shared without pulling AGPL code into consumers that only need the enum.
//! This module exists purely so callers can write
//! `symtropy_robotics_bridge::platform::PlatformType`, matching the
//! `agent::RoboticAgent` import convention used across the platform demos
//! and this crate's `examples/`.
//!
//! ## Why the conversion lives here, not in `-core`
//!
//! There are (as of 2026-07) **three** independently-defined platform-type
//! enums across the monorepo: `symthaea-core::embodiment::EmbodimentPlatform`
//! (canonical, ~20 variants), this crate's re-exported `PlatformType`
//! (~10 variants, game-engine facing), and a third in
//! `mycelix-civic`'s `robotics-dispatch` zome (not addressed here — see
//! `symthaea/NUCLEAR_ENERGY_PLAN_2026-07-06.md` Phase 4 for that scoping).
//! `symtropy-robotics-bridge-core` is **deliberately** Apache-2.0/MIT with
//! "no symthaea platform deps" (see its own crate doc) — depending on AGPL
//! `symthaea-core` there would be a licensing violation. This sibling crate
//! is already AGPL and already depends on the symthaea tree, so the
//! conversion lives here instead.
//!
//! Also note: `impl TryFrom<EmbodimentPlatform> for PlatformType` is not
//! possible here even though this crate is AGPL — Rust's orphan rule
//! requires the trait, or the `for` type, to be local to this crate, and
//! neither `TryFrom` (std), `PlatformType` (`-core`), nor `EmbodimentPlatform`
//! (`symthaea-core`) is. Hence a plain free function below instead of a
//! trait impl.

pub use symtropy_robotics_bridge_core::PlatformType;

use symthaea_core::embodiment::EmbodimentPlatform;

/// An [`EmbodimentPlatform`] with no equivalent [`PlatformType`] variant.
///
/// Carries the source variant so callers can decide what to do (log it,
/// fall back to a default, error out) rather than the conversion silently
/// discarding the information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedPlatform(pub EmbodimentPlatform);

impl std::fmt::Display for UnsupportedPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EmbodimentPlatform::{:?} has no symtropy_robotics_bridge_core::PlatformType equivalent",
            self.0
        )
    }
}

impl std::error::Error for UnsupportedPlatform {}

/// Map the canonical `symthaea-core` platform enum to this crate's
/// game-engine-facing one.
///
/// `EmbodimentPlatform` has ~20 variants (including non-physical ones like
/// `CareProvider`/`Browser`/`Phone`/`Desktop`, and newer physical platforms
/// like `Subterranean`/`Infrastructure`/`Scavenger`/`Agribot`/`Biota`/`Clime`
/// that `PlatformType` was never extended to cover); `PlatformType` has 10.
/// Every variant with no equivalent returns `Err(UnsupportedPlatform)`
/// naming exactly which one, rather than silently mapping to a default.
pub fn embodiment_platform_to_symtropy(
    platform: EmbodimentPlatform,
) -> Result<PlatformType, UnsupportedPlatform> {
    match platform {
        EmbodimentPlatform::Humanoid => Ok(PlatformType::Humanoid),
        EmbodimentPlatform::Quadrotor => Ok(PlatformType::Quadrotor),
        EmbodimentPlatform::Vehicle => Ok(PlatformType::Vehicle),
        EmbodimentPlatform::Helicopter => Ok(PlatformType::Helicopter),
        EmbodimentPlatform::Auv => Ok(PlatformType::Auv),
        EmbodimentPlatform::Manipulator => Ok(PlatformType::Manipulator),
        EmbodimentPlatform::Exoskeleton => Ok(PlatformType::Exoskeleton),
        EmbodimentPlatform::Surgical => Ok(PlatformType::Surgical),
        EmbodimentPlatform::Orbital => Ok(PlatformType::Orbital),
        EmbodimentPlatform::Quadruped => Ok(PlatformType::Quadruped),
        // No equivalent in PlatformType today. Listed explicitly (not a
        // wildcard `_`) so adding a new EmbodimentPlatform variant forces a
        // conscious decision here instead of silently falling through.
        EmbodimentPlatform::None
        | EmbodimentPlatform::Subterranean
        | EmbodimentPlatform::Infrastructure
        | EmbodimentPlatform::Scavenger
        | EmbodimentPlatform::Agribot
        | EmbodimentPlatform::Biota
        | EmbodimentPlatform::Clime
        | EmbodimentPlatform::CareProvider
        | EmbodimentPlatform::Browser
        | EmbodimentPlatform::Phone
        | EmbodimentPlatform::Desktop
        | EmbodimentPlatform::Detritivore => Err(UnsupportedPlatform(platform)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_symtropy_platforms_have_a_source_variant() {
        // Every PlatformType variant must be reachable from some
        // EmbodimentPlatform — if this fails, either PlatformType grew a
        // variant this map doesn't produce, or the map has a typo.
        let mut reached = std::collections::HashSet::new();
        for p in [
            EmbodimentPlatform::Humanoid,
            EmbodimentPlatform::Quadrotor,
            EmbodimentPlatform::Vehicle,
            EmbodimentPlatform::Helicopter,
            EmbodimentPlatform::Auv,
            EmbodimentPlatform::Manipulator,
            EmbodimentPlatform::Exoskeleton,
            EmbodimentPlatform::Surgical,
            EmbodimentPlatform::Orbital,
            EmbodimentPlatform::Quadruped,
        ] {
            if let Ok(mapped) = embodiment_platform_to_symtropy(p) {
                reached.insert(mapped);
            }
        }
        for expected in PlatformType::ALL {
            assert!(
                reached.contains(&expected),
                "{expected:?} unreachable from EmbodimentPlatform"
            );
        }
    }

    #[test]
    fn test_unsupported_platforms_return_named_error() {
        for p in [
            EmbodimentPlatform::None,
            EmbodimentPlatform::Subterranean,
            EmbodimentPlatform::Infrastructure,
            EmbodimentPlatform::Scavenger,
            EmbodimentPlatform::Agribot,
            EmbodimentPlatform::Biota,
            EmbodimentPlatform::Clime,
            EmbodimentPlatform::CareProvider,
            EmbodimentPlatform::Browser,
            EmbodimentPlatform::Phone,
            EmbodimentPlatform::Desktop,
            EmbodimentPlatform::Detritivore,
        ] {
            let err = embodiment_platform_to_symtropy(p).unwrap_err();
            assert_eq!(err.0, p);
            assert!(!err.to_string().is_empty());
        }
    }

    #[test]
    fn test_round_trip_preserves_actuator_semantics() {
        // Spot check: humanoid should still map to a humanoid, not silently
        // to something else due to enum-ordering drift between the crates.
        assert_eq!(
            embodiment_platform_to_symtropy(EmbodimentPlatform::Humanoid),
            Ok(PlatformType::Humanoid)
        );
        assert_eq!(
            embodiment_platform_to_symtropy(EmbodimentPlatform::Manipulator),
            Ok(PlatformType::Manipulator)
        );
    }
}
