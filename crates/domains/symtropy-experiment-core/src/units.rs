// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Small, explicit unit registry for the executable arithmetic graph.
//!
//! Values are stored and calculated in canonical base units, then converted back to the
//! variable's declared unit at the API boundary. This intentionally supports only scale units;
//! affine units such as degrees Celsius require a separate offset-aware quantity model.

use crate::Dimension;

/// Bump when supported symbols, conversion factors, or parser semantics change.
/// Serialized results carry this value so unit interpretation is reproducible.
pub const UNIT_REGISTRY_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitDefinition {
    pub dimension: Dimension,
    /// Positive factor that converts one declared unit into the graph's canonical unit.
    pub scale_to_canonical: f64,
}

const fn dimension(exponents: [i8; 8]) -> Dimension {
    Dimension { exponents }
}

const DIMENSIONLESS: Dimension = Dimension::DIMENSIONLESS;
const LENGTH: Dimension = dimension([1, 0, 0, 0, 0, 0, 0, 0]);
const AREA: Dimension = dimension([2, 0, 0, 0, 0, 0, 0, 0]);
const VOLUME: Dimension = dimension([3, 0, 0, 0, 0, 0, 0, 0]);
const MASS: Dimension = dimension([0, 1, 0, 0, 0, 0, 0, 0]);
const TIME: Dimension = dimension([0, 0, 1, 0, 0, 0, 0, 0]);
const CURRENT: Dimension = dimension([0, 0, 0, 1, 0, 0, 0, 0]);
const TEMPERATURE: Dimension = dimension([0, 0, 0, 0, 1, 0, 0, 0]);
const AMOUNT: Dimension = dimension([0, 0, 0, 0, 0, 1, 0, 0]);
const LUMINOUS_INTENSITY: Dimension = dimension([0, 0, 0, 0, 0, 0, 1, 0]);
const FREQUENCY: Dimension = dimension([0, 0, -1, 0, 0, 0, 0, 0]);
const FORCE: Dimension = dimension([1, 1, -2, 0, 0, 0, 0, 0]);
const PRESSURE: Dimension = dimension([-1, 1, -2, 0, 0, 0, 0, 0]);
const ENERGY: Dimension = dimension([2, 1, -2, 0, 0, 0, 0, 0]);
const POWER: Dimension = dimension([2, 1, -3, 0, 0, 0, 0, 0]);
const CHARGE: Dimension = dimension([0, 0, 1, 1, 0, 0, 0, 0]);
const VOLTAGE: Dimension = dimension([2, 1, -3, -1, 0, 0, 0, 0]);
const RESISTANCE: Dimension = dimension([2, 1, -3, -2, 0, 0, 0, 0]);
const CAPACITANCE: Dimension = dimension([-2, -1, 4, 2, 0, 0, 0, 0]);
const MAGNETIC_FLUX_DENSITY: Dimension = dimension([0, 1, -2, -1, 0, 0, 0, 0]);

fn atom_definition(symbol: &str) -> Option<UnitDefinition> {
    let (dimension, scale_to_canonical) = match symbol {
        "1" | "tooth" => (DIMENSIONLESS, 1.0),
        "m" => (LENGTH, 1.0),
        "km" => (LENGTH, 1_000.0),
        "cm" => (LENGTH, 0.01),
        "mm" => (LENGTH, 0.001),
        "um" => (LENGTH, 0.000_001),
        "in" => (LENGTH, 0.0254),
        "ft" => (LENGTH, 0.3048),
        "m2" => (AREA, 1.0),
        "L" => (VOLUME, 0.001),
        "mL" => (VOLUME, 0.000_001),
        "kg" => (MASS, 1.0),
        "g" => (MASS, 0.001),
        "mg" => (MASS, 0.000_001),
        "tonne" => (MASS, 1_000.0),
        "s" => (TIME, 1.0),
        "ms" => (TIME, 0.001),
        "min" => (TIME, 60.0),
        "h" => (TIME, 3_600.0),
        "A" => (CURRENT, 1.0),
        "K" => (TEMPERATURE, 1.0),
        "mol" => (AMOUNT, 1.0),
        "cd" => (LUMINOUS_INTENSITY, 1.0),
        "rad" => (Dimension::ANGLE, 1.0),
        "deg" => (Dimension::ANGLE, std::f64::consts::PI / 180.0),
        "Hz" => (FREQUENCY, 1.0),
        "N" => (FORCE, 1.0),
        "kN" => (FORCE, 1_000.0),
        "Pa" => (PRESSURE, 1.0),
        "kPa" => (PRESSURE, 1_000.0),
        "MPa" => (PRESSURE, 1_000_000.0),
        "bar" => (PRESSURE, 100_000.0),
        "J" => (ENERGY, 1.0),
        "kJ" => (ENERGY, 1_000.0),
        "Wh" => (ENERGY, 3_600.0),
        "W" => (POWER, 1.0),
        "kW" => (POWER, 1_000.0),
        "C" => (CHARGE, 1.0),
        "V" => (VOLTAGE, 1.0),
        "ohm" => (RESISTANCE, 1.0),
        "F" => (CAPACITANCE, 1.0),
        "T" => (MAGNETIC_FLUX_DENSITY, 1.0),
        _ => return None,
    };
    Some(UnitDefinition {
        dimension,
        scale_to_canonical,
    })
}

