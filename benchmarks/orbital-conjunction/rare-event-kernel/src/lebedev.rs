// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic low-order Lebedev rules and sphere-integrand adequacy diagnostics.
//!
//! The supported 6-, 14-, and 26-point rules have algebraic precisions 3, 5,
//! and 7, respectively. They are deliberately a small independently testable
//! subset, not a replacement for NASA CARA's production 5,810-point rule.
//!
//! NASA's public Pc3D_Hall implementation counts Lebedev points whose
//! integrand exceeds 1e-3 times the maximum integrand as a diagnostic of
//! whether enough nodes materially contribute to the collision-rate integral.
//! This module exposes that threshold and additional contribution diagnostics.
//!
//! References:
//! - Lebedev & Laikov (1999), "A quadrature formula for the sphere of the 131st
//!   algebraic order of accuracy", Doklady Mathematics 59(3), 477-481.
//! - NASA CARA Analysis Tools, Pc3D_Hall.m, Ncdot sphere-integration path.

use crate::KernelError;
use core::f64::consts::PI;

/// One unit-sphere quadrature node and its solid-angle weight.
///
/// Weights are expressed for integration over solid angle and sum to 4π.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SphereQuadratureNode {
    /// Cartesian x-coordinate on the unit sphere.
    pub x: f64,
    /// Cartesian y-coordinate on the unit sphere.
    pub y: f64,
    /// Cartesian z-coordinate on the unit sphere.
    pub z: f64,
    /// Positive solid-angle quadrature weight.
    pub weight: f64,
}

impl SphereQuadratureNode {
    /// Return the squared Euclidean norm of this node.
    pub fn norm_squared(self) -> f64 {
        self.x
            .mul_add(self.x, self.y.mul_add(self.y, self.z * self.z))
    }
}

/// Supported low-order Lebedev sphere rules.
///
/// Algebraic precision is the maximum total polynomial degree integrated
/// exactly in exact arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LebedevRule {
    /// Six axis nodes; algebraic precision 3.
    Points6,
    /// Six axis and eight cube-corner nodes; algebraic precision 5.
    Points14,
    /// Six axis, eight cube-corner and twelve edge-midpoint nodes;
    /// algebraic precision 7.
    Points26,
    /// NASA/CARA 5,810-node rule; algebraic precision 131.
    Points5810,
}

impl LebedevRule {
    /// Number of nodes in the rule.
    pub const fn node_count(self) -> usize {
        match self {
            Self::Points6 => 6,
            Self::Points14 => 14,
            Self::Points26 => 26,
            Self::Points5810 => 5810,
        }
    }

    /// Algebraic polynomial precision of the rule.
    pub const fn algebraic_precision(self) -> usize {
        match self {
            Self::Points6 => 3,
            Self::Points14 => 5,
            Self::Points26 => 7,
            Self::Points5810 => 131,
        }
    }
}

/// A validated deterministic Lebedev rule.
///
/// The fields are private so callers cannot mutate nodes or weights after the
/// built-in rule has been validated.
#[derive(Debug, Clone, PartialEq)]
pub struct LebedevSphereRule {
    rule: LebedevRule,
    nodes: Vec<SphereQuadratureNode>,
    weight_sum: f64,
}

impl LebedevSphereRule {
    /// Rule identifier and known algebraic precision.
    pub fn rule(&self) -> LebedevRule {
        self.rule
    }

    /// Nodes in deterministic order.
    pub fn nodes(&self) -> &[SphereQuadratureNode] {
        &self.nodes
    }

    /// Sum of the solid-angle weights.
    pub fn weight_sum(&self) -> f64 {
        self.weight_sum
    }
}

/// Build one of the supported positive-weight Lebedev rules.
///
/// The node order is stable and nested: the first six points are the axis
/// points, followed by eight cube corners, followed by twelve edge midpoints.
pub fn lebedev_sphere_rule(rule: LebedevRule) -> Result<LebedevSphereRule, KernelError> {
    if rule == LebedevRule::Points5810 {
        return lebedev_sphere_rule_5810();
    }

    let mut nodes = Vec::with_capacity(rule.node_count());
    let axis_weight = match rule {
        LebedevRule::Points6 => 4.0 * PI / 6.0,
        LebedevRule::Points14 => 4.0 * PI / 15.0,
        LebedevRule::Points26 => 4.0 * PI / 21.0,
        LebedevRule::Points5810 => unreachable!("handled before low-order construction"),
    };

    nodes.extend([
        SphereQuadratureNode {
            x: 1.0,
            y: 0.0,
            z: 0.0,
            weight: axis_weight,
        },
        SphereQuadratureNode {
            x: -1.0,
            y: 0.0,
            z: 0.0,
            weight: axis_weight,
        },
        SphereQuadratureNode {
            x: 0.0,
            y: 1.0,
            z: 0.0,
            weight: axis_weight,
        },
        SphereQuadratureNode {
            x: 0.0,
            y: -1.0,
            z: 0.0,
            weight: axis_weight,
        },
        SphereQuadratureNode {
            x: 0.0,
            y: 0.0,
            z: 1.0,
            weight: axis_weight,
        },
        SphereQuadratureNode {
            x: 0.0,
            y: 0.0,
            z: -1.0,
            weight: axis_weight,
        },
    ]);

    if matches!(rule, LebedevRule::Points14 | LebedevRule::Points26) {
        let coordinate = 1.0 / 3.0_f64.sqrt();
        let normalized_weight = if rule == LebedevRule::Points14 {
            3.0 / 40.0
        } else {
            9.0 / 280.0
        };
        let weight = 4.0 * PI * normalized_weight;
        for sx in [-1.0, 1.0] {
            for sy in [-1.0, 1.0] {
                for sz in [-1.0, 1.0] {
                    nodes.push(SphereQuadratureNode {
                        x: sx * coordinate,
                        y: sy * coordinate,
                        z: sz * coordinate,
                        weight,
                    });
                }
            }
        }
    }

    if rule == LebedevRule::Points26 {
        let coordinate = 1.0 / 2.0_f64.sqrt();
        let weight = 4.0 * PI * (4.0 / 105.0);

        for sx in [-1.0, 1.0] {
            for sy in [-1.0, 1.0] {
                nodes.push(SphereQuadratureNode {
                    x: sx * coordinate,
                    y: sy * coordinate,
                    z: 0.0,
                    weight,
                });
            }
        }
        for sx in [-1.0, 1.0] {
            for sz in [-1.0, 1.0] {
                nodes.push(SphereQuadratureNode {
                    x: sx * coordinate,
                    y: 0.0,
                    z: sz * coordinate,
                    weight,
                });
            }
        }
        for sy in [-1.0, 1.0] {
            for sz in [-1.0, 1.0] {
                nodes.push(SphereQuadratureNode {
                    x: 0.0,
                    y: sy * coordinate,
                    z: sz * coordinate,
                    weight,
                });
            }
        }
    }

    validate_nodes(&nodes, rule.node_count())?;
    let weight_sum = compensated_sum(nodes.iter().map(|node| node.weight));
    let expected_weight_sum = 4.0 * PI;
    if !weight_sum.is_finite()
        || (weight_sum - expected_weight_sum).abs() > 64.0 * f64::EPSILON * expected_weight_sum
    {
        return Err(KernelError::InvalidSphereQuadratureWeights);
    }

    Ok(LebedevSphereRule {
        rule,
        nodes,
        weight_sum,
    })
}

