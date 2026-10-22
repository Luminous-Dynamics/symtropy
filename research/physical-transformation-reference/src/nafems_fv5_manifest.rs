// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed source-separated records for the public NAFEMS FV5 benchmark family.
//!
//! The R0015/DIANA-reproduced theoretical values and the Ansys LS-DYNA
//! verification-manual values are explicitly different reference variants.
//! This module prevents benchmark reporting from silently treating them as
//! one gold-frequency list. It does not implement the 3D beam formulation.

use crate::Check;

/// A named external interpretation, not just a solver brand.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fv5ReferenceVariant {
    /// The theoretical frequency family reproduced in the DIANA FV5 page.
    R0015TheoryDianaPresentation,
    /// Ansys VM-LSDYNA-SOLVE-064, with the documented 2 x 2 m section.
    AnsysVmLsdynaSolve064,
}

/// Physical mode family; a family can have multiplicity greater than one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fv5ModeKind {
    Flexural,
    Torsional,
    Extensional,
}

/// A target is one named family, not a sorted single-eigenvalue index.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fv5ModeFamily {
    pub family_id: &'static str,
    pub kind: Fv5ModeKind,
    pub multiplicity: usize,
    pub target_hz: f64,
    /// Optional separately published solver output for the same source variant.
    pub solver_hz: Option<f64>,
}

/// Reference variant including source provenance and six frequency families.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fv5BenchmarkVariant {
    pub id: Fv5ReferenceVariant,
    pub source_url: &'static str,
    pub section_interpretation: &'static str,
    pub mode_families: [Fv5ModeFamily; 6],
    pub claim_ceiling: &'static str,
}

const R0015_DIANA_FAMILIES: [Fv5ModeFamily; 6] = [
    Fv5ModeFamily {
        family_id: "flexural-1",
        kind: Fv5ModeKind::Flexural,
        multiplicity: 2,
        target_hz: 42.649,
        solver_hz: Some(42.626),
    },
    Fv5ModeFamily {
        family_id: "torsional-1",
        kind: Fv5ModeKind::Torsional,
        multiplicity: 1,
        target_hz: 77.542,
        solver_hz: Some(70.768),
    },
    Fv5ModeFamily {
        family_id: "extensional-1",
        kind: Fv5ModeKind::Extensional,
        multiplicity: 1,
        target_hz: 125.000,
        solver_hz: Some(125.000),
    },
    Fv5ModeFamily {
        family_id: "flexural-2",
        kind: Fv5ModeKind::Flexural,
        multiplicity: 2,
        target_hz: 148.310,
        solver_hz: Some(148.580),
    },
    Fv5ModeFamily {
        family_id: "torsional-2",
        kind: Fv5ModeKind::Torsional,
        multiplicity: 1,
        target_hz: 233.100,
        solver_hz: Some(212.520),
    },
    Fv5ModeFamily {
        family_id: "flexural-3",
        kind: Fv5ModeKind::Flexural,
        multiplicity: 2,
        target_hz: 284.550,
        solver_hz: Some(288.750),
    },
];

const ANSYS_LSDYNA_FAMILIES: [Fv5ModeFamily; 6] = [
    Fv5ModeFamily {
        family_id: "flexural-1",
        kind: Fv5ModeKind::Flexural,
        multiplicity: 2,
        target_hz: 42.650,
        solver_hz: Some(42.642),
    },
    Fv5ModeFamily {
        family_id: "torsional-1",
        kind: Fv5ModeKind::Torsional,
        multiplicity: 1,
        target_hz: 71.200,
        solver_hz: Some(70.735),
    },
    Fv5ModeFamily {
        family_id: "extensional-1",
        kind: Fv5ModeKind::Extensional,
        multiplicity: 1,
        target_hz: 125.000,
        solver_hz: Some(124.968),
    },
    Fv5ModeFamily {
        family_id: "flexural-2",
        kind: Fv5ModeKind::Flexural,
        multiplicity: 2,
        target_hz: 148.150,
        solver_hz: Some(148.000),
    },
    Fv5ModeFamily {
        family_id: "torsional-2",
        kind: Fv5ModeKind::Torsional,
        multiplicity: 1,
        target_hz: 213.610,
        solver_hz: Some(211.769),
    },
    Fv5ModeFamily {
        family_id: "flexural-3",
        kind: Fv5ModeKind::Flexural,
        multiplicity: 2,
        target_hz: 283.470,
        solver_hz: Some(282.405),
    },
];

