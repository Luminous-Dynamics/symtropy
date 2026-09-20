// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! WORLD-SCOPE-CATALOG-00A executable reference oracle.
//!
//! This is a sparse read/index theorem over an already-authoritative hierarchy
//! identity. It cannot mint, delete, rename, or reparent a semantic scope.
//!
//! The critical negative-evidence rule is:
//!
//! `unavailable / uncovered / partial page != QualifiedNoScope`
//!
//! Only a complete page covering the queried key may prove qualified absence.

use symtropy_sim_contracts::{
    ContractError, DigestAlgorithm, ScopeId, TypedDigest32, WorldInstanceId,
};

const SCHEMA_VERSION: u32 = 1;
const MAX_PAGES: usize = 1_024;
const MAX_ENTRIES_PER_PAGE: usize = 1_024;
const CATALOG_DOMAIN: &[u8] = b"symtropy.scope-catalog.reference.v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Completeness {
    Partial,
    Complete,
}

impl Completeness {
    const fn code(self) -> u8 {
        match self {
            Self::Partial => 0,
            Self::Complete => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CatalogEntry {
    scope: ScopeId,
    parent: Option<ScopeId>,
}

impl CatalogEntry {
    fn new(scope: ScopeId, parent: Option<ScopeId>) -> Self {
        Self { scope, parent }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CatalogPage {
    lower_inclusive: ScopeId,
    upper_exclusive: ScopeId,
    completeness: Completeness,
    entries: Vec<CatalogEntry>,
}

impl CatalogPage {
    fn new(
        lower_inclusive: ScopeId,
        upper_exclusive: ScopeId,
        completeness: Completeness,
        mut entries: Vec<CatalogEntry>,
    ) -> Result<Self, RefError> {
        lower_inclusive.validate()?;
        upper_exclusive.validate()?;
        if lower_inclusive >= upper_exclusive {
            return Err(RefError::InvalidRange);
        }
        if entries.len() > MAX_ENTRIES_PER_PAGE {
            return Err(RefError::TooManyEntries);
        }

        entries.sort();
        let mut previous: Option<&ScopeId> = None;
        for entry in &entries {
            entry.scope.validate()?;
            if let Some(parent) = &entry.parent {
                parent.validate()?;
            }
            if entry.scope < lower_inclusive || entry.scope >= upper_exclusive {
                return Err(RefError::EntryOutsideRange);
            }
            if previous == Some(&entry.scope) {
                return Err(RefError::DuplicateScope);
            }
            previous = Some(&entry.scope);
        }

        Ok(Self {
            lower_inclusive,
            upper_exclusive,
            completeness,
            entries,
        })
    }

    fn covers(&self, scope: &ScopeId) -> bool {
        self.lower_inclusive.as_str() <= scope.as_str()
            && scope.as_str() < self.upper_exclusive.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Catalog {
    schema_version: u32,
    world: WorldInstanceId,
    source_hierarchy: TypedDigest32,
    pages: Vec<CatalogPage>,
}

impl Catalog {
    fn new(
        world: WorldInstanceId,
        source_hierarchy: TypedDigest32,
        mut pages: Vec<CatalogPage>,
    ) -> Result<Self, RefError> {
        if pages.len() > MAX_PAGES {
            return Err(RefError::TooManyPages);
        }
        world.validate()?;
        source_hierarchy.validate()?;

        pages.sort_by(|left, right| {
            left.lower_inclusive
                .cmp(&right.lower_inclusive)
                .then_with(|| left.upper_exclusive.cmp(&right.upper_exclusive))
                .then_with(|| left.completeness.cmp(&right.completeness))
        });

        for pair in pages.windows(2) {
            if pair[1].lower_inclusive < pair[0].upper_exclusive {
                return Err(RefError::OverlappingPages);
            }
        }

        let catalog = Self {
            schema_version: SCHEMA_VERSION,
            world,
            source_hierarchy,
            pages,
        };
        catalog.validate()?;
        Ok(catalog)
    }

    fn validate(&self) -> Result<(), RefError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(RefError::UnsupportedSchema);
        }
        if self.pages.len() > MAX_PAGES {
            return Err(RefError::TooManyPages);
        }
        self.world.validate()?;
        self.source_hierarchy.validate()?;

        for page in &self.pages {
            page.lower_inclusive.validate()?;
            page.upper_exclusive.validate()?;
            if page.lower_inclusive >= page.upper_exclusive {
                return Err(RefError::InvalidRange);
            }
            if page.entries.len() > MAX_ENTRIES_PER_PAGE {
                return Err(RefError::TooManyEntries);
            }
            let mut previous: Option<&ScopeId> = None;
            for entry in &page.entries {
                entry.scope.validate()?;
                if let Some(parent) = &entry.parent {
                    parent.validate()?;
                }
                if !page.covers(&entry.scope) {
                    return Err(RefError::EntryOutsideRange);
                }
                if previous == Some(&entry.scope) {
                    return Err(RefError::DuplicateScope);
                }
                previous = Some(&entry.scope);
            }
        }

        for pair in self.pages.windows(2) {
            if pair[1].lower_inclusive < pair[0].upper_exclusive {
                return Err(RefError::OverlappingPages);
            }
        }

        Ok(())
    }

    fn canonical_bytes(&self) -> Result<Vec<u8>, RefError> {
        self.validate()?;

        let mut bytes = Vec::new();
        bytes.extend_from_slice(CATALOG_DOMAIN);
        bytes.extend_from_slice(&self.schema_version.to_le_bytes());
        push_string(&mut bytes, self.world.as_str());
        push_digest(&mut bytes, &self.source_hierarchy);
        bytes.extend_from_slice(&(self.pages.len() as u32).to_le_bytes());

        for page in &self.pages {
            push_string(&mut bytes, page.lower_inclusive.as_str());
            push_string(&mut bytes, page.upper_exclusive.as_str());
            bytes.push(page.completeness.code());
            bytes.extend_from_slice(&(page.entries.len() as u32).to_le_bytes());

            for entry in &page.entries {
                push_string(&mut bytes, entry.scope.as_str());
                match &entry.parent {
                    Some(parent) => {
                        bytes.push(1);
                        push_string(&mut bytes, parent.as_str());
                    }
                    None => bytes.push(0),
                }
            }
        }

        Ok(bytes)
    }

    fn digest(&self) -> Result<TypedDigest32, RefError> {
        Ok(TypedDigest32::sha256(
            "symtropy.scope-catalog.reference.v1",
            SCHEMA_VERSION,
            &self.canonical_bytes()?,
        )?)
    }

    fn lookup_current(
        &self,
        world: &WorldInstanceId,
        current_hierarchy: &TypedDigest32,
        scope: &ScopeId,
    ) -> Result<Lookup, RefError> {
        self.validate()?;
        world.validate()?;
        current_hierarchy.validate()?;
        scope.validate()?;

        if world != &self.world {
            return Err(RefError::WorldMismatch);
        }
        if !current_hierarchy.same_typed_value(&self.source_hierarchy) {
            return Err(RefError::StaleCatalog);
        }

        let Some(page) = self.pages.iter().find(|page| page.covers(scope)) else {
            return Ok(Lookup::Incomplete);
        };

        if let Some(entry) = page.entries.iter().find(|entry| &entry.scope == scope) {
            return Ok(Lookup::Present {
                scope: entry.scope.clone(),
                parent: entry.parent.clone(),
            });
        }

        match page.completeness {
            Completeness::Complete => Ok(Lookup::QualifiedNoScope),
            Completeness::Partial => Ok(Lookup::Incomplete),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Lookup {
    Present {
        scope: ScopeId,
        parent: Option<ScopeId>,
    },
    QualifiedNoScope,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RefError {
    Contract,
    UnsupportedSchema,
    TooManyPages,
    TooManyEntries,
    InvalidRange,
    EntryOutsideRange,
    DuplicateScope,
    OverlappingPages,
    WorldMismatch,
    StaleCatalog,
}

impl From<ContractError> for RefError {
    fn from(_: ContractError) -> Self {
        Self::Contract
    }
}

fn push_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn push_digest(bytes: &mut Vec<u8>, digest: &TypedDigest32) {
    push_string(bytes, &digest.domain);
    match &digest.algorithm {
        DigestAlgorithm::Sha256 => bytes.push(0),
        DigestAlgorithm::Other(name) => {
            bytes.push(1);
            push_string(bytes, name);
        }
    }
    bytes.extend_from_slice(&digest.schema_version.to_le_bytes());
    bytes.extend_from_slice(&digest.value);
}

fn world(value: &str) -> WorldInstanceId {
    WorldInstanceId::parse(value).unwrap()
}

fn scope(value: &str) -> ScopeId {
    ScopeId::parse(value).unwrap()
}

fn hierarchy(value: &[u8]) -> TypedDigest32 {
    TypedDigest32::sha256("symtropy.scope-hierarchy.reference.v1", 1, value).unwrap()
}

fn complete_page(
    lower: &str,
    upper: &str,
    entries: Vec<CatalogEntry>,
) -> Result<CatalogPage, RefError> {
    CatalogPage::new(
        scope(lower),
        scope(upper),
        Completeness::Complete,
        entries,
    )
}

#[test]
fn exact_present_lookup_is_bound_to_world_and_hierarchy() {
    let source = hierarchy(b"hierarchy-a");
    let catalog = Catalog::new(
        world("world:a"),
        source.clone(),
        vec![complete_page(
            "a",
            "m",
            vec![CatalogEntry::new(
                scope("body:a"),
                Some(scope("system")),
            )],
        )
        .unwrap()],
    )
    .unwrap();

    match catalog
        .lookup_current(&world("world:a"), &source, &scope("body:a"))
        .unwrap()
    {
        Lookup::Present {
            scope: found,
            parent,
        } => {
            assert_eq!(found, scope("body:a"));
            assert_eq!(parent, Some(scope("system")));
        }
        other => panic!("expected Present, got {other:?}"),
    }
}

#[test]
fn complete_covering_page_can_prove_qualified_absence() {
    let source = hierarchy(b"hierarchy-a");
    let catalog = Catalog::new(
        world("world:a"),
        source.clone(),
        vec![complete_page("a", "m", vec![]).unwrap()],
    )
    .unwrap();

    assert_eq!(
        catalog
            .lookup_current(&world("world:a"), &source, &scope("body:missing"))
            .unwrap(),
        Lookup::QualifiedNoScope
    );
}

#[test]
fn uncovered_or_partial_page_is_never_qualified_absence() {
    let source = hierarchy(b"hierarchy-a");
    let partial = CatalogPage::new(
        scope("a"),
        scope("m"),
        Completeness::Partial,
        vec![],
    )
    .unwrap();
    let catalog = Catalog::new(world("world:a"), source.clone(), vec![partial]).unwrap();

    assert_eq!(
        catalog
            .lookup_current(&world("world:a"), &source, &scope("body:missing"))
            .unwrap(),
        Lookup::Incomplete
    );
    assert_eq!(
        catalog
            .lookup_current(&world("world:a"), &source, &scope("z:outside"))
            .unwrap(),
        Lookup::Incomplete
    );
}

#[test]
fn stale_hierarchy_or_wrong_world_cannot_use_catalog_as_current_truth() {
    let source = hierarchy(b"hierarchy-a");
    let catalog = Catalog::new(
        world("world:a"),
        source.clone(),
        vec![complete_page("a", "z", vec![]).unwrap()],
    )
    .unwrap();

    assert_eq!(
        catalog.lookup_current(
            &world("world:a"),
            &hierarchy(b"hierarchy-b"),
            &scope("body:a"),
        ),
        Err(RefError::StaleCatalog)
    );
    assert_eq!(
        catalog.lookup_current(&world("world:b"), &source, &scope("body:a")),
        Err(RefError::WorldMismatch)
    );
}

#[test]
fn page_and_entry_order_cannot_change_catalog_identity() {
    let source = hierarchy(b"hierarchy-a");
    let low_a = CatalogEntry::new(scope("body:a"), Some(scope("system")));
    let low_b = CatalogEntry::new(scope("body:b"), Some(scope("system")));
    let high = CatalogEntry::new(scope("region:z"), Some(scope("body:b")));

    let first = Catalog::new(
        world("world:a"),
        source.clone(),
        vec![
            complete_page("n", "z", vec![high.clone()]).unwrap(),
            complete_page("a", "n", vec![low_b.clone(), low_a.clone()]).unwrap(),
        ],
    )
    .unwrap();
    let second = Catalog::new(
        world("world:a"),
        source,
        vec![
            complete_page("a", "n", vec![low_a, low_b]).unwrap(),
            complete_page("n", "z", vec![high]).unwrap(),
        ],
    )
    .unwrap();

    assert_eq!(first, second);
    assert_eq!(first.digest().unwrap(), second.digest().unwrap());
}

#[test]
fn overlapping_pages_and_duplicate_keys_fail_closed() {
    let overlap = Catalog::new(
        world("world:a"),
        hierarchy(b"hierarchy-a"),
        vec![
            complete_page("a", "n", vec![]).unwrap(),
            complete_page("m", "z", vec![]).unwrap(),
        ],
    );
    assert_eq!(overlap, Err(RefError::OverlappingPages));

    let duplicate = complete_page(
        "a",
        "z",
        vec![
            CatalogEntry::new(scope("body:a"), Some(scope("system"))),
            CatalogEntry::new(scope("body:a"), Some(scope("other"))),
        ],
    );
    assert_eq!(duplicate, Err(RefError::DuplicateScope));
}

#[test]
fn changed_source_hierarchy_changes_catalog_identity_even_with_same_pages() {
    let page = complete_page(
        "a",
        "z",
        vec![CatalogEntry::new(
            scope("body:a"),
            Some(scope("system")),
        )],
    )
    .unwrap();

    let first = Catalog::new(
        world("world:a"),
        hierarchy(b"hierarchy-a"),
        vec![page.clone()],
    )
    .unwrap();
    let second = Catalog::new(
        world("world:a"),
        hierarchy(b"hierarchy-b"),
        vec![page],
    )
    .unwrap();

    assert_ne!(first.digest().unwrap(), second.digest().unwrap());
}

#[test]
fn positive_presence_does_not_require_complete_page() {
    let source = hierarchy(b"hierarchy-a");
    let partial = CatalogPage::new(
        scope("a"),
        scope("z"),
        Completeness::Partial,
        vec![CatalogEntry::new(
            scope("body:a"),
            Some(scope("system")),
        )],
    )
    .unwrap();
    let catalog = Catalog::new(world("world:a"), source.clone(), vec![partial]).unwrap();

    assert!(matches!(
        catalog
            .lookup_current(&world("world:a"), &source, &scope("body:a"))
            .unwrap(),
        Lookup::Present { .. }
    ));
}
