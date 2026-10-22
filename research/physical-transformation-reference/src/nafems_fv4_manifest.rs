// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed source-separated reference data for NAFEMS FV4 / off-centre masses.
//!
//! These tables preserve the source's mode ordering and close first pair.
//! They are literature records, not results from Symtropy's modal solver.
//! Exact lump attachment coordinates and the raw input deck remain unresolved.

use crate::Check;

/// Published result source. Values from different solver pages are deliberately
/// stored as distinct variants rather than blended into a single gold list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fv4ResultVariant {
    NafemsTargetAsPublishedByAnsys,
    AnsysLsDynaVm062,
    AltairOptiStructOsV0410,
    AnsysMechanicalApdlBeam188,
    AnsysMechanicalApdlBeam189,
}

/// One scalar mode target or result. The first two are close but not declared
/// mathematically repeated by the reference; each therefore has multiplicity 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fv4ModeFrequency {
    pub mode_index: usize,
    pub frequency_hz: f64,
}

/// Result table for one named source/solver variant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fv4FrequencyVariant {
    pub id: Fv4ResultVariant,
    pub source_url: &'static str,
    pub frequencies: [Fv4ModeFrequency; 6],
    /// Published errors are supplied for the LS-DYNA source only.
    pub published_error_percent: Option<[f64; 6]>,
    /// Normalization factors as displayed by Altair; their direction is not
    /// reconstructed into absolute solver frequencies without source confirmation.
    pub published_normalized_factors: Option<[f64; 6]>,
    pub claim_ceiling: &'static str,
}

/// Main NAFEMS target table as quoted by the LS-DYNA verification page.
pub const NAFEMS_TARGETS_HZ: [f64; 6] = [1.723, 1.727, 7.413, 9.971, 18.155, 26.957];

/// LS-DYNA values and error percentages exactly as listed on VM-062.
pub const LSDYNA_RESULTS_HZ: [f64; 6] = [1.722, 1.725, 7.410, 9.960, 18.079, 26.843];
pub const LSDYNA_PUBLISHED_ERRORS_PERCENT: [f64; 6] =
    [0.058, 0.116, 0.040, 0.110, 0.419, 0.423];

/// Altair OS-V 0410 mode targets and published normalized factors. The table
/// rendering does not justify reconstructing absolute solver results from
/// the ratios, so keep these factors in their own field.
pub const ALTAIR_TARGETS_HZ: [f64; 6] = [1.723, 1.727, 7.413, 9.972, 18.155, 26.957];
pub const ALTAIR_NORMALIZED_FACTORS: [f64; 6] = [1.0054, 1.0048, 1.0080, 1.0055, 1.0067, 1.0068];

/// Secondary APDL solver results quoted on VMP09-T4, BEAM188 and BEAM189.
pub const APDL_BEAM188_HZ: [f64; 6] = [1.7198, 1.7236, 7.4021, 9.9733, 18.0842, 27.0797];
pub const APDL_BEAM189_HZ: [f64; 6] = [1.7203, 1.7238, 7.4032, 9.9365, 18.0351, 26.6788];

const CEILING: &str =
    "source-published frequency record only; no FV4 solve or physical validation";

fn mode_table(values: [f64; 6]) -> [Fv4ModeFrequency; 6] {
    [
        Fv4ModeFrequency { mode_index: 1, frequency_hz: values[0] },
        Fv4ModeFrequency { mode_index: 2, frequency_hz: values[1] },
        Fv4ModeFrequency { mode_index: 3, frequency_hz: values[2] },
        Fv4ModeFrequency { mode_index: 4, frequency_hz: values[3] },
        Fv4ModeFrequency { mode_index: 5, frequency_hz: values[4] },
        Fv4ModeFrequency { mode_index: 6, frequency_hz: values[5] },
    ]
}

