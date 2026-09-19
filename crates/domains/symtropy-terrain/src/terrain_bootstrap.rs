// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Representation-only Terrain bootstrap manifest v1.
//!
//! This module freezes canonical bytes for a Terrain lineage plus durable chunk
//! references and exact lattice loci. It does **not** authorize a world,
//! construct `EarthChunkLatticeLocus`, mutate ECS state, select physical metric,
//! or place Terrain into any external coordinate frame.
//!
//! A later bootstrap-authority tranche may wrap a validated candidate in a
//! stronger non-Serde/non-reflective authority type. This module intentionally
//! provides no such promotion.

use core::fmt;
use std::collections::BTreeSet;

use crate::geometry_kernel::EarthChunkLatticeCoord;

/// Canonical schema for the first Terrain bootstrap candidate representation.
pub const TERRAIN_BOOTSTRAP_SCHEMA_VERSION: u32 = 1;

/// Maximum number of explicit chunk assignments admitted by the v1 candidate.
///
/// This is an operational/canonicalization bound, not a claim about total world
/// size. Larger worlds should use a versioned hierarchical/bootstrap successor.
pub const MAX_TERRAIN_BOOTSTRAP_CHUNKS: usize = 1_048_576;

const TERRAIN_BOOTSTRAP_DOMAIN: &[u8] = b"symtropy.terrain.bootstrap.candidate.v1\0";
const TERRAIN_LINEAGE_ID_BYTES: usize = 32;
const TERRAIN_CHUNK_REF_BYTES: usize = 32;
const TERRAIN_BOOTSTRAP_ENTRY_BYTES: usize = TERRAIN_CHUNK_REF_BYTES + 12;

/// Declares only the *representation source class* of the external lineage ref.
///
/// Neither variant is authority by itself. In particular,
/// `ExternalWorldLineageV1` means only that the 32-byte identity is intended to
/// reference some separately qualified world-lineage system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TerrainLineageProfileV1 {
    LocalExplicitV1,
    ExternalWorldLineageV1,
}

impl TerrainLineageProfileV1 {
    pub const fn code(self) -> u8 {
        match self {
            Self::LocalExplicitV1 => 1,
            Self::ExternalWorldLineageV1 => 2,
        }
    }
}

/// Opaque exact reference to the external lineage Terrain is intended to join.
///
/// This is representation only. A non-zero ID and a supported profile do not
/// establish that the named lineage is trusted, current, persistent, or real.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerrainLineageRefV1 {
    profile: TerrainLineageProfileV1,
    identity: [u8; TERRAIN_LINEAGE_ID_BYTES],
}

impl TerrainLineageRefV1 {
    pub fn new(
        profile: TerrainLineageProfileV1,
        identity: [u8; TERRAIN_LINEAGE_ID_BYTES],
    ) -> Result<Self, TerrainBootstrapError> {
        if identity.iter().all(|byte| *byte == 0) {
            return Err(TerrainBootstrapError::ZeroLineageIdentity);
        }
        Ok(Self { profile, identity })
    }

    pub const fn profile(self) -> TerrainLineageProfileV1 {
        self.profile
    }

    pub const fn identity(self) -> [u8; TERRAIN_LINEAGE_ID_BYTES] {
        self.identity
    }
}

/// Durable Terrain-local chunk reference used by bootstrap representation.
///
/// This is deliberately not a Bevy `Entity`, mesh/collider handle, allocation
/// index, or lattice coordinate. Persistence semantics are a later theorem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerrainChunkRefV1([u8; TERRAIN_CHUNK_REF_BYTES]);

impl TerrainChunkRefV1 {
    pub fn new(identity: [u8; TERRAIN_CHUNK_REF_BYTES]) -> Result<Self, TerrainBootstrapError> {
        if identity.iter().all(|byte| *byte == 0) {
            return Err(TerrainBootstrapError::ZeroChunkIdentity);
        }
        Ok(Self(identity))
    }

    pub const fn bytes(self) -> [u8; TERRAIN_CHUNK_REF_BYTES] {
        self.0
    }
}

/// One representation-level chunk-to-lattice assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerrainChunkBootstrapEntryV1 {
    chunk_ref: TerrainChunkRefV1,
    lattice_locus: EarthChunkLatticeCoord,
}

