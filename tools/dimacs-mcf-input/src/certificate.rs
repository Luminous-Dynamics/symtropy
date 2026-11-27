#![forbid(unsafe_code)]

//! Independent primal/dual verification for single-commodity DIMACS MCF.
//!
//! A valid certificate contains an original-space flow for every logical arc,
//! node potentials, the exact objective and the canonical instance digest.
//! The verifier checks primal bounds/conservation and nonnegative reduced cost
//! on every available forward/reverse residual arc. It does not call a solver.

use crate::McfInstance;
use sha2::{Digest, Sha256};
use std::fmt;

pub const MCF_OPTIMALITY_CERTIFICATE_SCHEMA: &str =
    "dimacs-mcf-primal-dual-optimality-certificate-v0.1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McfArcFlow {
    /// Must match the original one-based DIMACS arc ordinal.
    pub ordinal: u32,
    pub flow: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McfOptimalityCertificate {
    pub schema_version: String,
    /// SHA-256 over the deterministic canonical DIMACS instance bytes.
    pub graph_digest: String,
    /// Original-space flow witness in original arc order.
    pub arc_flows: Vec<McfArcFlow>,
    /// One-based node potentials with zero sentinel at index zero.
    pub node_potentials: Vec<i128>,
    /// Exact original objective, including all arc lower-bound costs.
    pub objective_cost: i128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McfCertificateError {
    pub message: String,
}

impl McfCertificateError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for McfCertificateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "DIMACS MCF certificate: {}", self.message)
    }
}

impl std::error::Error for McfCertificateError {}

fn validate_instance(instance: &McfInstance) -> Result<(), McfCertificateError> {
    let expected_supply_len = usize::try_from(instance.node_count)
        .ok()
        .and_then(|count| count.checked_add(1))
        .ok_or_else(|| McfCertificateError::new("node supply vector length overflow"))?;
    if instance.node_supplies.len() != expected_supply_len
        || instance.node_supplies.first().copied() != Some(0)
    {
        return Err(McfCertificateError::new(
            "node supply vector shape or zero sentinel is invalid",
        ));
    }
    if instance.arc_count as usize != instance.arcs.len() {
        return Err(McfCertificateError::new(
            "arc count does not match the number of arc records",
        ));
    }
    for (index, arc) in instance.arcs.iter().enumerate() {
        let ordinal = u32::try_from(index + 1)
            .map_err(|_| McfCertificateError::new("arc ordinal exceeds u32"))?;
        if arc.ordinal != ordinal {
            return Err(McfCertificateError::new(format!(
                "arc at position {} has ordinal {}, expected {ordinal}",
                index + 1,
                arc.ordinal
            )));
        }
        if arc.tail == 0
            || arc.head == 0
            || arc.tail > instance.node_count
            || arc.head > instance.node_count
        {
            return Err(McfCertificateError::new(format!(
                "arc {ordinal} endpoint is outside the declared node range"
            )));
        }
        if arc.lower_bound < 0 || arc.upper_bound < 0 || arc.upper_bound < arc.lower_bound {
            return Err(McfCertificateError::new(format!(
                "arc {ordinal} has invalid lower/upper bounds"
            )));
        }
    }
    Ok(())
}

/// Compute the SHA-256 binding for the canonical semantic DIMACS instance.
///
/// Source line numbers and comments are intentionally excluded; node supplies
/// are normalized by vertex ID while arc order and parallel-arc identities
/// are preserved by the canonical serializer.
pub fn canonical_instance_sha256(instance: &McfInstance) -> Result<String, McfCertificateError> {
    validate_instance(instance)?;
    let canonical = instance.to_dimacs();
    let digest = Sha256::digest(canonical.as_bytes());
    let mut output = String::with_capacity(7 + digest.len() * 2);
    output.push_str("sha256:");
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}")
            .map_err(|_| McfCertificateError::new("cannot format SHA-256 digest"))?;
    }
    Ok(output)
}