/// All external records with explicit source identity.
pub fn published_variants() -> [Fv4FrequencyVariant; 5] {
    [
        Fv4FrequencyVariant {
            id: Fv4ResultVariant::NafemsTargetAsPublishedByAnsys,
            source_url: "https://ansyshelp.ansys.com/public/views/secured/corp/v261/en/lsdyna_vm/vm-lsdyna-solve-062.html",
            frequencies: mode_table(NAFEMS_TARGETS_HZ),
            published_error_percent: None,
            published_normalized_factors: None,
            claim_ceiling: CEILING,
        },
        Fv4FrequencyVariant {
            id: Fv4ResultVariant::AnsysLsDynaVm062,
            source_url: "https://ansyshelp.ansys.com/public/views/secured/corp/v261/en/lsdyna_vm/vm-lsdyna-solve-062.html",
            frequencies: mode_table(LSDYNA_RESULTS_HZ),
            published_error_percent: Some(LSDYNA_PUBLISHED_ERRORS_PERCENT),
            published_normalized_factors: None,
            claim_ceiling: CEILING,
        },
        Fv4FrequencyVariant {
            id: Fv4ResultVariant::AltairOptiStructOsV0410,
            source_url: "https://2022.help.altair.com/2022/hwsolvers/os/topics/solvers/os/nafems_test_problem_fv4_r.htm",
            frequencies: mode_table(ALTAIR_TARGETS_HZ),
            published_error_percent: None,
            published_normalized_factors: Some(ALTAIR_NORMALIZED_FACTORS),
            claim_ceiling: CEILING,
        },
        Fv4FrequencyVariant {
            id: Fv4ResultVariant::AnsysMechanicalApdlBeam188,
            source_url: "https://ansyshelp.ansys.com/public/views/secured/corp/v261/en/ans_vm/vmp09t4.html",
            frequencies: mode_table(APDL_BEAM188_HZ),
            published_error_percent: None,
            published_normalized_factors: None,
            claim_ceiling: CEILING,
        },
        Fv4FrequencyVariant {
            id: Fv4ResultVariant::AnsysMechanicalApdlBeam189,
            source_url: "https://ansyshelp.ansys.com/public/views/secured/corp/v261/en/ans_vm/vmp09t4.html",
            frequencies: mode_table(APDL_BEAM189_HZ),
            published_error_percent: None,
            published_normalized_factors: None,
            claim_ceiling: CEILING,
        },
    ]
}

/// Relative separation of the first two target frequencies, relative to the
/// lower target frequency. This is an explicit measure of how fragile sorting
/// alone can be when following the first pair across model changes.
pub fn close_pair_relative_gap(frequencies: &[Fv4ModeFrequency; 6]) -> Check<f64> {
    validate_frequency_table(frequencies)?;
    if frequencies[0].mode_index != 1 || frequencies[1].mode_index != 2 {
        return Err("close-pair metric requires ordered modes 1 and 2".to_owned());
    }
    Ok((frequencies[1].frequency_hz - frequencies[0].frequency_hz)
        / frequencies[0].frequency_hz)
}

fn validate_frequency_table(frequencies: &[Fv4ModeFrequency; 6]) -> Check<()> {
    for (index, mode) in frequencies.iter().enumerate() {
        if mode.mode_index != index + 1
            || !mode.frequency_hz.is_finite()
            || mode.frequency_hz <= 0.0
        {
            return Err("FV4 frequency table must be finite, positive and ordered 1..6".to_owned());
        }
    }
    Ok(())
}