impl TerrainChunkBootstrapEntryV1 {
    pub const fn new(
        chunk_ref: TerrainChunkRefV1,
        lattice_locus: EarthChunkLatticeCoord,
    ) -> Self {
        Self {
            chunk_ref,
            lattice_locus,
        }
    }

    pub const fn chunk_ref(self) -> TerrainChunkRefV1 {
        self.chunk_ref
    }

    pub const fn lattice_locus(self) -> EarthChunkLatticeCoord {
        self.lattice_locus
    }
}

/// Validated, canonical-order Terrain bootstrap candidate.
///
/// **This is not authority.** It cannot install a live lattice locus and carries
/// no trust/currentness/persistence proof. It exists so later authority code has
/// one stable, independently testable representation to bind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainBootstrapCandidateV1 {
    schema_version: u32,
    lineage: TerrainLineageRefV1,
    entries: Vec<TerrainChunkBootstrapEntryV1>,
}

impl TerrainBootstrapCandidateV1 {
    pub fn new(
        lineage: TerrainLineageRefV1,
        mut entries: Vec<TerrainChunkBootstrapEntryV1>,
    ) -> Result<Self, TerrainBootstrapError> {
        if entries.len() > MAX_TERRAIN_BOOTSTRAP_CHUNKS {
            return Err(TerrainBootstrapError::TooManyChunks {
                actual: entries.len(),
                maximum: MAX_TERRAIN_BOOTSTRAP_CHUNKS,
            });
        }

        // Canonicalize first so duplicate failure selection is independent of
        // caller insertion order.
        entries.sort_unstable_by(|left, right| {
            left.chunk_ref
                .cmp(&right.chunk_ref)
                .then_with(|| left.lattice_locus.cmp(&right.lattice_locus))
        });

        let mut chunk_refs = BTreeSet::new();
        let mut loci = BTreeSet::new();
        for entry in &entries {
            if !chunk_refs.insert(entry.chunk_ref) {
                return Err(TerrainBootstrapError::DuplicateChunkRef(
                    entry.chunk_ref,
                ));
            }
            if !loci.insert(entry.lattice_locus) {
                return Err(TerrainBootstrapError::DuplicateLatticeLocus(
                    entry.lattice_locus,
                ));
            }
        }

        Ok(Self {
            schema_version: TERRAIN_BOOTSTRAP_SCHEMA_VERSION,
            lineage,
            entries,
        })
    }

    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub const fn lineage(&self) -> TerrainLineageRefV1 {
        self.lineage
    }

    pub fn entries(&self) -> &[TerrainChunkBootstrapEntryV1] {
        &self.entries
    }

    /// Serializer-independent exact v1 bytes.
    ///
    /// Grammar:
    ///
    /// ```text
    /// domain
    /// || u32le(schema_version)
    /// || u8(lineage_profile)
    /// || lineage_identity[32]
    /// || u32le(entry_count)
    /// || repeated canonical entries:
    ///      chunk_ref[32]
    ///      || i32le(lattice_x)
    ///      || i32le(lattice_y)
    ///      || i32le(lattice_z)
    /// ```
    ///
    /// Entries are always sorted by durable chunk reference and duplicate chunk
    /// refs/loci were rejected before construction.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(
            TERRAIN_BOOTSTRAP_DOMAIN.len()
                + 4
                + 1
                + TERRAIN_LINEAGE_ID_BYTES
                + 4
                + self.entries.len() * TERRAIN_BOOTSTRAP_ENTRY_BYTES,
        );
        bytes.extend_from_slice(TERRAIN_BOOTSTRAP_DOMAIN);
        bytes.extend_from_slice(&self.schema_version.to_le_bytes());
        bytes.push(self.lineage.profile.code());
        bytes.extend_from_slice(&self.lineage.identity);
        bytes.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());

        for entry in &self.entries {
            bytes.extend_from_slice(&entry.chunk_ref.0);
            bytes.extend_from_slice(&entry.lattice_locus.x.to_le_bytes());
            bytes.extend_from_slice(&entry.lattice_locus.y.to_le_bytes());
            bytes.extend_from_slice(&entry.lattice_locus.z.to_le_bytes());
        }

        bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerrainBootstrapError {
    ZeroLineageIdentity,
    ZeroChunkIdentity,
    TooManyChunks { actual: usize, maximum: usize },
    DuplicateChunkRef(TerrainChunkRefV1),
    DuplicateLatticeLocus(EarthChunkLatticeCoord),
}