// NASA CARA Analysis Tools getLebedevSphere.m, case 5810.
// The 144 symmetry-orbit records below are the published orbit parameters
// (type, a, b, normalized weight). Coordinates are expanded algorithmically
// by the standard octahedral symmetry groups; each node weight is 4π times
// the listed normalized weight.
//
// Source: https://github.com/nasa/CARA_Analysis_Tools/blob/43f0160f94d76a91805d86521c3e8fa0d09ecd68/DistributedMatlab/ProbabilityOfCollision/Utils/getLebedevSphere.m
//
// Preserve the published decimal coefficients verbatim; they are source data,
// not hand-authored f64 constants whose excess precision should be rounded.
#[allow(clippy::excessive_precision)]
const LEBEDEV_5810_ORBITS: &[(u8, f64, f64, f64)] = &[
    (1, 0.0, 0.6112185773983089E-1, 0.9735347946175486E-5),
    (2, 0.0, 0.6112185773983089E-1, 0.1907581241803167E-3),
    (3, 0.0, 0.6112185773983089E-1, 0.1901059546737578E-3),
    (
        4,
        0.1182361662400277E-1,
        0.6112185773983089E-1,
        0.3926424538919212E-4,
    ),
    (
        4,
        0.3062145009138958E-1,
        0.6112185773983089E-1,
        0.6667905467294382E-4,
    ),
    (
        4,
        0.5329794036834243E-1,
        0.6112185773983089E-1,
        0.8868891315019135E-4,
    ),
    (
        4,
        0.7848165532862220E-1,
        0.6112185773983089E-1,
        0.1066306000958872E-3,
    ),
    (
        4,
        0.1054038157636201E+0,
        0.6112185773983089E-1,
        0.1214506743336128E-3,
    ),
    (
        4,
        0.1335577797766211E+0,
        0.6112185773983089E-1,
        0.1338054681640871E-3,
    ),
    (
        4,
        0.1625769955502252E+0,
        0.6112185773983089E-1,
        0.1441677023628504E-3,
    ),
    (
        4,
        0.1921787193412792E+0,
        0.6112185773983089E-1,
        0.1528880200826557E-3,
    ),
    (
        4,
        0.2221340534690548E+0,
        0.6112185773983089E-1,
        0.1602330623773609E-3,
    ),
    (
        4,
        0.2522504912791132E+0,
        0.6112185773983089E-1,
        0.1664102653445244E-3,
    ),
    (
        4,
        0.2823610860679697E+0,
        0.6112185773983089E-1,
        0.1715845854011323E-3,
    ),
    (
        4,
        0.3123173966267560E+0,
        0.6112185773983089E-1,
        0.1758901000133069E-3,
    ),
    (
        4,
        0.3419847036953789E+0,
        0.6112185773983089E-1,
        0.1794382485256736E-3,
    ),
    (
        4,
        0.3712386456999758E+0,
        0.6112185773983089E-1,
        0.1823238106757407E-3,
    ),
    (
        4,
        0.3999627649876828E+0,
        0.6112185773983089E-1,
        0.1846293252959976E-3,
    ),
    (
        4,
        0.4280466458648093E+0,
        0.6112185773983089E-1,
        0.1864284079323098E-3,
    ),
    (
        4,
        0.4553844360185711E+0,
        0.6112185773983089E-1,
        0.1877882694626914E-3,
    ),
    (
        4,
        0.4818736094437834E+0,
        0.6112185773983089E-1,
        0.1887716321852025E-3,
    ),
    (
        4,
        0.5074138709260629E+0,
        0.6112185773983089E-1,
        0.1894381638175673E-3,
    ),
    (
        4,
        0.5319061304570707E+0,
        0.6112185773983089E-1,
        0.1898454899533629E-3,
    ),
    (
        4,
        0.5552514978677286E+0,
        0.6112185773983089E-1,
        0.1900497929577815E-3,
    ),
    (
        4,
        0.5981009025246183E+0,
        0.6112185773983089E-1,
        0.1900671501924092E-3,
    ),
    (
        4,
        0.6173990192228116E+0,
        0.6112185773983089E-1,
        0.1899837555533510E-3,
    ),
    (
        4,
        0.6351365239411131E+0,
        0.6112185773983089E-1,
        0.1899014113156229E-3,
    ),
    (
        4,
        0.6512010228227200E+0,
        0.6112185773983089E-1,
        0.1898581257705106E-3,
    ),
    (
        4,
        0.6654758363948120E+0,
        0.6112185773983089E-1,
        0.1898804756095753E-3,
    ),
    (
        4,
        0.6778410414853370E+0,
        0.6112185773983089E-1,
        0.1899793610426402E-3,
    ),
    (
        4,
        0.6881760887484110E+0,
        0.6112185773983089E-1,
        0.1901464554844117E-3,
    ),
    (
        4,
        0.6963645267094598E+0,
        0.6112185773983089E-1,
        0.1903533246259542E-3,
    ),
    (
        4,
        0.7023010617153579E+0,
        0.6112185773983089E-1,
        0.1905556158463228E-3,
    ),
    (
        4,
        0.7059004636628753E+0,
        0.6112185773983089E-1,
        0.1907037155663528E-3,
    ),
    (
        5,
        0.3552470312472575E-1,
        0.6112185773983089E-1,
        0.5992997844249967E-4,
    ),
    (
        5,
        0.9151176620841283E-1,
        0.6112185773983089E-1,
        0.9749059382456978E-4,
    ),
    (
        5,
        0.1566197930068980E+0,
        0.6112185773983089E-1,
        0.1241680804599158E-3,
    ),
    (
        5,
        0.2265467599271907E+0,
        0.6112185773983089E-1,
        0.1437626154299360E-3,
    ),
    (
        5,
        0.2988242318581361E+0,
        0.6112185773983089E-1,
        0.1584200054793902E-3,
    ),
    (
        5,
        0.3717482419703886E+0,
        0.6112185773983089E-1,
        0.1694436550982744E-3,
    ),
    (
        5,
        0.4440094491758889E+0,
        0.6112185773983089E-1,
        0.1776617014018108E-3,
    ),
    (
        5,
        0.5145337096756642E+0,
        0.6112185773983089E-1,
        0.1836132434440077E-3,
    ),
    (
        5,
        0.5824053672860230E+0,
        0.6112185773983089E-1,
        0.1876494727075983E-3,
    ),
    (
        5,
        0.6468283961043370E+0,
        0.6112185773983089E-1,
        0.1899906535336482E-3,
    ),
    (
        6,
        0.6095964259104373E-1,
        0.1787828275342931E-1,
        0.8143252820767350E-4,
    ),
    (
        6,
        0.8811962270959388E-1,
        0.3953888740792096E-1,
        0.9998859890887728E-4,
    ),
    (
        6,
        0.1165936722428831E+0,
        0.6378121797722990E-1,
        0.1156199403068359E-3,
    ),
    (
        6,
        0.1460232857031785E+0,
        0.8985890813745037E-1,
        0.1287632092635513E-3,
    ),
    (
        6,
        0.1761197110181755E+0,
        0.1172606510576162E+0,
        0.1398378643365139E-3,
    ),
    (
        6,
        0.2066471190463718E+0,
        0.1456102876970995E+0,
        0.1491876468417391E-3,
    ),
    (
        6,
        0.2374076026328152E+0,
        0.1746153823011775E+0,
        0.1570855679175456E-3,
    ),
    (
        6,
        0.2682305474337051E+0,
        0.2040383070295584E+0,
        0.1637483948103775E-3,
    ),
    (
        6,
        0.2989653312142369E+0,
        0.2336788634003698E+0,
        0.1693500566632843E-3,
    ),
    (
        6,
        0.3294762752772209E+0,
        0.2633632752654219E+0,
        0.1740322769393633E-3,
    ),
    (
        6,
        0.3596390887276086E+0,
        0.2929369098051601E+0,
        0.1779126637278296E-3,
    ),
    (
        6,
        0.3893383046398812E+0,
        0.3222592785275512E+0,
        0.1810908108835412E-3,
    ),
    (
        6,
        0.4184653789358347E+0,
        0.3512004791195743E+0,
        0.1836529132600190E-3,
    ),
    (
        6,
        0.4469172319076166E+0,
        0.3796385677684537E+0,
        0.1856752841777379E-3,
    ),
    (
        6,
        0.4745950813276976E+0,
        0.4074575378263879E+0,
        0.1872270566606832E-3,
    ),
    (
        6,
        0.5014034601410262E+0,
        0.4345456906027828E+0,
        0.1883722645591307E-3,
    ),
    (
        6,
        0.5272493404551239E+0,
        0.4607942515205134E+0,
        0.1891714324525297E-3,
    ),
    (
        6,
        0.5520413051846366E+0,
        0.4860961284181720E+0,
        0.1896827480450146E-3,
    ),
    (
        6,
        0.5756887237503077E+0,
        0.5103447395342790E+0,
        0.1899628417059528E-3,
    ),
    (
        6,
        0.1225039430588352E+0,
        0.2136455922655793E-1,
        0.1123301829001669E-3,
    ),
    (
        6,
        0.1539113217321372E+0,
        0.4520926166137188E-1,
        0.1253698826711277E-3,
    ),
    (
        6,
        0.1856213098637712E+0,
        0.7086468177864818E-1,
        0.1366266117678531E-3,
    ),
    (
        6,
        0.2174998728035131E+0,
        0.9785239488772918E-1,
        0.1462736856106918E-3,
    ),
    (
        6,
        0.2494128336938330E+0,
        0.1258106396267210E+0,
        0.1545076466685412E-3,
    ),
    (
        6,
        0.2812321562143480E+0,
        0.1544529125047001E+0,
        0.1615096280814007E-3,
    ),
    (
        6,
        0.3128372276456111E+0,
        0.1835433512202753E+0,
        0.1674366639741759E-3,
    ),
    (
        6,
        0.3441145160177973E+0,
        0.2128813258619585E+0,
        0.1724225002437900E-3,
    ),
    (
        6,
        0.3749567714853510E+0,
        0.2422913734880829E+0,
        0.1765810822987288E-3,
    ),
    (
        6,
        0.4052621732015610E+0,
        0.2716163748391453E+0,
        0.1800104126010751E-3,
    ),
    (
        6,
        0.4349335453522385E+0,
        0.3007127671240280E+0,
        0.1827960437331284E-3,
    ),
    (
        6,
        0.4638776641524965E+0,
        0.3294470677216479E+0,
        0.1850140300716308E-3,
    ),
    (
        6,
        0.4920046410462687E+0,
        0.3576932543699155E+0,
        0.1867333507394938E-3,
    ),
    (
        6,
        0.5192273554861704E+0,
        0.3853307059757764E+0,
        0.1880178688638289E-3,
    ),
    (
        6,
        0.5454609081136522E+0,
        0.4122425044452694E+0,
        0.1889278925654758E-3,
    ),
    (
        6,
        0.5706220661424140E+0,
        0.4383139587781027E+0,
        0.1895213832507346E-3,
    ),
    (
        6,
        0.5946286755181518E+0,
        0.4634312536300553E+0,
        0.1898548277397420E-3,
    ),
    (
        6,
        0.1905370790924295E+0,
        0.2371311537781979E-1,
        0.1349105935937341E-3,
    ),
    (
        6,
        0.2242518717748009E+0,
        0.4917878059254806E-1,
        0.1444060068369326E-3,
    ),
    (
        6,
        0.2577190808025936E+0,
        0.7595498960495142E-1,
        0.1526797390930008E-3,
    ),
    (
        6,
        0.2908724534927187E+0,
        0.1036991083191100E+0,
        0.1598208771406474E-3,
    ),
    (
        6,
        0.3236354020056219E+0,
        0.1321348584450234E+0,
        0.1659354368615331E-3,
    ),
    (
        6,
        0.3559267359304543E+0,
        0.1610316571314789E+0,
        0.1711279910946440E-3,
    ),
    (
        6,
        0.3876637123676956E+0,
        0.1901912080395707E+0,
        0.1754952725601440E-3,
    ),
    (
        6,
        0.4187636705218842E+0,
        0.2194384950137950E+0,
        0.1791247850802529E-3,
    ),
    (
        6,
        0.4491449019883107E+0,
        0.2486155334763858E+0,
        0.1820954300877716E-3,
    ),
    (
        6,
        0.4787270932425445E+0,
        0.2775768931812335E+0,
        0.1844788524548449E-3,
    ),
    (
        6,
        0.5074315153055574E+0,
        0.3061863786591120E+0,
        0.1863409481706220E-3,
    ),
    (
        6,
        0.5351810507738336E+0,
        0.3343144718152556E+0,
        0.1877433008795068E-3,
    ),
    (
        6,
        0.5619001025975381E+0,
        0.3618362729028427E+0,
        0.1887444543705232E-3,
    ),
    (
        6,
        0.5875144035268046E+0,
        0.3886297583620408E+0,
        0.1894009829375006E-3,
    ),
    (
        6,
        0.6119507308734495E+0,
        0.4145742277792031E+0,
        0.1897683345035198E-3,
    ),
    (
        6,
        0.2619733870119463E+0,
        0.2540047186389353E-1,
        0.1517327037467653E-3,
    ),
    (
        6,
        0.2968149743237949E+0,
        0.5208107018543989E-1,
        0.1587740557483543E-3,
    ),
    (
        6,
        0.3310451504860488E+0,
        0.7971828470885599E-1,
        0.1649093382274097E-3,
    ),
    (
        6,
        0.3646215567376676E+0,
        0.1080465999177927E+0,
        0.1701915216193265E-3,
    ),
    (
        6,
        0.3974916785279360E+0,
        0.1368413849366629E+0,
        0.1746847753144065E-3,
    ),
    (
        6,
        0.4295967403772029E+0,
        0.1659073184763559E+0,
        0.1784555512007570E-3,
    ),
    (
        6,
        0.4608742854473447E+0,
        0.1950703730454614E+0,
        0.1815687562112174E-3,
    ),
    (
        6,
        0.4912598858949903E+0,
        0.2241721144376724E+0,
        0.1840864370663302E-3,
    ),
    (
        6,
        0.5206882758945558E+0,
        0.2530655255406489E+0,
        0.1860676785390006E-3,
    ),
    (
        6,
        0.5490940914019819E+0,
        0.2816118409731066E+0,
        0.1875690583743703E-3,
    ),
    (
        6,
        0.5764123302025542E+0,
        0.3096780504593238E+0,
        0.1886453236347225E-3,
    ),
    (
        6,
        0.6025786004213506E+0,
        0.3371348366394987E+0,
        0.1893501123329645E-3,
    ),
    (
        6,
        0.6275291964794956E+0,
        0.3638547827694396E+0,
        0.1897366184519868E-3,
    ),
    (
        6,
        0.3348189479861771E+0,
        0.2664841935537443E-1,
        0.1643908815152736E-3,
    ),
    (
        6,
        0.3699515545855295E+0,
        0.5424000066843495E-1,
        0.1696300350907768E-3,
    ),
    (
        6,
        0.4042003071474669E+0,
        0.8251992715430854E-1,
        0.1741553103844483E-3,
    ),
    (
        6,
        0.4375320100182624E+0,
        0.1112695182483710E+0,
        0.1780015282386092E-3,
    ),
    (
        6,
        0.4699054490335947E+0,
        0.1402964116467816E+0,
        0.1812116787077125E-3,
    ),
    (
        6,
        0.5012739879431952E+0,
        0.1694275117584291E+0,
        0.1838323158085421E-3,
    ),
    (
        6,
        0.5315874883754966E+0,
        0.1985038235312689E+0,
        0.1859113119837737E-3,
    ),
    (
        6,
        0.5607937109622117E+0,
        0.2273765660020893E+0,
        0.1874969220221698E-3,
    ),
    (
        6,
        0.5888393223495521E+0,
        0.2559041492849764E+0,
        0.1886375612681076E-3,
    ),
    (
        6,
        0.6156705979160163E+0,
        0.2839497251976899E+0,
        0.1893819575809276E-3,
    ),
    (
        6,
        0.6412338809078123E+0,
        0.3113791060500690E+0,
        0.1897794748256767E-3,
    ),
    (
        6,
        0.4076051259257167E+0,
        0.2757792290858463E-1,
        0.1738963926584846E-3,
    ),
    (
        6,
        0.4423788125791520E+0,
        0.5584136834984293E-1,
        0.1777442359873466E-3,
    ),
    (
        6,
        0.4760480917328258E+0,
        0.8457772087727143E-1,
        0.1810010815068719E-3,
    ),
    (
        6,
        0.5085838725946297E+0,
        0.1135975846359248E+0,
        0.1836920318248129E-3,
    ),
    (
        6,
        0.5399513637391218E+0,
        0.1427286904765053E+0,
        0.1858489473214328E-3,
    ),
    (
        6,
        0.5701118433636380E+0,
        0.1718112740057635E+0,
        0.1875079342496592E-3,
    ),
    (
        6,
        0.5990240530606021E+0,
        0.2006944855985351E+0,
        0.1887080239102310E-3,
    ),
    (
        6,
        0.6266452685139695E+0,
        0.2292335090598907E+0,
        0.1894905752176822E-3,
    ),
    (
        6,
        0.6529320971415942E+0,
        0.2572871512353714E+0,
        0.1898991061200695E-3,
    ),
    (
        6,
        0.4791583834610126E+0,
        0.2826094197735932E-1,
        0.1809065016458791E-3,
    ),
    (
        6,
        0.5130373952796940E+0,
        0.5699871359683649E-1,
        0.1836297121596799E-3,
    ),
    (
        6,
        0.5456252429628476E+0,
        0.8602712528554394E-1,
        0.1858426916241869E-3,
    ),
    (
        6,
        0.5768956329682385E+0,
        0.1151748137221281E+0,
        0.1875654101134641E-3,
    ),
    (
        6,
        0.6068186944699046E+0,
        0.1442811654136362E+0,
        0.1888240751833503E-3,
    ),
    (
        6,
        0.6353622248024907E+0,
        0.1731930321657680E+0,
        0.1896497383866979E-3,
    ),
    (
        6,
        0.6624927035731797E+0,
        0.2017619958756061E+0,
        0.1900775530219121E-3,
    ),
    (
        6,
        0.5484933508028488E+0,
        0.2874219755907391E-1,
        0.1858525041478814E-3,
    ),
    (
        6,
        0.5810207682142106E+0,
        0.5778312123713695E-1,
        0.1876248690077947E-3,
    ),
    (
        6,
        0.6120955197181352E+0,
        0.8695262371439526E-1,
        0.1889404439064607E-3,
    ),
    (
        6,
        0.6416944284294319E+0,
        0.1160893767057166E+0,
        0.1898168539265290E-3,
    ),
    (
        6,
        0.6697926391731260E+0,
        0.1450378826743251E+0,
        0.1902779940661772E-3,
    ),
    (
        6,
        0.6147594390585488E+0,
        0.2904957622341456E-1,
        0.1890125641731815E-3,
    ),
    (
        6,
        0.6455390026356783E+0,
        0.5823809152617197E-1,
        0.1899434637795751E-3,
    ),
    (
        6,
        0.6747258588365477E+0,
        0.8740384899884715E-1,
        0.1904520856831751E-3,
    ),
    (
        6,
        0.6772135750395347E+0,
        0.2919946135808105E-1,
        0.1905534498734563E-3,
    ),
];