fn checked_reduced_cost(
    cost: i128,
    tail_potential: i128,
    head_potential: i128,
    arc_ordinal: u32,
    direction: &str,
) -> Result<i128, McfCertificateError> {
    cost.checked_add(tail_potential)
        .and_then(|value| value.checked_sub(head_potential))
        .ok_or_else(|| {
            McfCertificateError::new(format!(
                "reduced-cost arithmetic overflow on arc {arc_ordinal} {direction}"
            ))
        })
}

/// Verify primal feasibility and a dual potential certificate for minimum cost.
///
/// Sign convention: positive node supply means net outflow. The reduced cost
/// of a residual arc u -> v is c(u,v) + potential[u] - potential[v]. Every
/// residual forward arc (flow < upper) and reverse arc (flow > lower) must
/// have nonnegative reduced cost. Together with primal feasibility this rules
/// out negative-cost residual cycles and proves global MCF optimality for this
/// single-commodity linear network model.
///
/// All certificate flows use original DIMACS arc ordinals and original bounds,
/// not the transformed residual arc space. The graph digest binds the proof
/// to the canonical input, including parallel-arc order and exact cost/bounds.
pub fn verify_mcf_optimality(
    instance: &McfInstance,
    certificate: &McfOptimalityCertificate,
) -> Result<(), McfCertificateError> {
    validate_instance(instance)?;

    if certificate.schema_version != MCF_OPTIMALITY_CERTIFICATE_SCHEMA {
        return Err(McfCertificateError::new("unsupported certificate schema"));
    }
    let expected_digest = canonical_instance_sha256(instance)?;
    if certificate.graph_digest != expected_digest {
        return Err(McfCertificateError::new(
            "certificate graph digest does not match the canonical DIMACS instance",
        ));
    }
    if certificate.arc_flows.len() != instance.arcs.len() {
        return Err(McfCertificateError::new(
            "flow witness does not cover every logical DIMACS arc",
        ));
    }

    let potential_len = usize::try_from(instance.node_count)
        .ok()
        .and_then(|count| count.checked_add(1))
        .ok_or_else(|| McfCertificateError::new("potential vector length overflow"))?;
    if certificate.node_potentials.len() != potential_len
        || certificate.node_potentials.first().copied() != Some(0)
    {
        return Err(McfCertificateError::new(
            "node-potential vector shape or zero sentinel is invalid",
        ));
    }

    let mut balances = Vec::new();
    balances.try_reserve_exact(potential_len).map_err(|error| {
        McfCertificateError::new(format!("cannot reserve primal balance vector: {error}"))
    })?;
    balances.resize(potential_len, 0_i128);

    let mut objective = 0_i128;
    for (index, (arc, flow_record)) in instance.arcs.iter().zip(&certificate.arc_flows).enumerate()
    {
        if flow_record.ordinal != arc.ordinal {
            return Err(McfCertificateError::new(format!(
                "flow witness ordinal {} at position {} does not match input arc {}",
                flow_record.ordinal,
                index + 1,
                arc.ordinal
            )));
        }
        let flow = flow_record.flow;
        if flow < arc.lower_bound || flow > arc.upper_bound {
            return Err(McfCertificateError::new(format!(
                "flow {} on arc {} is outside [{}, {}]",
                flow, arc.ordinal, arc.lower_bound, arc.upper_bound
            )));
        }

        let amount = i128::from(flow);
        balances[arc.tail as usize] =
            balances[arc.tail as usize]
                .checked_add(amount)
                .ok_or_else(|| {
                    McfCertificateError::new(format!(
                        "outflow balance overflow at node {}",
                        arc.tail
                    ))
                })?;
        balances[arc.head as usize] =
            balances[arc.head as usize]
                .checked_sub(amount)
                .ok_or_else(|| {
                    McfCertificateError::new(format!(
                        "inflow balance overflow at node {}",
                        arc.head
                    ))
                })?;

        let arc_cost = amount
            .checked_mul(i128::from(arc.unit_cost))
            .ok_or_else(|| {
                McfCertificateError::new(format!(
                    "objective multiplication overflow on arc {}",
                    arc.ordinal
                ))
            })?;
        objective = objective
            .checked_add(arc_cost)
            .ok_or_else(|| McfCertificateError::new("objective accumulation overflowed i128"))?;

        let tail_potential = certificate.node_potentials[arc.tail as usize];
        let head_potential = certificate.node_potentials[arc.head as usize];
        if flow < arc.upper_bound {
            let reduced = checked_reduced_cost(
                i128::from(arc.unit_cost),
                tail_potential,
                head_potential,
                arc.ordinal,
                "forward",
            )?;
            if reduced < 0 {
                return Err(McfCertificateError::new(format!(
                    "negative forward reduced cost {reduced} on arc {}",
                    arc.ordinal
                )));
            }
        }
        if flow > arc.lower_bound {
            let reverse_cost = -i128::from(arc.unit_cost);
            let reduced = checked_reduced_cost(
                reverse_cost,
                head_potential,
                tail_potential,
                arc.ordinal,
                "reverse",
            )?;
            if reduced < 0 {
                return Err(McfCertificateError::new(format!(
                    "negative reverse reduced cost {reduced} on arc {}",
                    arc.ordinal
                )));
            }
        }
    }

    for (node, (&observed, &declared)) in balances
        .iter()
        .zip(&instance.node_supplies)
        .enumerate()
        .skip(1)
    {
        if observed != i128::from(declared) {
            return Err(McfCertificateError::new(format!(
                "primal conservation mismatch at node {node}: observed {observed}, declared {declared}"
            )));
        }
    }
    if objective != certificate.objective_cost {
        return Err(McfCertificateError::new(format!(
            "objective mismatch: recomputed {objective}, certificate declares {}",
            certificate.objective_cost
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_dimacs_mcf;

    fn certificate(
        instance: &McfInstance,
        flows: &[i64],
        potentials: &[i128],
        objective: i128,
    ) -> McfOptimalityCertificate {
        McfOptimalityCertificate {
            schema_version: MCF_OPTIMALITY_CERTIFICATE_SCHEMA.to_string(),
            graph_digest: canonical_instance_sha256(instance).unwrap(),
            arc_flows: instance
                .arcs
                .iter()
                .zip(flows)
                .map(|(arc, &flow)| McfArcFlow {
                    ordinal: arc.ordinal,
                    flow,
                })
                .collect(),
            node_potentials: potentials.to_vec(),
            objective_cost: objective,
        }
    }

    #[test]
    fn verifies_a_single_arc_primal_dual_optimality_certificate() {
        let instance = parse_dimacs_mcf("p min 2 1\nn 1 3\nn 2 -3\na 1 2 0 5 7").unwrap();
        let cert = certificate(&instance, &[3], &[0, 0, 7], 21);
        verify_mcf_optimality(&instance, &cert).unwrap();
    }

    #[test]
    fn verifies_parallel_arc_identity_and_optimality() {
        let instance =
            parse_dimacs_mcf("p min 2 2\nn 1 3\nn 2 -3\na 1 2 0 3 1\na 1 2 0 3 3").unwrap();
        let cert = certificate(&instance, &[3, 0], &[0, 0, 1], 3);
        verify_mcf_optimality(&instance, &cert).unwrap();
    }

    #[test]
    fn rejects_a_feasible_but_nonoptimal_parallel_arc_flow() {
        let instance =
            parse_dimacs_mcf("p min 2 2\nn 1 3\nn 2 -3\na 1 2 0 3 1\na 1 2 0 3 3").unwrap();
        let cert = certificate(&instance, &[0, 3], &[0, 0, 1], 9);
        assert!(
            verify_mcf_optimality(&instance, &cert)
                .unwrap_err()
                .to_string()
                .contains("negative reverse reduced cost")
        );
    }

    #[test]
    fn rejects_negative_forward_reduced_cost_on_available_arc() {
        let instance = parse_dimacs_mcf("p min 2 1\na 1 2 0 5 2").unwrap();
        // flow == lower bound: only the forward residual arc is available.
        // pi[1] = 0 and pi[2] = 3 make c + pi[1] - pi[2] = -1.
        let cert = certificate(&instance, &[0], &[0, 0, 3], 0);
        assert!(
            verify_mcf_optimality(&instance, &cert)
                .unwrap_err()
                .to_string()
                .contains("negative forward reduced cost")
        );
    }

    #[test]
    fn verifies_negative_cost_at_upper_capacity() {
        let instance = parse_dimacs_mcf("p min 2 1\nn 1 5\nn 2 -5\na 1 2 0 5 -2").unwrap();
        let cert = certificate(&instance, &[5], &[0, 0, 0], -10);
        verify_mcf_optimality(&instance, &cert).unwrap();
    }

    #[test]
    fn verifies_mandatory_lower_bound_and_objective_offset_in_original_space() {
        let instance = parse_dimacs_mcf("p min 2 1\nn 1 2\nn 2 -2\na 1 2 2 5 -3").unwrap();
        let cert = certificate(&instance, &[2], &[0, 3, 0], -6);
        verify_mcf_optimality(&instance, &cert).unwrap();
    }

    #[test]
    fn accepts_a_negative_cost_arc_at_its_lower_bound_when_dual_allows_it() {
        let instance = parse_dimacs_mcf("p min 2 1\na 1 2 0 5 -2").unwrap();
        let cert = certificate(&instance, &[0], &[0, 2, 0], 0);
        verify_mcf_optimality(&instance, &cert).unwrap();
    }

    #[test]
    fn rejects_wrong_graph_digest_and_certificate_schema() {
        let instance = parse_dimacs_mcf("p min 2 1\nn 1 1\nn 2 -1\na 1 2 0 2 4").unwrap();
        let mut cert = certificate(&instance, &[1], &[0, 0, 4], 4);
        cert.graph_digest = "sha256:00".to_string();
        assert!(
            verify_mcf_optimality(&instance, &cert)
                .unwrap_err()
                .to_string()
                .contains("graph digest")
        );

        let mut cert = certificate(&instance, &[1], &[0, 0, 4], 4);
        cert.schema_version = "unknown".to_string();
        assert!(
            verify_mcf_optimality(&instance, &cert)
                .unwrap_err()
                .to_string()
                .contains("schema")
        );
    }

    #[test]
    fn rejects_wrong_objective_and_primal_conservation() {
        let instance = parse_dimacs_mcf("p min 2 1\nn 1 3\nn 2 -3\na 1 2 0 5 7").unwrap();
        let wrong_cost = certificate(&instance, &[3], &[0, 0, 7], 22);
        assert!(
            verify_mcf_optimality(&instance, &wrong_cost)
                .unwrap_err()
                .to_string()
                .contains("objective mismatch")
        );
        let wrong_balance = certificate(&instance, &[2], &[0, 0, 7], 14);
        assert!(
            verify_mcf_optimality(&instance, &wrong_balance)
                .unwrap_err()
                .to_string()
                .contains("conservation mismatch")
        );
    }

    #[test]
    fn rejects_wrong_arc_ordinal_and_potential_vector_shape() {
        let instance = parse_dimacs_mcf("p min 2 1\nn 1 1\nn 2 -1\na 1 2 0 2 4").unwrap();
        let mut wrong_arc = certificate(&instance, &[1], &[0, 0, 4], 4);
        wrong_arc.arc_flows[0].ordinal = 2;
        assert!(
            verify_mcf_optimality(&instance, &wrong_arc)
                .unwrap_err()
                .to_string()
                .contains("ordinal")
        );
        let wrong_potentials = certificate(&instance, &[1], &[0, 4], 4);
        assert!(
            verify_mcf_optimality(&instance, &wrong_potentials)
                .unwrap_err()
                .to_string()
                .contains("potential vector")
        );
    }
}