impl fmt::Display for TerrainBootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLineageIdentity => {
                f.write_str("Terrain bootstrap lineage identity must be non-zero")
            }
            Self::ZeroChunkIdentity => {
                f.write_str("Terrain bootstrap chunk identity must be non-zero")
            }
            Self::TooManyChunks { actual, maximum } => write!(
                f,
                "Terrain bootstrap contains {actual} chunks, exceeding v1 maximum {maximum}"
            ),
            Self::DuplicateChunkRef(chunk_ref) => {
                write!(f, "duplicate Terrain bootstrap chunk reference {chunk_ref:?}")
            }
            Self::DuplicateLatticeLocus(locus) => {
                write!(f, "duplicate Terrain bootstrap lattice locus {locus:?}")
            }
        }
    }
}

impl std::error::Error for TerrainBootstrapError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id32(first: u8) -> [u8; 32] {
        let mut bytes = [0_u8; 32];
        bytes[0] = first;
        bytes
    }

    fn lineage(first: u8) -> TerrainLineageRefV1 {
        TerrainLineageRefV1::new(TerrainLineageProfileV1::LocalExplicitV1, id32(first))
            .expect("test lineage must be valid")
    }

    fn chunk(first: u8) -> TerrainChunkRefV1 {
        TerrainChunkRefV1::new(id32(first)).expect("test chunk ref must be valid")
    }

    #[test]
    fn zero_identities_fail_closed() {
        assert_eq!(
            TerrainLineageRefV1::new(
                TerrainLineageProfileV1::LocalExplicitV1,
                [0_u8; 32],
            ),
            Err(TerrainBootstrapError::ZeroLineageIdentity)
        );
        assert_eq!(
            TerrainChunkRefV1::new([0_u8; 32]),
            Err(TerrainBootstrapError::ZeroChunkIdentity)
        );
    }

    #[test]
    fn entry_insertion_order_cannot_change_candidate_or_bytes() {
        let a = TerrainChunkBootstrapEntryV1::new(
            chunk(1),
            EarthChunkLatticeCoord::new(-4, 2, 8),
        );
        let b = TerrainChunkBootstrapEntryV1::new(
            chunk(2),
            EarthChunkLatticeCoord::new(7, -3, 9),
        );

        let first = TerrainBootstrapCandidateV1::new(lineage(7), vec![a, b]).unwrap();
        let second = TerrainBootstrapCandidateV1::new(lineage(7), vec![b, a]).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.canonical_bytes(), second.canonical_bytes());
        assert_eq!(first.entries(), &[a, b]);
    }

    #[test]
    fn duplicate_chunk_ref_fails_before_candidate_exists() {
        let first = TerrainChunkBootstrapEntryV1::new(
            chunk(3),
            EarthChunkLatticeCoord::new(0, 0, 0),
        );
        let second = TerrainChunkBootstrapEntryV1::new(
            chunk(3),
            EarthChunkLatticeCoord::new(1, 0, 0),
        );

        assert_eq!(
            TerrainBootstrapCandidateV1::new(lineage(1), vec![second, first]),
            Err(TerrainBootstrapError::DuplicateChunkRef(chunk(3)))
        );
    }

    #[test]
    fn duplicate_lattice_locus_fails_before_candidate_exists() {
        let locus = EarthChunkLatticeCoord::new(-7, 4, 11);
        let first = TerrainChunkBootstrapEntryV1::new(chunk(4), locus);
        let second = TerrainChunkBootstrapEntryV1::new(chunk(5), locus);

        assert_eq!(
            TerrainBootstrapCandidateV1::new(lineage(1), vec![second, first]),
            Err(TerrainBootstrapError::DuplicateLatticeLocus(locus))
        );
    }

    #[test]
    fn changed_lineage_chunk_or_locus_changes_exact_bytes() {
        let base_entry =
            TerrainChunkBootstrapEntryV1::new(chunk(1), EarthChunkLatticeCoord::new(1, 2, 3));
        let base = TerrainBootstrapCandidateV1::new(lineage(1), vec![base_entry]).unwrap();

        let changed_lineage =
            TerrainBootstrapCandidateV1::new(lineage(2), vec![base_entry]).unwrap();
        let changed_chunk = TerrainBootstrapCandidateV1::new(
            lineage(1),
            vec![TerrainChunkBootstrapEntryV1::new(
                chunk(2),
                EarthChunkLatticeCoord::new(1, 2, 3),
            )],
        )
        .unwrap();
        let changed_locus = TerrainBootstrapCandidateV1::new(
            lineage(1),
            vec![TerrainChunkBootstrapEntryV1::new(
                chunk(1),
                EarthChunkLatticeCoord::new(1, 2, 4),
            )],
        )
        .unwrap();

        assert_ne!(base.canonical_bytes(), changed_lineage.canonical_bytes());
        assert_ne!(base.canonical_bytes(), changed_chunk.canonical_bytes());
        assert_ne!(base.canonical_bytes(), changed_locus.canonical_bytes());
    }

    #[test]
    fn external_lineage_profile_is_part_of_exact_identity() {
        let identity = id32(9);
        let local =
            TerrainLineageRefV1::new(TerrainLineageProfileV1::LocalExplicitV1, identity).unwrap();
        let external = TerrainLineageRefV1::new(
            TerrainLineageProfileV1::ExternalWorldLineageV1,
            identity,
        )
        .unwrap();
        let entry =
            TerrainChunkBootstrapEntryV1::new(chunk(1), EarthChunkLatticeCoord::new(0, 0, 0));

        let local = TerrainBootstrapCandidateV1::new(local, vec![entry]).unwrap();
        let external = TerrainBootstrapCandidateV1::new(external, vec![entry]).unwrap();

        assert_ne!(local.canonical_bytes(), external.canonical_bytes());
    }

    #[test]
    fn canonical_bytes_have_frozen_v1_grammar() {
        let candidate = TerrainBootstrapCandidateV1::new(
            lineage(0x11),
            vec![TerrainChunkBootstrapEntryV1::new(
                chunk(0x22),
                EarthChunkLatticeCoord::new(-1, 2, -3),
            )],
        )
        .unwrap();

        let bytes = candidate.canonical_bytes();
        let prefix_len = TERRAIN_BOOTSTRAP_DOMAIN.len();
        assert_eq!(&bytes[..prefix_len], TERRAIN_BOOTSTRAP_DOMAIN);
        assert_eq!(
            &bytes[prefix_len..prefix_len + 4],
            &TERRAIN_BOOTSTRAP_SCHEMA_VERSION.to_le_bytes()
        );
        assert_eq!(bytes[prefix_len + 4], 1);
        assert_eq!(bytes[prefix_len + 5], 0x11);
        assert!(bytes[prefix_len + 6..prefix_len + 37]
            .iter()
            .all(|byte| *byte == 0));

        let count_offset = prefix_len + 5 + TERRAIN_LINEAGE_ID_BYTES;
        assert_eq!(
            &bytes[count_offset..count_offset + 4],
            &1_u32.to_le_bytes()
        );

        let entry_offset = count_offset + 4;
        assert_eq!(bytes[entry_offset], 0x22);
        assert!(bytes[entry_offset + 1..entry_offset + 32]
            .iter()
            .all(|byte| *byte == 0));
        assert_eq!(
            &bytes[entry_offset + 32..entry_offset + 36],
            &(-1_i32).to_le_bytes()
        );
        assert_eq!(
            &bytes[entry_offset + 36..entry_offset + 40],
            &2_i32.to_le_bytes()
        );
        assert_eq!(
            &bytes[entry_offset + 40..entry_offset + 44],
            &(-3_i32).to_le_bytes()
        );
        assert_eq!(bytes.len(), entry_offset + TERRAIN_BOOTSTRAP_ENTRY_BYTES);
    }

    #[test]
    fn empty_candidate_is_canonical_but_not_authority() {
        let candidate = TerrainBootstrapCandidateV1::new(lineage(3), vec![]).unwrap();
        assert!(candidate.entries().is_empty());
        assert!(!candidate.canonical_bytes().is_empty());
    }
}