fn lebedev_sphere_rule_5810() -> Result<LebedevSphereRule, KernelError> {
    let mut nodes = Vec::with_capacity(5810);
    for &(orbit_type, a, b, normalized_weight) in LEBEDEV_5810_ORBITS {
        if !normalized_weight.is_finite() || normalized_weight <= 0.0 {
            return Err(KernelError::InvalidSphereQuadratureWeights);
        }
        let weight = 4.0 * PI * normalized_weight;
        match orbit_type {
            1 => {
                push_node(&mut nodes, 1.0, 0.0, 0.0, weight);
                push_node(&mut nodes, -1.0, 0.0, 0.0, weight);
                push_node(&mut nodes, 0.0, 1.0, 0.0, weight);
                push_node(&mut nodes, 0.0, -1.0, 0.0, weight);
                push_node(&mut nodes, 0.0, 0.0, 1.0, weight);
                push_node(&mut nodes, 0.0, 0.0, -1.0, weight);
            }
            2 => {
                let coordinate = 1.0 / 2.0_f64.sqrt();
                for zero_axis in 0..3 {
                    let remaining = remaining_axes(zero_axis);
                    for mask in 0..4 {
                        let first = signed_coordinate(coordinate, mask & 1 != 0);
                        let second = signed_coordinate(coordinate, mask & 2 != 0);
                        let mut point = [0.0; 3];
                        point[remaining[0]] = first;
                        point[remaining[1]] = second;
                        push_node(&mut nodes, point[0], point[1], point[2], weight);
                    }
                }
            }
            3 => {
                let coordinate = 1.0 / 3.0_f64.sqrt();
                for mask in 0..8 {
                    push_node(
                        &mut nodes,
                        signed_coordinate(coordinate, mask & 1 != 0),
                        signed_coordinate(coordinate, mask & 2 != 0),
                        signed_coordinate(coordinate, mask & 4 != 0),
                        weight,
                    );
                }
            }
            4 => {
                let square = 1.0 - 2.0 * a * a;
                if !a.is_finite() || a <= 0.0 || square <= 0.0 {
                    return Err(KernelError::InvalidSphereQuadratureNode);
                }
                let special_coordinate = square.sqrt();
                for special_axis in 0..3 {
                    let mut base = [a; 3];
                    base[special_axis] = special_coordinate;
                    for mask in 0..8 {
                        push_node(
                            &mut nodes,
                            signed_coordinate(base[0], mask & 1 != 0),
                            signed_coordinate(base[1], mask & 2 != 0),
                            signed_coordinate(base[2], mask & 4 != 0),
                            weight,
                        );
                    }
                }
            }
            5 => {
                let square = 1.0 - a * a;
                if !a.is_finite() || a <= 0.0 || square <= 0.0 {
                    return Err(KernelError::InvalidSphereQuadratureNode);
                }
                let other_coordinate = square.sqrt();
                for zero_axis in 0..3 {
                    let remaining = remaining_axes(zero_axis);
                    for (first_abs, second_abs) in [(a, other_coordinate), (other_coordinate, a)] {
                        for mask in 0..4 {
                            let mut point = [0.0; 3];
                            point[remaining[0]] = signed_coordinate(first_abs, mask & 1 != 0);
                            point[remaining[1]] = signed_coordinate(second_abs, mask & 2 != 0);
                            push_node(&mut nodes, point[0], point[1], point[2], weight);
                        }
                    }
                }
            }
            6 => {
                let square = 1.0 - a * a - b * b;
                if !a.is_finite() || !b.is_finite() || a <= 0.0 || b <= 0.0 || square <= 0.0 {
                    return Err(KernelError::InvalidSphereQuadratureNode);
                }
                let third_coordinate = square.sqrt();
                let permutations = [
                    [a, b, third_coordinate],
                    [a, third_coordinate, b],
                    [b, a, third_coordinate],
                    [b, third_coordinate, a],
                    [third_coordinate, a, b],
                    [third_coordinate, b, a],
                ];
                for base in permutations {
                    for mask in 0..8 {
                        push_node(
                            &mut nodes,
                            signed_coordinate(base[0], mask & 1 != 0),
                            signed_coordinate(base[1], mask & 2 != 0),
                            signed_coordinate(base[2], mask & 4 != 0),
                            weight,
                        );
                    }
                }
            }
            _ => return Err(KernelError::InvalidSphereQuadratureNode),
        }
    }

    validate_nodes(&nodes, 5810)?;
    let weight_sum = compensated_sum(nodes.iter().map(|node| node.weight));
    if !weight_sum.is_finite() || (weight_sum - 4.0 * PI).abs() > 2.0e-12 {
        return Err(KernelError::InvalidSphereQuadratureWeights);
    }

    Ok(LebedevSphereRule {
        rule: LebedevRule::Points5810,
        nodes,
        weight_sum,
    })
}