/// Resolve a unit expression against the intentionally small, explicit unit registry.
///
/// Grammar uses registered atom names joined by * and /, with optional signed integer powers
/// such as m^2, s^-2, kg*m/s^2, and rad/s. Parentheses, implicit products, and affine units are
/// not supported. Unknown units fail closed; they are never silently treated as dimensionless
/// or as scale 1.
pub fn unit_definition(unit: &str) -> Result<UnitDefinition, String> {
    let expression = unit.trim();
    if expression.is_empty() {
        return Err("unit expression must not be empty".to_owned());
    }
    if expression.chars().any(char::is_whitespace) {
        return Err(format!(
            "unit expression '{expression}' contains whitespace; use registered symbols and explicit operators"
        ));
    }

    let mut result_exponents = [0_i8; 8];
    let mut result_scale = 1.0_f64;
    let mut token_start = 0_usize;
    let mut operator_power = 1_i8;

    for (index, ch) in expression.char_indices() {
        if ch != '*' && ch != '/' {
            continue;
        }
        apply_term(
            &expression[token_start..index],
            operator_power,
            &mut result_exponents,
            &mut result_scale,
        )?;
        token_start = index + ch.len_utf8();
        operator_power = if ch == '/' { -1 } else { 1 };
    }
    apply_term(
        &expression[token_start..],
        operator_power,
        &mut result_exponents,
        &mut result_scale,
    )?;

    if !result_scale.is_finite() || result_scale <= 0.0 {
        return Err(format!(
            "unit expression '{expression}' has an unsupported or non-finite scale"
        ));
    }

    Ok(UnitDefinition {
        dimension: Dimension {
            exponents: result_exponents,
        },
        scale_to_canonical: result_scale,
    })
}

fn apply_term(
    term: &str,
    operator_power: i8,
    result_exponents: &mut [i8; 8],
    result_scale: &mut f64,
) -> Result<(), String> {
    if term.is_empty() {
        return Err("unit expression contains an empty term".to_owned());
    }

    let (symbol, raw_power) = match term.rsplit_once('^') {
        Some((symbol, power)) => {
            if symbol.is_empty() {
                return Err(format!("unit term '{term}' has no symbol"));
            }
            let power = power
                .parse::<i8>()
                .map_err(|_| format!("unit term '{term}' has an invalid integer exponent"))?;
            if !(-8..=8).contains(&power) {
                return Err(format!(
                    "unit term '{term}' exponent is outside the supported range -8..=8"
                ));
            }
            (symbol, power)
        }
        None => (term, 1_i8),
    };

    let power = raw_power
        .checked_mul(operator_power)
        .ok_or_else(|| format!("unit term '{term}' exponent overflow"))?;
    let atom = atom_definition(symbol)
        .ok_or_else(|| format!("unit symbol '{symbol}' is not in the supported registry"))?;

    for (index, atom_exponent) in atom.dimension.exponents.iter().enumerate() {
        let contribution = atom_exponent
            .checked_mul(power)
            .ok_or_else(|| format!("unit term '{term}' dimension exponent overflow"))?;
        result_exponents[index] = result_exponents[index]
            .checked_add(contribution)
            .ok_or_else(|| format!("unit expression dimension overflow at axis {index}"))?;
    }

    *result_scale *= atom.scale_to_canonical.powi(i32::from(power));
    if !result_scale.is_finite() || *result_scale <= 0.0 {
        return Err(format!(
            "unit expression term '{term}' produces an unsupported scale"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_prefixes_share_dimensions_and_convert_to_canonical_scale() {
        let centimetre = unit_definition("cm").unwrap();
        let metre = unit_definition("m").unwrap();
        assert_eq!(centimetre.dimension, metre.dimension);
        assert!((centimetre.scale_to_canonical - 0.01).abs() < f64::EPSILON);
        assert_eq!(metre.scale_to_canonical, 1.0);
    }

    #[test]
    fn compound_units_derive_dimensions_and_scales() {
        let acceleration = unit_definition("m/s^2").unwrap();
        assert_eq!(acceleration.dimension, dimension([1, 0, -2, 0, 0, 0, 0, 0]));
        let kilonewton = unit_definition("kN").unwrap();
        assert_eq!(kilonewton.dimension, FORCE);
        assert_eq!(kilonewton.scale_to_canonical, 1_000.0);
    }

    #[test]
    fn degrees_convert_to_semantic_radians() {
        let degrees = unit_definition("deg").unwrap();
        let radians = unit_definition("rad").unwrap();
        assert_eq!(degrees.dimension, Dimension::ANGLE);
        assert_eq!(radians.dimension, Dimension::ANGLE);
        assert!((180.0 * degrees.scale_to_canonical - std::f64::consts::PI).abs() < 1e-12);
    }

    #[test]
    fn unknown_and_affine_units_fail_closed() {
        assert!(unit_definition("widgets").is_err());
        assert!(unit_definition("degC").is_err());
        assert!(unit_definition("m//s").is_err());
    }
}
