//! Dependency-free reference implementation of co-polar interferometric
//! closure invariants for a single time/frequency sample.
//!
//! This is a small research oracle, not a VLBI data-reduction pipeline. It
//! assumes one scalar complex visibility per unordered station pair and
//! Hermitian reverse baselines. It does not implement polarization leakage,
//! time/frequency averaging, thermal-noise covariance, imaging, or inference.
//!
//! For reference station 0, the advariant on the closed triangle (0,a,b) is
//! A_0ab = V_0a * (conj(V_ab))^(-1) * V_b0.
//! Under station-based complex gains, A'_0ab = |g_0|^2 A_0ab. A common L2
//! normalization removes this one positive real scale degree of freedom.
//!
//! Reference formalism:
//! Thyagarajan et al., Phys. Rev. D 105, 043019 (2022), DOI
//! 10.1103/PhysRevD.105.043019; and the public ClosureInvariants package:
//! https://github.com/nithyanandan/ClosureInvariants

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Complex64 {
    pub re: f64,
    pub im: f64,
}

impl Complex64 {
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    pub fn conjugate(self) -> Self {
        Self::new(self.re, -self.im)
    }

    pub fn multiply(self, rhs: Self) -> Self {
        Self::new(
            self.re * rhs.re - self.im * rhs.im,
            self.re * rhs.im + self.im * rhs.re,
        )
    }