fn push_node(nodes: &mut Vec<SphereQuadratureNode>, x: f64, y: f64, z: f64, weight: f64) {
    nodes.push(SphereQuadratureNode { x, y, z, weight });
}

fn signed_coordinate(value: f64, negative: bool) -> f64 {
    if negative { -value } else { value }
}

fn remaining_axes(zero_axis: usize) -> [usize; 2] {
    match zero_axis {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    }
}

fn validate_nodes(
    nodes: &[SphereQuadratureNode],
    expected_count: usize,
) -> Result<(), KernelError> {
    if nodes.len() != expected_count {
        return Err(KernelError::InvalidSphereQuadratureNodeCount);
    }
    for node in nodes {
        if !node.x.is_finite()
            || !node.y.is_finite()
            || !node.z.is_finite()
            || !node.weight.is_finite()
        {
            return Err(KernelError::NonFiniteValue);
        }
        if node.weight <= 0.0 {
            return Err(KernelError::InvalidSphereQuadratureWeights);
        }
        let norm_squared = node.norm_squared();
        if !norm_squared.is_finite() || (norm_squared - 1.0).abs() > 64.0 * f64::EPSILON {
            return Err(KernelError::InvalidSphereQuadratureNode);
        }
    }
    Ok(())
}

/// Configuration for Lebedev integrand-contribution diagnostics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SphereQuadratureDiagnosticsConfig {
    /// Relative threshold against the largest unweighted integrand value.
    ///
    /// NASA's Pc3D_Hall diagnostic uses 1e-3. A node is significant only when
    /// its sample is strictly greater than threshold times the maximum sample.
    pub relative_contribution_threshold: f64,
    /// Minimum significant-node count required by the caller's adequacy flag.
    ///
    /// Zero records diagnostics without requiring a minimum.
    pub minimum_significant_nodes: usize,
}