const UNQUALIFIED_CEILING: &str =
    "typed literature reference only; not an executed FV5 solve or physical validation";

/// Return the reference variants separately. Consumers must choose one ID.
pub fn documented_variants() -> [Fv5BenchmarkVariant; 2] {
    [
        Fv5BenchmarkVariant {
            id: Fv5ReferenceVariant::R0015TheoryDianaPresentation,
            source_url: "https://manuals.dianafea.com/d1010/en/1353544-1353584-deep-simply-supported-beam.html",
            section_interpretation: "R0015/DIANA theoretical family; exact section/raw input requires source reconciliation",
            mode_families: R0015_DIANA_FAMILIES,
            claim_ceiling: UNQUALIFIED_CEILING,
        },
        Fv5BenchmarkVariant {
            id: Fv5ReferenceVariant::AnsysVmLsdynaSolve064,
            source_url: "https://ansyshelp.ansys.com/public/views/secured/corp/v261/en/lsdyna_vm/vm-lsdyna-solve-064.html",
            section_interpretation: "10 m beam; 2 x 2 m section as stated in the Ansys page",
            mode_families: ANSYS_LSDYNA_FAMILIES,
            claim_ceiling: UNQUALIFIED_CEILING,
        },
    ]
}

/// Relative discrepancy between target values of two *different named
/// variants* for the same family. This diagnostic does not select a winner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VariantTargetDiscrepancy {
    pub family_id: &'static str,
    pub left_variant: Fv5ReferenceVariant,
    pub right_variant: Fv5ReferenceVariant,
    pub left_target_hz: f64,
    pub right_target_hz: f64,
    pub relative_difference_to_left: f64,
}

/// Compare families by stable physical family ID, never by sorted eigenvalue
/// index. An invalid/duplicated family or mismatched kind/multiplicity fails.
pub fn compare_reference_variants(
    left: &Fv5BenchmarkVariant,
    right: &Fv5BenchmarkVariant,
) -> Check<Vec<VariantTargetDiscrepancy>> {
    if left.id == right.id {
        return Err("variant comparison requires two distinct reference IDs".to_owned());
    }
    validate_variant(left)?;
    validate_variant(right)?;

    let mut result = Vec::with_capacity(left.mode_families.len());
    for left_family in left.mode_families {
        let mut matches = right
            .mode_families
            .iter()
            .filter(|candidate| candidate.family_id == left_family.family_id);
        let right_family = matches
            .next()
            .ok_or_else(|| format!("right variant is missing family {}", left_family.family_id))?;
        if matches.next().is_some() {
            return Err(format!(
                "right variant has duplicate family {}",
                left_family.family_id
            ));
        }
        if right_family.kind != left_family.kind
            || right_family.multiplicity != left_family.multiplicity
        {
            return Err(format!(
                "family identity mismatch for {}",
                left_family.family_id
            ));
        }
        let relative_difference_to_left =
            (right_family.target_hz - left_family.target_hz).abs() / left_family.target_hz;
        result.push(VariantTargetDiscrepancy {
            family_id: left_family.family_id,
            left_variant: left.id,
            right_variant: right.id,
            left_target_hz: left_family.target_hz,
            right_target_hz: right_family.target_hz,
            relative_difference_to_left,
        });
    }
    if result.len() != right.mode_families.len() {
        return Err("reference variants do not contain the same mode-family set".to_owned());
    }
    Ok(result)
}