/// Relative target errors in percent, independently recomputed from the
/// reference targets and a selected solver result table.
pub fn relative_errors_percent(
    targets: &[Fv4ModeFrequency; 6],
    solver: &[Fv4ModeFrequency; 6],
) -> Check<[f64; 6]> {
    validate_frequency_table(targets)?;
    validate_frequency_table(solver)?;
    let mut result = [0.0; 6];
    for index in 0..6 {
        result[index] = (solver[index].frequency_hz - targets[index].frequency_hz).abs()
            / targets[index].frequency_hz
            * 100.0;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{
        ALTAIR_NORMALIZED_FACTORS, ALTAIR_TARGETS_HZ, APDL_BEAM188_HZ, APDL_BEAM189_HZ,
        Fv4ResultVariant, LSDYNA_PUBLISHED_ERRORS_PERCENT, LSDYNA_RESULTS_HZ,
        NAFEMS_TARGETS_HZ, close_pair_relative_gap, mode_table, published_variants,
        relative_errors_percent,
    };

    const FIXTURE: &str = include_str!("../fixtures/nafems_fv4_close_modes_manifest_v0_1.txt");

    #[test]
    fn first_pair_is_close_and_preserved_by_mode_index() {
        let gap = close_pair_relative_gap(&mode_table(NAFEMS_TARGETS_HZ))
            .expect("valid NAFEMS target table");
        assert!((NAFEMS_TARGETS_HZ[1] - NAFEMS_TARGETS_HZ[0] - 0.004).abs() < 1.0e-12);
        assert!((gap - 0.002321532211259433).abs() < 1.0e-15);
        assert!(gap < 0.0025);
    }

    #[test]
    fn published_lsdyna_errors_match_recomputed_target_differences() {
        let calculated = relative_errors_percent(
            &mode_table(NAFEMS_TARGETS_HZ),
            &mode_table(LSDYNA_RESULTS_HZ),
        )
        .expect("calculate LS-DYNA target errors");
        for index in 0..6 {
            assert!(
                (calculated[index] - LSDYNA_PUBLISHED_ERRORS_PERCENT[index]).abs() < 0.001,
                "mode {} published error does not match: calculated={:.8}, published={:.8}",
                index + 1,
                calculated[index],
                LSDYNA_PUBLISHED_ERRORS_PERCENT[index]
            );
        }
    }

    #[test]
    fn solver_variants_remain_distinct_source_bound_records() {
        let variants = published_variants();
        assert_eq!(variants.len(), 5);
        assert_ne!(variants[0].id, variants[1].id);
        assert_eq!(variants[2].id, Fv4ResultVariant::AltairOptiStructOsV0410);
        assert_eq!(variants[2].frequencies[3].frequency_hz, 9.972);
        assert_eq!(variants[2].published_normalized_factors, Some(ALTAIR_NORMALIZED_FACTORS));
        assert_ne!(variants[3].id, variants[4].id);
        assert_eq!(variants[3].frequencies[0].frequency_hz, APDL_BEAM188_HZ[0]);
        assert_eq!(variants[4].frequencies[0].frequency_hz, APDL_BEAM189_HZ[0]);
        assert_eq!(variants[1].published_normalized_factors, None);
        assert_eq!(variants[1].frequencies[3].frequency_hz, 9.960);
        assert_eq!(ALTAIR_TARGETS_HZ[3] - NAFEMS_TARGETS_HZ[3], 0.001);
        assert!(variants.iter().all(|variant| variant.claim_ceiling.contains("no FV4 solve")));
        assert!(
            (APDL_BEAM188_HZ[1] - APDL_BEAM188_HZ[0]).abs() / APDL_BEAM188_HZ[0] < 0.003
        );
        assert!(
            (APDL_BEAM189_HZ[1] - APDL_BEAM189_HZ[0]).abs() / APDL_BEAM189_HZ[0] < 0.003
        );
    }

    #[test]
    fn altair_normalization_factors_remain_separate_from_frequency_values() {
        let variants = published_variants();
        let altair = variants.iter()
            .find(|variant| variant.id == Fv4ResultVariant::AltairOptiStructOsV0410)
            .expect("Altair FV4 variant");
        assert_eq!(altair.frequencies[3].frequency_hz, 9.972);
        assert_eq!(altair.published_normalized_factors, Some(ALTAIR_NORMALIZED_FACTORS));
        assert!(altair.published_error_percent.is_none());
    }

    #[test]
    fn frozen_fixture_keeps_source_inputs_and_unresolved_offset_fields() {
        let fixture = FIXTURE.trim_end_matches('\n');
        assert!(fixture.contains("target_hz=1.723,1.727,7.413,9.971,18.155,26.957"));
        assert!(fixture.contains("lsdyna_hz=1.722,1.725,7.410,9.960,18.079,26.843"));
        assert!(fixture.contains("closest_pair_gap_hz=0.004"));
        assert!(fixture.contains("altair_target_hz=1.723,1.727,7.413,9.972,18.155,26.957"));
        assert!(fixture.contains("altair_normalized_factors=1.0054,1.0048,1.0080,1.0055,1.0067,1.0068"));
        assert!(fixture.contains("mass_attachment_coordinates_status=unresolved_primary_input"));
        assert!(fixture.contains("claim_ceiling=manifest_only_no_FV4_solve_or_physical_validation"));
        assert!(published_variants().iter().any(|variant| {
            variant.id == Fv4ResultVariant::NafemsTargetAsPublishedByAnsys
        }));
    }
}