impl Default for SphereQuadratureDiagnosticsConfig {
    fn default() -> Self {
        Self {
            relative_contribution_threshold: 1.0e-3,
            minimum_significant_nodes: 0,
        }
    }
}

/// Result of integrating a nonnegative sphere integrand with contribution diagnostics.
///
/// These diagnostics do not form a formal quadrature error bound. The adequacy
/// flag only records the caller's minimum-significant-node criterion.
#[derive(Debug, Clone, PartialEq)]
pub struct SphereQuadratureReport {
    /// Rule used for the integration.
    pub rule: LebedevRule,
    /// Number of quadrature nodes.
    pub node_count: usize,
    /// Lebedev approximation to the solid-angle integral.
    pub integral: f64,
    /// Sum of the rule's weights; should equal 4π.
    pub weight_sum: f64,
    /// Largest unweighted integrand sample.
    pub maximum_integrand: f64,
    /// Relative threshold used for the significance diagnostic.
    pub relative_contribution_threshold: f64,
    /// Number of nodes with integrand strictly above threshold times maximum.
    pub significant_node_count: usize,
    /// Fraction of total solid-angle weight at significant nodes.
    pub significant_weight_fraction: f64,
    /// Fraction of estimated integral contributed by significant nodes.
    pub significant_integral_fraction: f64,
    /// Largest weighted node contribution divided by the integral.
    pub maximum_node_integral_fraction: f64,
    /// Whether the configured minimum significant-node count was met.
    pub adequacy_satisfied: bool,
}