fn validate_variant(variant: &Fv5BenchmarkVariant) -> Check<()> {
    if variant.source_url.is_empty()
        || variant.section_interpretation.is_empty()
        || variant.claim_ceiling.is_empty()
    {
        return Err("variant must declare source, model interpretation and claim ceiling".to_owned());
    }
    let mut seen = std::collections::BTreeSet::new();
    for family in variant.mode_families {
        if family.family_id.is_empty()
            || family.multiplicity == 0
            || !family.target_hz.is_finite()
            || family.target_hz <= 0.0
            || family
                .solver_hz
                .is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            return Err("mode family has invalid identity, multiplicity or frequency".to_owned());
        }
        if !seen.insert(family.family_id) {
            return Err(format!("duplicate mode family {}", family.family_id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        Fv5ModeKind, Fv5ReferenceVariant, compare_reference_variants, documented_variants,
    };

    const R0015_FIXTURE: &str =
        include_str!("../fixtures/nafems_r0015_fv5_mode_manifest_v0_1.txt");
    const ANSYS_FIXTURE: &str =
        include_str!("../fixtures/nafems_fv5_lsdyna_variant_v0_1.txt");

    #[test]
    fn variants_preserve_nine_modes_and_repeated_flexural_multiplicities() {
        let variants = documented_variants();
        for variant in variants {
            assert_eq!(
                variant.mode_families.iter().map(|family| family.multiplicity).sum::<usize>(),
                9
            );
            assert_eq!(
                variant.mode_families.iter().filter(|family| family.kind == Fv5ModeKind::Flexural)
                    .map(|family| family.multiplicity).sum::<usize>(),
                6
            );
            assert_eq!(
                variant.mode_families.iter().filter(|family| family.kind == Fv5ModeKind::Torsional)
                    .map(|family| family.multiplicity).sum::<usize>(),
                2
            );
            assert_eq!(
                variant.mode_families.iter().filter(|family| family.kind == Fv5ModeKind::Extensional)
                    .map(|family| family.multiplicity).sum::<usize>(),
                1
            );
            assert!(variant.claim_ceiling.contains("not an executed FV5 solve"));
        }
    }

    #[test]
    fn torsion_targets_are_not_silently_merged_across_variants() {
        let variants = documented_variants();
        let differences =
            compare_reference_variants(&variants[0], &variants[1]).expect("separate variants");
        assert_eq!(differences.len(), 6);
        assert!(differences.iter().all(|difference| {
            difference.left_variant == Fv5ReferenceVariant::R0015TheoryDianaPresentation
                && difference.right_variant == Fv5ReferenceVariant::AnsysVmLsdynaSolve064
        }));
        let torsion_one = differences
            .iter()
            .find(|difference| difference.family_id == "torsional-1")
            .expect("first torsional family");
        let torsion_two = differences
            .iter()
            .find(|difference| difference.family_id == "torsional-2")
            .expect("second torsional family");
        assert!(torsion_one.relative_difference_to_left > 0.08);
        assert!(torsion_two.relative_difference_to_left > 0.08);
        assert_eq!(torsion_one.left_target_hz, 77.542);
        assert_eq!(torsion_one.right_target_hz, 71.200);
        assert_eq!(torsion_two.left_target_hz, 233.100);
        assert_eq!(torsion_two.right_target_hz, 213.610);
    }

    #[test]
    fn flexural_and_extensional_targets_remain_near_cross_variant_values() {
        let variants = documented_variants();
        let differences =
            compare_reference_variants(&variants[0], &variants[1]).expect("valid variants");
        let flexural_one = differences
            .iter()
            .find(|difference| difference.family_id == "flexural-1")
            .expect("first flexural family");
        let extensional = differences
            .iter()
            .find(|difference| difference.family_id == "extensional-1")
            .expect("extensional family");
        assert!(flexural_one.relative_difference_to_left < 1.0e-4);
        assert!(extensional.relative_difference_to_left < 1.0e-12);
    }

    #[test]
    fn source_fixtures_preserve_variant_specific_sections_and_targets() {
        let r0015 = R0015_FIXTURE.trim_end_matches('\n');
        let ansys = ANSYS_FIXTURE.trim_end_matches('\n');
        assert!(r0015.contains("target_family=torsional1,1,77.542"));
        assert!(r0015.contains("section_geometry_status=unresolved_primary_source"));
        assert!(ansys.contains("section=2x2_m"));
        assert!(ansys.contains("target_family=torsional1,1,71.200"));
        assert!(ansys.contains("target_family=torsional2,1,213.610"));
        assert!(ansys.contains("claim_ceiling=vendor_published_variant_only"));
    }

    #[test]
    fn same_variant_comparison_fails_closed() {
        let variants = documented_variants();
        assert!(compare_reference_variants(&variants[0], &variants[0]).is_err());
    }
}
