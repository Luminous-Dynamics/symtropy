//! Analytical centrifugal-prestress references for idealized rotating structures.
//!
//! This crate implements only two small-strain, steady-rotation analytical
//! references: a uniform thin ring and a uniform radial spoke. It is not a
//! finite-element solver, a flexible-rotor modal solver, a design allowable,
//! or a structural-safety qualification.
//!
//! All inputs and outputs use SI units. Signed RPM is accepted; steady
//! centrifugal stress depends on omega squared and is therefore invariant to
//! rotation direction in these idealized models.

use std::error::Error;
use std::fmt;

const RPM_TO_RAD_PER_SEC: f64 = std::f64::consts::TAU / 60.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelError {
    NonFiniteInput,
    NonPositiveParameter,
    InvalidStationGrid,
    StationOutsideDomain,
    NonFiniteResult,
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NonFiniteInput => "input must be finite",
            Self::NonPositiveParameter => "density, modulus, radius and area must be positive",
            Self::InvalidStationGrid => {
                "station grid must be strictly increasing and include both 0 and radius"
            }
            Self::StationOutsideDomain => "station radius must lie in [0, radius]",
            Self::NonFiniteResult => "derived result is non-finite or overflowed",
        };
        f.write_str(message)
    }
}

impl Error for ModelError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RingInput {
    pub density_kg_m3: f64,
    pub youngs_modulus_pa: f64,
    pub radius_m: f64,
    pub rpm: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RingResult {
    pub omega_rad_s: f64,
    pub hoop_stress_pa: f64,
    pub hoop_strain: f64,
    pub radial_extension_m: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadialSpokeInput {
    pub density_kg_m3: f64,
    pub youngs_modulus_pa: f64,
    pub radius_m: f64,
    pub rpm: f64,
    pub area_m2: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpokeStation {
    pub radius_m: f64,
    pub axial_force_n: f64,
    pub axial_stress_pa: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RadialSpokeResult {
    pub omega_rad_s: f64,
    pub root_axial_force_n: f64,
    pub root_stress_pa: f64,
    pub tip_extension_m: f64,
    pub stations: Vec<SpokeStation>,
}

/// Convert signed revolutions per minute to signed radians per second.
///
/// Negative RPM is a valid rotation-direction choice. Non-finite inputs and
/// overflowed conversions are rejected.
pub fn rpm_to_omega(rpm: f64) -> Result<f64, ModelError> {
    if !rpm.is_finite() {
        return Err(ModelError::NonFiniteInput);
    }

    let omega = rpm * RPM_TO_RAD_PER_SEC;
    if !omega.is_finite() {
        return Err(ModelError::NonFiniteResult);
    }
    Ok(omega)
}

/// Evaluate the ideal uniform thin-ring centrifugal hoop-stress reference.
///
/// Model:
/// - sigma_theta = rho * omega^2 * R^2
/// - epsilon_theta = sigma_theta / E
/// - delta_R = epsilon_theta * R
///
/// Assumptions and exclusions are documented in the rotating prestress
/// reference design-basis document.
pub fn evaluate_thin_ring(input: RingInput) -> Result<RingResult, ModelError> {
    validate_positive(input.density_kg_m3)?;
    validate_positive(input.youngs_modulus_pa)?;
    validate_positive(input.radius_m)?;

    let omega_rad_s = rpm_to_omega(input.rpm)?;
    let omega_squared = omega_rad_s * omega_rad_s;
    let radius_squared = input.radius_m * input.radius_m;
    if !omega_squared.is_finite() || !radius_squared.is_finite() {
        return Err(ModelError::NonFiniteResult);
    }

    let hoop_stress_pa =
        input.density_kg_m3 * omega_squared * radius_squared;
    let hoop_strain = hoop_stress_pa / input.youngs_modulus_pa;
    let radial_extension_m = hoop_strain * input.radius_m;

    if !hoop_stress_pa.is_finite()
        || !hoop_strain.is_finite()
        || !radial_extension_m.is_finite()
    {
        return Err(ModelError::NonFiniteResult);
    }

    Ok(RingResult {
        omega_rad_s,
        hoop_stress_pa,
        hoop_strain,
        radial_extension_m,
    })
}

/// Evaluate a uniform radial spoke in steady rotation.
///
/// The spoke is fixed at the hub (r=0), free at the tip (r=R), and loaded by
/// its own centrifugal body force on the undeformed reference line:
///
/// N(x) = 0.5 * rho * A * omega^2 * (R^2 - x^2)
/// sigma(x) = N(x) / A
/// u(R) = rho * omega^2 * R^3 / (3 * E)
///
/// sample_radii_m must be strictly increasing and include both 0 and R so
/// that root and free-tip boundary values are visible in the result.
pub fn evaluate_radial_spoke(
    input: RadialSpokeInput,
    sample_radii_m: &[f64],
) -> Result<RadialSpokeResult, ModelError> {
    validate_positive(input.density_kg_m3)?;
    validate_positive(input.youngs_modulus_pa)?;
    validate_positive(input.radius_m)?;
    validate_positive(input.area_m2)?;

    if sample_radii_m.len() < 2
        || sample_radii_m.first().copied() != Some(0.0)
        || sample_radii_m.last().copied() != Some(input.radius_m)
    {
        return Err(ModelError::InvalidStationGrid);
    }

    let mut previous = None;
    for &station in sample_radii_m {
        if !station.is_finite() {
            return Err(ModelError::NonFiniteInput);
        }
        if !(0.0..=input.radius_m).contains(&station) {
            return Err(ModelError::StationOutsideDomain);
        }
        if let Some(previous_radius) = previous
            && station <= previous_radius
        {
            return Err(ModelError::InvalidStationGrid);
        }
        previous = Some(station);
    }

    let omega_rad_s = rpm_to_omega(input.rpm)?;
    let omega_squared = omega_rad_s * omega_rad_s;
    let radius_squared = input.radius_m * input.radius_m;
    if !omega_squared.is_finite() || !radius_squared.is_finite() {
        return Err(ModelError::NonFiniteResult);
    }

    let root_axial_force_n =
        0.5 * input.density_kg_m3 * input.area_m2 * omega_squared * radius_squared;
    let root_stress_pa =
        0.5 * input.density_kg_m3 * omega_squared * radius_squared;
    let radius_cubed = radius_squared * input.radius_m;
    let tip_extension_m = input.density_kg_m3 * omega_squared * radius_cubed
        / (3.0 * input.youngs_modulus_pa);

    if !root_axial_force_n.is_finite()
        || !root_stress_pa.is_finite()
        || !tip_extension_m.is_finite()
    {
        return Err(ModelError::NonFiniteResult);
    }

    let mut stations = Vec::with_capacity(sample_radii_m.len());
    for &station_radius_m in sample_radii_m {
        let station_squared = station_radius_m * station_radius_m;
        let remaining_radius_squared = radius_squared - station_squared;

        let axial_force_n = 0.5
            * input.density_kg_m3
            * input.area_m2
            * omega_squared
            * remaining_radius_squared;
        let axial_stress_pa = 0.5
            * input.density_kg_m3
            * omega_squared
            * remaining_radius_squared;

        if !axial_force_n.is_finite() || !axial_stress_pa.is_finite() {
            return Err(ModelError::NonFiniteResult);
        }

        stations.push(SpokeStation {
            radius_m: station_radius_m,
            axial_force_n,
            axial_stress_pa,
        });
    }

    Ok(RadialSpokeResult {
        omega_rad_s,
        root_axial_force_n,
        root_stress_pa,
        tip_extension_m,
        stations,
    })
}

fn validate_positive(value: f64) -> Result<(), ModelError> {
    if !value.is_finite() {
        return Err(ModelError::NonFiniteInput);
    }
    if value <= 0.0 {
        return Err(ModelError::NonPositiveParameter);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const REL_TOL: f64 = 1.0e-12;
    const ABS_TOL: f64 = 1.0e-12;

    fn close(actual: f64, expected: f64) {
        let tolerance = ABS_TOL.max(REL_TOL * expected.abs());
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:.3e}"
        );
    }

    fn ring_input(rpm: f64) -> RingInput {
        RingInput {
            density_kg_m3: 2700.0,
            youngs_modulus_pa: 70.0e9,
            radius_m: 100.0,
            rpm,
        }
    }

    fn spoke_input(rpm: f64) -> RadialSpokeInput {
        RadialSpokeInput {
            density_kg_m3: 2700.0,
            youngs_modulus_pa: 70.0e9,
            radius_m: 100.0,
            rpm,
            area_m2: 0.01,
        }
    }

    #[test]
    fn ring_matches_independent_two_rpm_reference() {
        let result = evaluate_thin_ring(ring_input(2.0)).unwrap();
        close(result.omega_rad_s, 0.209_439_510_239_319_53);
        close(result.hoop_stress_pa, 1_184_352.528_130_723);
        close(result.hoop_strain, 0.000_016_919_321_830_438_897);
        close(result.radial_extension_m, 0.001_691_932_183_043_889_7);
    }

    #[test]
    fn ring_stress_and_extension_scale_with_rpm_squared() {
        let two_rpm = evaluate_thin_ring(ring_input(2.0)).unwrap();
        let four_rpm = evaluate_thin_ring(ring_input(4.0)).unwrap();
        close(four_rpm.hoop_stress_pa / two_rpm.hoop_stress_pa, 4.0);
        close(
            four_rpm.radial_extension_m / two_rpm.radial_extension_m,
            4.0,
        );
    }

    #[test]
    fn ring_steady_centrifugal_result_is_direction_invariant() {
        let positive = evaluate_thin_ring(ring_input(2.0)).unwrap();
        let negative = evaluate_thin_ring(ring_input(-2.0)).unwrap();
        close(positive.hoop_stress_pa, negative.hoop_stress_pa);
        close(positive.radial_extension_m, negative.radial_extension_m);
        close(positive.omega_rad_s, -negative.omega_rad_s);
    }

    #[test]
    fn radial_spoke_matches_independent_force_stress_and_extension() {
        let result =
            evaluate_radial_spoke(spoke_input(2.0), &[0.0, 50.0, 100.0]).unwrap();

        close(result.omega_rad_s, 0.209_439_510_239_319_53);
        close(result.root_axial_force_n, 5_921.762_640_653_614);
        close(result.root_stress_pa, 592_176.264_065_361_5);
        close(result.tip_extension_m, 0.000_563_977_394_347_963_3);

        assert_eq!(result.stations.len(), 3);
        close(result.stations[0].axial_force_n, result.root_axial_force_n);
        close(result.stations[0].axial_stress_pa, result.root_stress_pa);
        close(result.stations[1].axial_force_n, 4_441.321_980_490_21);
        close(result.stations[1].axial_stress_pa, 444_132.198_049_021);
        close(result.stations[2].axial_force_n, 0.0);
        close(result.stations[2].axial_stress_pa, 0.0);
    }

    #[test]
    fn spoke_root_force_equals_integrated_centrifugal_body_force() {
        let input = spoke_input(2.0);
        let result =
            evaluate_radial_spoke(input, &[0.0, input.radius_m]).unwrap();
        let omega = rpm_to_omega(input.rpm).unwrap();
        let integrated_load =
            0.5 * input.density_kg_m3 * input.area_m2 * omega * omega
                * input.radius_m * input.radius_m;
        close(result.root_axial_force_n, integrated_load);
    }

    #[test]
    fn spoke_force_stress_and_extension_scale_with_rpm_squared() {
        let two_rpm =
            evaluate_radial_spoke(spoke_input(2.0), &[0.0, 100.0]).unwrap();
        let four_rpm =
            evaluate_radial_spoke(spoke_input(4.0), &[0.0, 100.0]).unwrap();
        close(
            four_rpm.root_axial_force_n / two_rpm.root_axial_force_n,
            4.0,
        );
        close(four_rpm.root_stress_pa / two_rpm.root_stress_pa, 4.0);
        close(
            four_rpm.tip_extension_m / two_rpm.tip_extension_m,
            4.0,
        );
    }

    #[test]
    fn zero_spin_has_zero_steady_centrifugal_response() {
        let ring = evaluate_thin_ring(ring_input(0.0)).unwrap();
        close(ring.hoop_stress_pa, 0.0);
        close(ring.radial_extension_m, 0.0);

        let spoke =
            evaluate_radial_spoke(spoke_input(0.0), &[0.0, 50.0, 100.0])
                .unwrap();
        close(spoke.root_axial_force_n, 0.0);
        close(spoke.root_stress_pa, 0.0);
        close(spoke.tip_extension_m, 0.0);
        for station in spoke.stations {
            close(station.axial_force_n, 0.0);
            close(station.axial_stress_pa, 0.0);
        }
    }

    #[test]
    fn invalid_positive_parameters_are_rejected() {
        let mut invalid_ring = ring_input(2.0);
        invalid_ring.radius_m = 0.0;
        assert_eq!(
            evaluate_thin_ring(invalid_ring),
            Err(ModelError::NonPositiveParameter)
        );

        let mut invalid_ring = ring_input(2.0);
        invalid_ring.youngs_modulus_pa = f64::NAN;
        assert_eq!(
            evaluate_thin_ring(invalid_ring),
            Err(ModelError::NonFiniteInput)
        );

        let mut invalid_spoke = spoke_input(2.0);
        invalid_spoke.area_m2 = -0.01;
        assert_eq!(
            evaluate_radial_spoke(invalid_spoke, &[0.0, 100.0]),
            Err(ModelError::NonPositiveParameter)
        );
    }

    #[test]
    fn station_grid_must_cover_domain_and_be_strictly_increasing() {
        let input = spoke_input(2.0);
        assert_eq!(
            evaluate_radial_spoke(input, &[0.0, 50.0]),
            Err(ModelError::InvalidStationGrid)
        );
        assert_eq!(
            evaluate_radial_spoke(input, &[0.0, 50.0, 50.0, 100.0]),
            Err(ModelError::InvalidStationGrid)
        );
        assert_eq!(
            evaluate_radial_spoke(input, &[0.0, 101.0, 100.0]),
            Err(ModelError::StationOutsideDomain)
        );
    }

    #[test]
    fn overflowed_derived_values_are_rejected() {
        let mut invalid_ring = ring_input(f64::MAX);
        invalid_ring.density_kg_m3 = 1.0;
        invalid_ring.youngs_modulus_pa = 1.0;
        invalid_ring.radius_m = 1.0;
        assert_eq!(
            evaluate_thin_ring(invalid_ring),
            Err(ModelError::NonFiniteResult)
        );

        let mut invalid_spoke = spoke_input(2.0);
        invalid_spoke.radius_m = 1.0e200;
        assert_eq!(
            evaluate_radial_spoke(
                invalid_spoke,
                &[0.0, invalid_spoke.radius_m]
            ),
            Err(ModelError::NonFiniteResult)
        );
    }

    #[test]
    fn non_finite_spin_is_rejected() {
        assert_eq!(rpm_to_omega(f64::INFINITY), Err(ModelError::NonFiniteInput));
    }
}