/// Integrate a nonnegative integrand sample vector over the unit sphere.
///
/// The caller supplies one finite nonnegative sample per rule node in exactly
/// the deterministic node order returned by lebedev_sphere_rule. This function
/// reproduces NASA's relative significant-node diagnostic and adds weighted
/// contribution fractions. It does not estimate unobserved quadrature error or
/// prove convergence between rules.
pub fn integrate_sphere_samples(
    rule: &LebedevSphereRule,
    integrand_values: &[f64],
    config: SphereQuadratureDiagnosticsConfig,
) -> Result<SphereQuadratureReport, KernelError> {
    validate_nodes(&rule.nodes, rule.rule.node_count())?;
    let actual_weight_sum = compensated_sum(rule.nodes.iter().map(|node| node.weight));
    if !actual_weight_sum.is_finite()
        || (actual_weight_sum - 4.0 * PI).abs() > 64.0 * f64::EPSILON * 4.0 * PI
        || (actual_weight_sum - rule.weight_sum).abs() > 64.0 * f64::EPSILON * 4.0 * PI
    {
        return Err(KernelError::InvalidSphereQuadratureWeights);
    }
    if integrand_values.len() != rule.nodes.len() {
        return Err(KernelError::InvalidSphereQuadratureNodeCount);
    }
    if !config.relative_contribution_threshold.is_finite()
        || !(0.0..=1.0).contains(&config.relative_contribution_threshold)
    {
        return Err(KernelError::InvalidSphereQuadratureThreshold);
    }
    if integrand_values.iter().any(|value| !value.is_finite()) {
        return Err(KernelError::NonFiniteValue);
    }
    if integrand_values.iter().any(|value| *value < 0.0) {
        return Err(KernelError::InvalidSphereIntegrand);
    }

    let maximum_integrand = integrand_values.iter().copied().fold(0.0_f64, f64::max);
    if maximum_integrand <= 0.0 {
        return Err(KernelError::ZeroSphereIntegrand);
    }

    let weighted_contributions = rule
        .nodes
        .iter()
        .zip(integrand_values)
        .map(|(node, value)| node.weight * value)
        .collect::<Vec<_>>();
    if weighted_contributions
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(KernelError::NumericalOverflow);
    }

    let integral = compensated_sum(weighted_contributions.iter().copied());
    if !integral.is_finite() || integral <= 0.0 {
        return Err(KernelError::NumericalOverflow);
    }

    let threshold = config.relative_contribution_threshold * maximum_integrand;
    let significant_indices = integrand_values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| (*value > threshold).then_some(index))
        .collect::<Vec<_>>();
    let significant_weight = compensated_sum(
        significant_indices
            .iter()
            .map(|index| rule.nodes[*index].weight),
    );
    let significant_integral = compensated_sum(
        significant_indices
            .iter()
            .map(|index| weighted_contributions[*index]),
    );
    let maximum_node_contribution = weighted_contributions
        .iter()
        .copied()
        .fold(0.0_f64, f64::max);

    Ok(SphereQuadratureReport {
        rule: rule.rule,
        node_count: rule.nodes.len(),
        integral,
        weight_sum: rule.weight_sum,
        maximum_integrand,
        relative_contribution_threshold: config.relative_contribution_threshold,
        significant_node_count: significant_indices.len(),
        significant_weight_fraction: significant_weight / rule.weight_sum,
        significant_integral_fraction: significant_integral / integral,
        maximum_node_integral_fraction: maximum_node_contribution / integral,
        adequacy_satisfied: significant_indices.len() >= config.minimum_significant_nodes,
    })
}

/// Tolerances and contribution requirements for the low-order refinement ladder.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptiveSphereQuadratureConfig {
    /// Absolute difference tolerance between successive rule estimates.
    pub absolute_tolerance: f64,
    /// Relative difference tolerance, scaled by the finer estimate.
    pub relative_tolerance: f64,
    /// Per-rule material-contribution diagnostic configuration.
    pub diagnostics: SphereQuadratureDiagnosticsConfig,
}

/// Result of comparing successive nested low-order Lebedev estimates.
///
/// The reported difference is an empirical refinement indicator, not a
/// mathematically rigorous upper bound on quadrature error.
#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveSphereQuadratureReport {
    /// Final and finest rule actually evaluated.
    pub final_report: SphereQuadratureReport,
    /// Reports from the 6-, 14- and, when needed, 26-point evaluations.
    pub evaluations: Vec<SphereQuadratureReport>,
    /// Absolute difference between the final two rule estimates.
    pub successive_absolute_difference: f64,
    /// Relative difference, scaled by the absolute finer estimate.
    pub successive_relative_difference: f64,
    /// Whether the final successive difference met the requested tolerance.
    pub numerical_convergence_achieved: bool,
    /// Whether the final rule met its significant-node requirement.
    pub adequacy_satisfied: bool,
    /// True only when both numerical convergence and contribution adequacy passed.
    pub qualified: bool,
}

/// Integrate one deterministic nonnegative function over nested 6/14/26-point rules.
///
/// The same function is evaluated at each rule's nodes. The 6-point estimate is
/// compared with the 14-point estimate; if the requested tolerance or the
/// significant-node criterion is not met, the 26-point rule is evaluated and
/// compared with the 14-point estimate.
///
/// The function must be deterministic for a fixed node. The API cannot prevent
/// callers from using interior mutability, so the caller remains responsible
/// for honoring this requirement.
///
/// The qualified flag is only a low-order numerical-refinement and node-support
/// diagnostic. It is not a rigorous quadrature error bound, a complete Lebedev
/// convergence study, or a physical-model qualification.
pub fn integrate_sphere_adaptive<F>(
    integrand: F,
    config: AdaptiveSphereQuadratureConfig,
) -> Result<AdaptiveSphereQuadratureReport, KernelError>
where
    F: Fn(f64, f64, f64) -> f64,
{
    if !config.absolute_tolerance.is_finite()
        || !config.relative_tolerance.is_finite()
        || config.absolute_tolerance < 0.0
        || config.relative_tolerance < 0.0
        || (config.absolute_tolerance == 0.0 && config.relative_tolerance == 0.0)
    {
        return Err(KernelError::InvalidSphereQuadratureTolerance);
    }

    let mut evaluations = Vec::with_capacity(3);
    let coarse = evaluate_sphere_rule(LebedevRule::Points6, &integrand, config.diagnostics)?;
    let medium = evaluate_sphere_rule(LebedevRule::Points14, &integrand, config.diagnostics)?;
    let (first_absolute, first_relative) =
        estimate_refinement_difference(coarse.integral, medium.integral)?;
    let first_converged = refinement_tolerance_met(
        first_absolute,
        medium.integral,
        config.absolute_tolerance,
        config.relative_tolerance,
    );
    let medium_adequate = medium.adequacy_satisfied;

    evaluations.push(coarse);
    evaluations.push(medium.clone());

    if first_converged && medium_adequate {
        return Ok(build_adaptive_report(
            medium,
            evaluations,
            first_absolute,
            first_relative,
            true,
        ));
    }

    let fine = evaluate_sphere_rule(LebedevRule::Points26, &integrand, config.diagnostics)?;
    let (fine_absolute, fine_relative) =
        estimate_refinement_difference(medium.integral, fine.integral)?;
    let fine_converged = refinement_tolerance_met(
        fine_absolute,
        fine.integral,
        config.absolute_tolerance,
        config.relative_tolerance,
    );
    evaluations.push(fine.clone());

    Ok(build_adaptive_report(
        fine,
        evaluations,
        fine_absolute,
        fine_relative,
        fine_converged,
    ))
}

