use material_damage_reference::fatigue::{
    LoadBlock, MinerAssessment, SnCurve, SnPoint, miner_damage,
};
use material_damage_reference::paris::ParisLaw;
use material_damage_reference::DamageError;

fn synthetic_sn_curve() -> SnCurve {
    // Synthetic verification points only; not properties of a real material.
    SnCurve::new(vec![
        SnPoint { stress_amplitude_mpa: 300.0, cycles_to_failure: 10_000 },
        SnPoint { stress_amplitude_mpa: 200.0, cycles_to_failure: 40_000 },
        SnPoint { stress_amplitude_mpa: 100.0, cycles_to_failure: 1_000_000 },
    ])
    .expect("synthetic S-N curve should be valid")
}

#[test]
fn sn_curve_returns_frozen_endpoints_and_log_log_midpoint() {
    let curve = SnCurve::new(vec![
        SnPoint { stress_amplitude_mpa: 300.0, cycles_to_failure: 10_000 },
        SnPoint { stress_amplitude_mpa: 100.0, cycles_to_failure: 1_000_000 },
    ])
    .unwrap();

    assert_eq!(curve.cycles_to_failure(300.0).unwrap(), 10_000.0);
    assert_eq!(curve.cycles_to_failure(100.0).unwrap(), 1_000_000.0);

    let stress_midpoint = (300.0_f64 * 100.0).sqrt();
    let cycles = curve.cycles_to_failure(stress_midpoint).unwrap();
    assert!((cycles - 100_000.0).abs() <= 1.0e-8);
}

#[test]
fn miner_rule_matches_independent_block_damage_known_answer() {
    let curve = synthetic_sn_curve();
    let report = miner_damage(
        &curve,
        &[
            LoadBlock { stress_amplitude_mpa: 200.0, cycles: 10_000 },
            LoadBlock { stress_amplitude_mpa: 300.0, cycles: 2_500 },
        ],
    )
    .unwrap();

    assert!((report.damage_fraction - 0.5).abs() <= 1.0e-15);
    assert_eq!(report.total_cycles, 12_500);
    assert_eq!(report.assessment, MinerAssessment::BelowUnity);
    assert_eq!(report.blocks.len(), 2);
}

#[test]
fn miner_unity_threshold_is_reported_without_claiming_physical_failure() {
    let curve = synthetic_sn_curve();
    let report = miner_damage(
        &curve,
        &[LoadBlock { stress_amplitude_mpa: 200.0, cycles: 40_000 }],
    )
    .unwrap();

    assert_eq!(report.damage_fraction, 1.0);
    assert_eq!(report.assessment, MinerAssessment::AtOrAboveUnity);
}

#[test]
fn paris_law_constant_delta_k_matches_closed_form_increment() {
    let law = ParisLaw::new(1.0e-10, 3.0, 5.0, 20.0).unwrap();
    let result = law.constant_delta_k_growth(0.002, 10.0, 1_000).unwrap();

    assert!((result.growth_rate_m_per_cycle - 1.0e-7).abs() <= 1.0e-20);
    assert!((result.crack_extension_m - 1.0e-4).abs() <= 1.0e-18);
    assert!((result.final_crack_length_m - 0.0021).abs() <= 1.0e-18);
}

#[test]
fn calibrated_domain_boundaries_are_inclusive() {
    let law = ParisLaw::new(1.0e-10, 3.0, 5.0, 20.0).unwrap();
    assert!(law.constant_delta_k_growth(0.001, 5.0, 1).is_ok());
    assert!(law.constant_delta_k_growth(0.001, 20.0, 1).is_ok());
}

#[test]
fn unsupported_extrapolation_is_rejected() {
    let curve = synthetic_sn_curve();
    assert!(matches!(
        curve.cycles_to_failure(99.0),
        Err(DamageError::StressOutsideSnRange { .. })
    ));
    assert!(matches!(
        curve.cycles_to_failure(301.0),
        Err(DamageError::StressOutsideSnRange { .. })
    ));

    let law = ParisLaw::new(1.0e-10, 3.0, 5.0, 20.0).unwrap();
    assert!(matches!(
        law.constant_delta_k_growth(0.001, 4.99, 100),
        Err(DamageError::DeltaKOutsideParisRange { .. })
    ));
}

#[test]
fn malformed_curves_spectra_and_laws_fail_closed() {
    assert!(matches!(
        SnCurve::new(vec![SnPoint { stress_amplitude_mpa: 100.0, cycles_to_failure: 1_000 }]),
        Err(DamageError::InsufficientSnPoints { .. })
    ));

    assert!(matches!(
        SnCurve::new(vec![
            SnPoint { stress_amplitude_mpa: 100.0, cycles_to_failure: 1_000 },
            SnPoint { stress_amplitude_mpa: 110.0, cycles_to_failure: 2_000 },
        ]),
        Err(DamageError::InvalidSnOrdering)
    ));

    let curve = synthetic_sn_curve();
    assert_eq!(miner_damage(&curve, &[]), Err(DamageError::EmptyLoadSpectrum));
    assert_eq!(
        miner_damage(&curve, &[LoadBlock { stress_amplitude_mpa: 200.0, cycles: 0 }]),
        Err(DamageError::ZeroCycleCount)
    );
    assert!(matches!(
        ParisLaw::new(1.0, 3.0, 10.0, 5.0),
        Err(DamageError::InvalidParisRange)
    ));
}
