//! Type-separated opaque identities.
//!
//! Construction authority belongs to product adapters. The core deliberately does
//! not prescribe UUID, hash, database, Bevy entity, or network identity schemes.
//!
//! Domain identity is enforced by the Rust type system rather than by convention:
//!
//! ```compile_fail
//! use symtropy_anima_core::{AgentId, PerceptId};
//!
//! fn require_agent(_: AgentId) {}
//!
//! let percept = PerceptId::from_bytes([0; 32]);
//! require_agent(percept);
//! ```

macro_rules! opaque_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; 32]);

        impl $name {
            /// Creates an ID from an adapter-provided stable byte token.
            #[must_use]
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            /// Returns the exact stable byte token.
            #[must_use]
            pub const fn into_bytes(self) -> [u8; 32] {
                self.0
            }

            /// Borrows the exact stable byte token.
            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }
    };
}

// Persistent entities and causal occurrences. These are intentionally distinct
// public types even though they share an opaque representation.
opaque_id!(/// Persistent embodied-agent identity.
    AgentId);
opaque_id!(/// Identity of a physical or communicative emission occurrence.
    EmissionId);
opaque_id!(/// Identity of a receptor-legitimate percept occurrence.
    PerceptId);
opaque_id!(/// Identity of a future-bearing experience occurrence.
    ExperienceId);
opaque_id!(/// Identity of a committed intent occurrence.
    IntentId);
opaque_id!(/// Identity of an action occurrence.
    ActionId);

// Content/config identities. Product adapters should bind these to exact
// version/content lineage rather than treating them as occurrence counters.
opaque_id!(/// Identity of an exact decision/update policy.
    PolicyId);
opaque_id!(/// Identity of an exact species/body/cognition profile.
    ProfileId);
opaque_id!(/// Identity of an exact aggregation method used to derive a sample window.
    AggregationMethodId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_ids_round_trip_exact_bytes() {
        let bytes = [0x5a; 32];
        let id = AgentId::from_bytes(bytes);
        assert_eq!(id.into_bytes(), bytes);
    }

    #[test]
    fn occurrence_and_content_ids_have_stable_ordering() {
        let lower = PolicyId::from_bytes([0; 32]);
        let upper = PolicyId::from_bytes([1; 32]);
        assert!(lower < upper);
    }

    #[test]
    fn ids_are_fixed_size_and_heap_free() {
        assert_eq!(core::mem::size_of::<AgentId>(), 32);
        assert_eq!(core::mem::size_of::<PolicyId>(), 32);
    }
}
