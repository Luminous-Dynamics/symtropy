use material_damage_reference::fatigue::{LoadBlock, SnCurve, SnPoint, miner_damage};
use material_damage_reference::paris::ParisLaw;

fn curve() -> SnCurve {
    SnCurve::new(vec![
        SnPoint {
            stress_amplitude_mpa: 300.0,
            cycles_to_failure: 10_000,
        },
        SnPoint {
            stress_amplitude_mpa: 200.0,
            cycles_to_failure: 40_000,
        },
        SnPoint {
            stress_amplitude_mpa: 100.0,
            cycles_to_failure: 1_000_000,
        },
    ])
    .unwrap()
}

fn close(left: f64, right: f64) {
    let tolerance = 1.0e-12_f64.max(1.0e-10 * left.abs().max(right.abs()));
    assert!(
        (left - right).abs() <= tolerance,
        "left={left}, right={right}, tolerance={tolerance}"
    );
}

#[test]
fn miner_damage_is_invariant_to_splitting_a_load_block() {
    let sn = curve();
    let whole = miner_damage(
        &sn,
        &[LoadBlock {
            stress_amplitude_mpa: 200.0,
            cycles: 12_000,
        }],
    )
    .unwrap();
    let split = miner_damage(
        &sn,
        &[
            LoadBlock {
                stress_amplitude_mpa: 200.0,
                cycles: 5_000,
            },
            LoadBlock {
                stress_amplitude_mpa: 200.0,
                cycles: 7_000,
            },
        ],
    )
    .unwrap();

    close(whole.damage_fraction, split.damage_fraction);
    assert_eq!(whole.total_cycles, split.total_cycles);
}

#[test]
fn linear_miner_damage_is_permutation_invariant_for_declared_blocks() {
    let sn = curve();
    let a = LoadBlock {
        stress_amplitude_mpa: 200.0,
        cycles: 10_000,
    };
    let b = LoadBlock {
        stress_amplitude_mpa: 300.0,
        cycles: 2_000,
    };
    let c = LoadBlock {
        stress_amplitude_mpa: 100.0,
        cycles: 100_000,
    };

    let forward = miner_damage(&sn, &[a, b, c]).unwrap();
    let reverse = miner_damage(&sn, &[c, b, a]).unwrap();
    close(forward.damage_fraction, reverse.damage_fraction);
    assert_eq!(forward.total_cycles, reverse.total_cycles);
}

#[test]
fn greater_stress_amplitude_never_increases_predicted_sn_life() {
    let sn = curve();
    let low = sn.cycles_to_failure(150.0).unwrap();
    let high = sn.cycles_to_failure(250.0).unwrap();
    assert!(high < low);
}

#[test]
fn paris_growth_is_linear_in_cycles_and_coefficient_at_fixed_delta_k() {
    let law = ParisLaw::new(1.0e-10, 3.0, 5.0, 20.0).unwrap();
    let one = law.constant_delta_k_growth(0.001, 10.0, 1_000).unwrap();
    let two = law.constant_delta_k_growth(0.001, 10.0, 2_000).unwrap();
    close(two.crack_extension_m, 2.0 * one.crack_extension_m);

    let doubled_coefficient = ParisLaw::new(2.0e-10, 3.0, 5.0, 20.0).unwrap();
    let doubled = doubled_coefficient
        .constant_delta_k_growth(0.001, 10.0, 1_000)
        .unwrap();
    close(doubled.crack_extension_m, 2.0 * one.crack_extension_m);
}

#[test]
fn paris_growth_increases_with_delta_k_inside_declared_regime() {
    let law = ParisLaw::new(1.0e-10, 3.0, 5.0, 20.0).unwrap();
    let low = law.constant_delta_k_growth(0.001, 8.0, 100).unwrap();
    let high = law.constant_delta_k_growth(0.001, 12.0, 100).unwrap();
    assert!(high.crack_extension_m > low.crack_extension_m);
}