fn evaluate_sphere_rule<F>(
    selected: LebedevRule,
    integrand: &F,
    diagnostics: SphereQuadratureDiagnosticsConfig,
) -> Result<SphereQuadratureReport, KernelError>
where
    F: Fn(f64, f64, f64) -> f64,
{
    let rule = lebedev_sphere_rule(selected)?;
    let values = rule
        .nodes()
        .iter()
        .map(|node| integrand(node.x, node.y, node.z))
        .collect::<Vec<_>>();
    integrate_sphere_samples(&rule, &values, diagnostics)
}

fn estimate_refinement_difference(coarse: f64, fine: f64) -> Result<(f64, f64), KernelError> {
    let absolute = (fine - coarse).abs();
    if !absolute.is_finite() || !fine.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    let relative = absolute / fine.abs().max(f64::MIN_POSITIVE);
    if !relative.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    Ok((absolute, relative))
}

fn refinement_tolerance_met(
    absolute_difference: f64,
    fine_estimate: f64,
    absolute_tolerance: f64,
    relative_tolerance: f64,
) -> bool {
    absolute_difference <= absolute_tolerance + relative_tolerance * fine_estimate.abs()
}

fn build_adaptive_report(
    final_report: SphereQuadratureReport,
    evaluations: Vec<SphereQuadratureReport>,
    successive_absolute_difference: f64,
    successive_relative_difference: f64,
    numerical_convergence_achieved: bool,
) -> AdaptiveSphereQuadratureReport {
    let adequacy_satisfied = final_report.adequacy_satisfied;
    AdaptiveSphereQuadratureReport {
        final_report,
        evaluations,
        successive_absolute_difference,
        successive_relative_difference,
        numerical_convergence_achieved,
        adequacy_satisfied,
        qualified: numerical_convergence_achieved && adequacy_satisfied,
    }
}