    pub fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }

    /// Compute 1/conj(z) using a scaled expression to reduce overflow risk.
    fn inverse_conjugate(self) -> Result<Self, ClosureError> {
        if !self.is_finite() {
            return Err(ClosureError::NonFiniteVisibility);
        }
        let scale = self.re.abs().max(self.im.abs());
        if scale == 0.0 {
            return Err(ClosureError::ZeroVisibility);
        }
        let re = self.re / scale;
        let im = self.im / scale;
        let denominator = re * re + im * im;
        let factor = (1.0 / scale) / denominator;
        let result = Self::new(re * factor, im * factor);
        if result.is_finite() {
            Ok(result)
        } else {
            Err(ClosureError::ArithmeticOverflow)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BaselineVisibility {
    /// Station indices. Either orientation is accepted; reverse lookup is
    /// supplied by complex conjugation.
    pub station_a: usize,
    pub station_b: usize,
    pub value: Complex64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClosureError {
    AtLeastThreeStationsRequired,
    InvalidReferenceStation,
    InvalidStationIndex,
    AutocorrelationNotAllowed,
    DuplicateBaseline,
    IncompleteBaselineSet,
    NonFiniteVisibility,
    ZeroVisibility,
    ZeroAdvariantNorm,
    ArithmeticOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalizedAdvariant {
    pub station_a: usize,
    pub station_b: usize,
    pub value: Complex64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClosureInvariantSet {
    pub station_count: usize,
    pub reference_station: usize,
    pub advariants: Vec<NormalizedAdvariant>,
    /// The unit-L2 sphere removes one real scale degree of freedom.
    pub independent_real_degrees_of_freedom: usize,
}

impl ClosureInvariantSet {
    pub fn l2_norm(&self) -> f64 {
        let mut scale = 0.0_f64;
        let mut sum_squares = 1.0_f64;
        for item in &self.advariants {
            for x in [item.value.re.abs(), item.value.im.abs()] {
                if x != 0.0 {
                    if scale < x {
                        let ratio = scale / x;
                        sum_squares = 1.0 + sum_squares * ratio * ratio;
                        scale = x;
                    } else {
                        let ratio = x / scale;
                        sum_squares += ratio * ratio;
                    }
                }
            }
        }
        if scale == 0.0 { 0.0 } else { scale * sum_squares.sqrt() }
    }
}

/// Calculate the complete reference-pinned co-polar closure-invariant set.
///
/// All unordered cross-correlations must be supplied exactly once. The
/// implementation constructs (N-1)(N-2)/2 complex advariants and normalizes
/// their concatenated real/imaginary coordinates by one common L2 norm.
/// This gives N^2-3N+1 independent real degrees of freedom.
///
/// The returned components are not statistically independent. Do not use a
/// diagonal likelihood merely because this representation is convenient;
/// carry the joint covariance or evaluate the likelihood in visibility space.
pub fn closure_invariants(
    station_count: usize,
    reference_station: usize,
    baselines: &[BaselineVisibility],
) -> Result<ClosureInvariantSet, ClosureError> {
    if station_count < 3 {
        return Err(ClosureError::AtLeastThreeStationsRequired);
    }
    if reference_station >= station_count {
        return Err(ClosureError::InvalidReferenceStation);
    }
    let matrix_len = station_count
        .checked_mul(station_count)
        .ok_or(ClosureError::ArithmeticOverflow)?;
    let expected_baselines = station_count
        .checked_mul(station_count - 1)
        .and_then(|x| x.checked_div(2))
        .ok_or(ClosureError::ArithmeticOverflow)?;
    if baselines.len() != expected_baselines {
        return Err(ClosureError::IncompleteBaselineSet);
    }

    let mut matrix: Vec<Option<Complex64>> = vec![None; matrix_len];
    for baseline in baselines {
        let a = baseline.station_a;
        let b = baseline.station_b;
        if a >= station_count || b >= station_count {
            return Err(ClosureError::InvalidStationIndex);
        }
        if a == b {
            return Err(ClosureError::AutocorrelationNotAllowed);
        }
        if !baseline.value.is_finite() {
            return Err(ClosureError::NonFiniteVisibility);
        }
        if baseline.value.re == 0.0 && baseline.value.im == 0.0 {
            return Err(ClosureError::ZeroVisibility);
        }

        let (low, high, canonical) = if a < b {
            (a, b, baseline.value)
        } else {
            (b, a, baseline.value.conjugate())
        };
        let index = low * station_count + high;
        if matrix[index].is_some() {
            return Err(ClosureError::DuplicateBaseline);
        }
        matrix[index] = Some(canonical);
    }
    if matrix.iter().filter(|value| value.is_some()).count() != expected_baselines {
        return Err(ClosureError::IncompleteBaselineSet);
    }

    let visibility = |a: usize, b: usize| -> Result<Complex64, ClosureError> {
        if a < b {
            matrix[a * station_count + b].ok_or(ClosureError::IncompleteBaselineSet)
        } else {
            matrix[b * station_count + a]
                .map(Complex64::conjugate)
                .ok_or(ClosureError::IncompleteBaselineSet)
        }
    };

    let mut advariants = Vec::new();
    for a in 0..station_count {
        if a == reference_station {
            continue;
        }
        for b in (a + 1)..station_count {
            if b == reference_station {
                continue;
            }
            let v_ref_a = visibility(reference_station, a)?;
            let v_a_b = visibility(a, b)?;
            let v_b_ref = visibility(b, reference_station)?;

            let value = v_ref_a
                .multiply(v_a_b.inverse_conjugate()?)
                .multiply(v_b_ref);
            if !value.is_finite() {
                return Err(ClosureError::ArithmeticOverflow);
            }
            advariants.push(NormalizedAdvariant {
                station_a: a,
                station_b: b,
                value,
            });
        }
    }

    // Stable L2 normalization: divide each coordinate by scale and then
    // by sqrt(sum((coordinate/scale)^2)), avoiding a large raw norm.
    let mut scale = 0.0_f64;
    let mut sum_squares = 1.0_f64;
    for item in &advariants {
        for x in [item.value.re.abs(), item.value.im.abs()] {
            if x != 0.0 {
                if scale < x {
                    let ratio = scale / x;
                    sum_squares = 1.0 + sum_squares * ratio * ratio;
                    scale = x;
                } else {
                    let ratio = x / scale;
                    sum_squares += ratio * ratio;
                }
            }
        }
    }
    if scale == 0.0 {
        return Err(ClosureError::ZeroAdvariantNorm);
    }
    let norm_factor = sum_squares.sqrt();
    for item in &mut advariants {
        item.value.re = (item.value.re / scale) / norm_factor;
        item.value.im = (item.value.im / scale) / norm_factor;
    }

    let triangle_count = advariants.len();
    let independent_real_degrees_of_freedom = triangle_count
        .checked_mul(2)
        .and_then(|x| x.checked_sub(1))
        .ok_or(ClosureError::ArithmeticOverflow)?;

    Ok(ClosureInvariantSet {
        station_count,
        reference_station,
        advariants,
        independent_real_degrees_of_freedom,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visibility(a: usize, b: usize) -> Complex64 {
        let x = (a + 1) as f64;
        let y = (b + 1) as f64;
        Complex64::new(0.7 + 0.13 * x - 0.04 * y, 0.11 * x + 0.17 * y)
    }

    fn complete_baselines(n: usize) -> Vec<BaselineVisibility> {
        let mut out = Vec::new();
        for a in 0..n {
            for b in (a + 1)..n {
                out.push(BaselineVisibility {
                    station_a: a,
                    station_b: b,
                    value: visibility(a, b),
                });
            }
        }
        out
    }

    fn corrupt(value: Complex64, gain_a: Complex64, gain_b: Complex64) -> Complex64 {
        gain_a.multiply(value).multiply(gain_b.conjugate())
    }

    #[test]
    fn seven_station_array_has_twenty_nine_real_invariants() {
        let result = closure_invariants(7, 0, &complete_baselines(7)).unwrap();
        assert_eq!(result.advariants.len(), 15);
        assert_eq!(result.independent_real_degrees_of_freedom, 29);
        assert!((result.l2_norm() - 1.0).abs() < 1e-14);
    }

    #[test]
    fn normalized_advariants_are_invariant_to_station_gains() {
        let n = 5;
        let base = complete_baselines(n);
        let gains = [
            Complex64::new(1.2, 0.4),
            Complex64::new(0.8, -0.2),
            Complex64::new(1.1, 0.3),
            Complex64::new(0.9, -0.1),
            Complex64::new(1.3, 0.15),
        ];
        let corrupted: Vec<_> = base.iter().map(|baseline| BaselineVisibility {
            station_a: baseline.station_a,
            station_b: baseline.station_b,
            value: corrupt(
                baseline.value,
                gains[baseline.station_a],
                gains[baseline.station_b],
            ),
        }).collect();

        let clean_set = closure_invariants(n, 0, &base).unwrap();
        let corrupted_set = closure_invariants(n, 0, &corrupted).unwrap();
        assert_eq!(clean_set.advariants.len(), corrupted_set.advariants.len());
        for (clean, observed) in clean_set.advariants.iter().zip(&corrupted_set.advariants) {
            assert!((clean.value.re - observed.value.re).abs() < 2e-14);
            assert!((clean.value.im - observed.value.im).abs() < 2e-14);
        }
    }

    #[test]
    fn reversed_baseline_orientation_is_handled_by_conjugation() {
        let mut base = complete_baselines(4);
        for item in &mut base {
            std::mem::swap(&mut item.station_a, &mut item.station_b);
            item.value = item.value.conjugate();
        }
        let forward = closure_invariants(4, 0, &complete_baselines(4)).unwrap();
        let reverse = closure_invariants(4, 0, &base).unwrap();
        for (a, b) in forward.advariants.iter().zip(&reverse.advariants) {
            assert!((a.value.re - b.value.re).abs() < 1e-14);
            assert!((a.value.im - b.value.im).abs() < 1e-14);
        }
    }

    #[test]
    fn incomplete_duplicate_zero_and_nonfinite_inputs_fail_closed() {
        let mut base = complete_baselines(4);
        let missing = closure_invariants(4, 0, &base[..base.len() - 1]);
        assert_eq!(missing, Err(ClosureError::IncompleteBaselineSet));

        let mut duplicate = base.clone();
        duplicate.push(duplicate[0]);
        let duplicate_result = closure_invariants(4, 0, &duplicate);
        assert_eq!(duplicate_result, Err(ClosureError::IncompleteBaselineSet));

        base[0].value = Complex64::new(0.0, 0.0);
        assert_eq!(closure_invariants(4, 0, &base), Err(ClosureError::ZeroVisibility));

        let mut nonfinite = complete_baselines(4);
        nonfinite[0].value = Complex64::new(f64::NAN, 0.0);
        assert_eq!(closure_invariants(4, 0, &nonfinite), Err(ClosureError::NonFiniteVisibility));
    }
}