fn compensated_sum(values: impl IntoIterator<Item = f64>) -> f64 {
    // Neumaier compensated summation for positive terms with varied magnitudes.
    let mut sum = 0.0;
    let mut correction = 0.0;
    for value in values {
        let next = sum + value;
        if sum.abs() >= value.abs() {
            correction += (sum - next) + value;
        } else {
            correction += (value - next) + sum;
        }
        sum = next;
    }
    sum + correction
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(selected: LebedevRule) -> LebedevSphereRule {
        lebedev_sphere_rule(selected).expect("valid built-in rule")
    }

    fn double_factorial(value: usize) -> f64 {
        if value <= 1 {
            return 1.0;
        }
        (1..=value)
            .step_by(2)
            .fold(1.0, |product, term| product * term as f64)
    }

    fn exact_sphere_monomial_integral(a: usize, b: usize, c: usize) -> f64 {
        if a % 2 == 1 || b % 2 == 1 || c % 2 == 1 {
            return 0.0;
        }
        let numerator = double_factorial(a.saturating_sub(1))
            * double_factorial(b.saturating_sub(1))
            * double_factorial(c.saturating_sub(1));
        let denominator = double_factorial(a + b + c + 1);
        4.0 * PI * numerator / denominator
    }

    fn integrate_monomial(rule: &LebedevSphereRule, a: usize, b: usize, c: usize) -> f64 {
        compensated_sum(rule.nodes.iter().map(|node| {
            node.weight * node.x.powi(a as i32) * node.y.powi(b as i32) * node.z.powi(c as i32)
        }))
    }

    #[test]
    fn nasa_5810_rule_expands_to_valid_nodes_and_four_pi_weight_sum() {
        let rule = lebedev_sphere_rule(LebedevRule::Points5810).expect("NASA 5,810-point rule");
        assert_eq!(rule.nodes.len(), 5810);
        assert_eq!(rule.rule, LebedevRule::Points5810);
        assert_eq!(rule.rule.algebraic_precision(), 131);
        assert!((rule.weight_sum - 4.0 * PI).abs() < 2.0e-12);
        assert!(
            rule.nodes
                .iter()
                .all(|node| { node.weight > 0.0 && (node.norm_squared() - 1.0).abs() < 1.0e-13 })
        );
    }

    #[test]
    fn nasa_5810_rule_matches_selected_high_degree_sphere_moments() {
        let rule = lebedev_sphere_rule(LebedevRule::Points5810).expect("NASA 5,810-point rule");
        for (a, b, c) in [
            (130, 0, 0),
            (64, 64, 0),
            (42, 44, 44),
            (131, 0, 0),
            (65, 64, 0),
        ] {
            let actual = integrate_monomial(&rule, a, b, c);
            let expected = exact_sphere_monomial_integral(a, b, c);
            if expected == 0.0 {
                assert!(actual.abs() < 2.0e-12);
            } else {
                assert!(
                    (actual - expected).abs() / expected.abs() < 2.0e-10,
                    "moment ({a},{b},{c}): actual={actual:e}, expected={expected:e}"
                );
            }
        }
    }

    #[test]
    fn all_rules_have_unit_nodes_and_weights_sum_to_four_pi() {
        for selected in [
            LebedevRule::Points6,
            LebedevRule::Points14,
            LebedevRule::Points26,
        ] {
            let rule = rule(selected);
            assert_eq!(rule.nodes.len(), selected.node_count());
            assert!((rule.weight_sum - 4.0 * PI).abs() < 1.0e-13);
            assert!(
                rule.nodes.iter().all(|node| {
                    (node.norm_squared() - 1.0).abs() < 1.0e-14 && node.weight > 0.0
                })
            );
        }
    }

    #[test]
    fn six_point_rule_is_exact_through_degree_three() {
        let rule = rule(LebedevRule::Points6);
        for total in 0..=3 {
            for a in 0..=total {
                for b in 0..=(total - a) {
                    let c = total - a - b;
                    let actual = integrate_monomial(&rule, a, b, c);
                    let expected = exact_sphere_monomial_integral(a, b, c);
                    assert!((actual - expected).abs() < 1.0e-12);
                }
            }
        }
    }

    #[test]
    fn fourteen_point_rule_is_exact_through_degree_five() {
        let rule = rule(LebedevRule::Points14);
        for total in 0..=5 {
            for a in 0..=total {
                for b in 0..=(total - a) {
                    let c = total - a - b;
                    let actual = integrate_monomial(&rule, a, b, c);
                    let expected = exact_sphere_monomial_integral(a, b, c);
                    assert!((actual - expected).abs() < 1.0e-12);
                }
            }
        }
    }

    #[test]
    fn twenty_six_point_rule_is_exact_through_degree_seven() {
        let rule = rule(LebedevRule::Points26);
        for total in 0..=7 {
            for a in 0..=total {
                for b in 0..=(total - a) {
                    let c = total - a - b;
                    let actual = integrate_monomial(&rule, a, b, c);
                    let expected = exact_sphere_monomial_integral(a, b, c);
                    assert!((actual - expected).abs() < 1.0e-12);
                }
            }
        }
    }

    #[test]
    fn constant_integrand_integrates_to_four_pi() {
        for selected in [
            LebedevRule::Points6,
            LebedevRule::Points14,
            LebedevRule::Points26,
        ] {
            let rule = rule(selected);
            let report = integrate_sphere_samples(
                &rule,
                &vec![1.0; rule.nodes.len()],
                SphereQuadratureDiagnosticsConfig::default(),
            )
            .expect("constant integrand");
            assert!((report.integral - 4.0 * PI).abs() < 1.0e-12);
            assert_eq!(report.significant_node_count, rule.nodes.len());
            assert!((report.significant_integral_fraction - 1.0).abs() < 1.0e-14);
        }
    }

    #[test]
    fn nasa_relative_contribution_threshold_is_strict_and_named() {
        let rule = rule(LebedevRule::Points6);
        let mut values = vec![1.0e-3; rule.nodes.len()];
        values[0] = 1.0;
        values[1] = 1.000_000_1e-3;
        let report = integrate_sphere_samples(
            &rule,
            &values,
            SphereQuadratureDiagnosticsConfig {
                relative_contribution_threshold: 1.0e-3,
                minimum_significant_nodes: 0,
            },
        )
        .expect("valid diagnostic");
        assert_eq!(report.significant_node_count, 2);
        assert!((report.significant_weight_fraction - 2.0 / 6.0).abs() < 1.0e-14);
    }

    #[test]
    fn concentrated_integrand_is_marked_as_low_support() {
        let rule = rule(LebedevRule::Points26);
        let mut values = vec![1.0e-8; rule.nodes.len()];
        values[3] = 1.0;
        let report = integrate_sphere_samples(
            &rule,
            &values,
            SphereQuadratureDiagnosticsConfig {
                relative_contribution_threshold: 1.0e-3,
                minimum_significant_nodes: 3,
            },
        )
        .expect("valid concentrated diagnostic");
        assert_eq!(report.significant_node_count, 1);
        assert!(!report.adequacy_satisfied);
        assert!(report.maximum_node_integral_fraction > 0.1);
    }

    #[test]
    fn invalid_sample_shapes_values_and_thresholds_fail_closed() {
        let rule = rule(LebedevRule::Points6);
        assert_eq!(
            integrate_sphere_samples(&rule, &[1.0], SphereQuadratureDiagnosticsConfig::default()),
            Err(KernelError::InvalidSphereQuadratureNodeCount)
        );

        let mut negative = vec![1.0; rule.nodes.len()];
        negative[2] = -1.0;
        assert_eq!(
            integrate_sphere_samples(
                &rule,
                &negative,
                SphereQuadratureDiagnosticsConfig::default()
            ),
            Err(KernelError::InvalidSphereIntegrand)
        );

        let mut non_finite = vec![1.0; rule.nodes.len()];
        non_finite[2] = f64::NAN;
        assert_eq!(
            integrate_sphere_samples(
                &rule,
                &non_finite,
                SphereQuadratureDiagnosticsConfig::default()
            ),
            Err(KernelError::NonFiniteValue)
        );

        assert_eq!(
            integrate_sphere_samples(
                &rule,
                &vec![1.0; rule.nodes.len()],
                SphereQuadratureDiagnosticsConfig {
                    relative_contribution_threshold: f64::NAN,
                    minimum_significant_nodes: 0,
                }
            ),
            Err(KernelError::InvalidSphereQuadratureThreshold)
        );
    }

    #[test]
    fn adaptive_refinement_stops_at_fourteen_for_a_low_degree_integrand() {
        let result = integrate_sphere_adaptive(
            |x, _y, _z| 1.0 + 0.2 * x * x,
            AdaptiveSphereQuadratureConfig {
                absolute_tolerance: 1.0e-12,
                relative_tolerance: 1.0e-12,
                diagnostics: SphereQuadratureDiagnosticsConfig::default(),
            },
        )
        .expect("smooth low-degree integrand");
        assert_eq!(result.evaluations.len(), 2);
        assert_eq!(result.final_report.rule, LebedevRule::Points14);
        assert!(result.numerical_convergence_achieved);
        assert!(result.adequacy_satisfied);
        assert!(result.qualified);
    }

    #[test]
    fn adaptive_refinement_uses_twenty_six_points_when_six_and_fourteen_disagree() {
        let result = integrate_sphere_adaptive(
            |x, y, z| (4.0 * x + 2.0 * y - z).exp(),
            AdaptiveSphereQuadratureConfig {
                absolute_tolerance: 1.0e-14,
                relative_tolerance: 1.0e-14,
                diagnostics: SphereQuadratureDiagnosticsConfig::default(),
            },
        )
        .expect("positive smooth integrand");
        assert_eq!(result.evaluations.len(), 3);
        assert_eq!(result.final_report.rule, LebedevRule::Points26);
        assert!(result.successive_absolute_difference.is_finite());
    }

    #[test]
    fn adaptive_qualification_separates_node_support_from_estimate_stability() {
        let result = integrate_sphere_adaptive(
            |x, y, z| {
                if x > 0.999 && y.abs() < 1.0e-12 && z.abs() < 1.0e-12 {
                    1.0
                } else {
                    1.0e-8
                }
            },
            AdaptiveSphereQuadratureConfig {
                absolute_tolerance: 1.0,
                relative_tolerance: 1.0,
                diagnostics: SphereQuadratureDiagnosticsConfig {
                    relative_contribution_threshold: 1.0e-3,
                    minimum_significant_nodes: 3,
                },
            },
        )
        .expect("low-support integrand");
        assert!(!result.adequacy_satisfied);
        assert!(!result.qualified);
    }

    #[test]
    fn adaptive_refinement_rejects_invalid_tolerances() {
        assert_eq!(
            integrate_sphere_adaptive(
                |_, _, _| 1.0,
                AdaptiveSphereQuadratureConfig {
                    absolute_tolerance: 0.0,
                    relative_tolerance: 0.0,
                    diagnostics: SphereQuadratureDiagnosticsConfig::default(),
                },
            ),
            Err(KernelError::InvalidSphereQuadratureTolerance)
        );
    }

    #[test]
    fn zero_integrand_is_not_given_a_fake_significance_peak() {
        let rule = rule(LebedevRule::Points6);
        assert_eq!(
            integrate_sphere_samples(
                &rule,
                &vec![0.0; rule.nodes.len()],
                SphereQuadratureDiagnosticsConfig::default()
            ),
            Err(KernelError::ZeroSphereIntegrand)
        );
    }

    #[test]
    fn sphere_rule_order_is_deterministic_and_nested() {
        let six = rule(LebedevRule::Points6);
        let fourteen = rule(LebedevRule::Points14);
        let twenty_six = rule(LebedevRule::Points26);

        let six_points = six
            .nodes
            .iter()
            .map(|node| (node.x, node.y, node.z))
            .collect::<Vec<_>>();
        let fourteen_points = fourteen
            .nodes
            .iter()
            .map(|node| (node.x, node.y, node.z))
            .collect::<Vec<_>>();
        let twenty_six_points = twenty_six
            .nodes
            .iter()
            .map(|node| (node.x, node.y, node.z))
            .collect::<Vec<_>>();
        assert_eq!(six_points, fourteen_points[..6]);
        assert_eq!(fourteen_points, twenty_six_points[..14]);
    }
}
