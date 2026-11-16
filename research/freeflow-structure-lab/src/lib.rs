//! Bounded geometry/topology scaffold for free-flow modular truss research.
//!
//! This crate deliberately does not claim structural strength. It generates
//! a deterministic 3D graph and checks geometry/topology invariants only.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }
    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }
    pub fn normalized(self) -> Option<Self> {
        let length = self.norm();
        if !length.is_finite() || length <= f64::MIN_POSITIVE {
            return None;
        }
        Some(self.scale(1.0 / length))
    }
    pub fn scale(self, factor: f64) -> Self {
        Self::new(self.x * factor, self.y * factor, self.z * factor)
    }
    pub fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
    pub fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
    pub fn distance(self, other: Self) -> f64 {
        self.sub(other).norm()
    }
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HubFamily {
    UniversalFourPortV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterfaceFamily {
    TaperLockV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemberRole {
    BranchStrut,
    CrossBrace,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: u32,
    pub position: Vec3,
    pub level: u8,
    pub hub_family: HubFamily,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub id: u32,
    pub from: u32,
    pub to: u32,
    pub role: MemberRole,
    pub interface_family: InterfaceFamily,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub depth: u8,
    pub root_member_length: f64,
    pub branch_angle_rad: f64,
    pub twist_rad: f64,
    pub length_ratio: f64,
    pub max_nodes: usize,
    pub min_member_length: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            depth: 4,
            root_member_length: 1.0,
            branch_angle_rad: 0.42,
            twist_rad: 0.35,
            length_ratio: 0.62,
            max_nodes: 1_024,
            min_member_length: 1.0e-5,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum GenerationError {
    InvalidDepth,
    InvalidLength,
    InvalidAngle,
    InvalidTwist,
    InvalidLengthRatio,
    InvalidMemberMinimum,
    NodeBudgetExceeded { required: usize, maximum: usize },
    ArithmeticOverflow,
    NonFiniteGeometry,
    DegenerateMember { member_id: u32, length: f64 },
    DuplicateMember { from: u32, to: u32 },
    Disconnected { components: usize },
    InvalidGraph(&'static str),
}

impl fmt::Display for GenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDepth => write!(f, "depth must be in 1..=10"),
            Self::InvalidLength => write!(f, "root_member_length must be finite and positive"),
            Self::InvalidAngle => write!(f, "branch_angle_rad must be finite and in (0.05, 1.2)"),
            Self::InvalidTwist => write!(f, "twist_rad must be finite and in [-3.14, 3.14]"),
            Self::InvalidLengthRatio => write!(f, "length_ratio must be finite and in (0, 1)"),
            Self::InvalidMemberMinimum => {
                write!(f, "min_member_length must be finite and positive")
            }
            Self::NodeBudgetExceeded { required, maximum } => write!(
                f,
                "node budget exceeded: required {required}, maximum {maximum}"
            ),
            Self::ArithmeticOverflow => write!(f, "geometry size arithmetic overflow"),
            Self::NonFiniteGeometry => write!(f, "generated geometry contains non-finite values"),
            Self::DegenerateMember { member_id, length } => {
                write!(f, "member {member_id} has invalid length {length}")
            }
            Self::DuplicateMember { from, to } => write!(
                f,
                "duplicate undirected member between nodes {from} and {to}"
            ),
            Self::Disconnected { components } => {
                write!(f, "geometry graph is disconnected: {components} components")
            }
            Self::InvalidGraph(message) => write!(f, "invalid graph: {message}"),
        }
    }
}
impl Error for GenerationError {}

#[derive(Clone, Debug, PartialEq)]
pub struct TopologyMetrics {
    pub node_count: usize,
    pub member_count: usize,
    pub connected_components: usize,
    pub cycle_rank: usize,
    pub maximum_node_degree: usize,
    pub total_member_length: f64,
    pub minimum_member_length: f64,
    pub maximum_member_length: f64,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    pub same_build_digest_non_security: u64,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructureGraph {
    pub config: Config,
    pub nodes: Vec<Node>,
    pub members: Vec<Member>,
}

#[derive(Clone, Copy)]
struct Branch {
    start_node: u32,
    direction: Vec3,
    frame_u: Vec3,
    frame_v: Vec3,
    length: f64,
    level: u8,
}

impl Config {
    pub fn expected_node_count(&self) -> Result<usize, GenerationError> {
        if !(1..=10).contains(&self.depth) {
            return Err(GenerationError::InvalidDepth);
        }
        let leaves = 1usize
            .checked_shl(u32::from(self.depth))
            .ok_or(GenerationError::ArithmeticOverflow)?;
        let expanded = leaves
            .checked_sub(1)
            .ok_or(GenerationError::ArithmeticOverflow)?;
        expanded
            .checked_mul(2)
            .and_then(|count| count.checked_add(1))
            .ok_or(GenerationError::ArithmeticOverflow)
    }

    pub fn validate(&self) -> Result<(), GenerationError> {
        if !self.root_member_length.is_finite() || self.root_member_length <= 0.0 {
            return Err(GenerationError::InvalidLength);
        }
        if !self.branch_angle_rad.is_finite() || !(0.05..=1.2).contains(&self.branch_angle_rad) {
            return Err(GenerationError::InvalidAngle);
        }
        if !self.twist_rad.is_finite() || self.twist_rad.abs() > 3.14 {
            return Err(GenerationError::InvalidTwist);
        }
        if !self.length_ratio.is_finite() || !(0.0..1.0).contains(&self.length_ratio) {
            return Err(GenerationError::InvalidLengthRatio);
        }
        if !self.min_member_length.is_finite() || self.min_member_length <= 0.0 {
            return Err(GenerationError::InvalidMemberMinimum);
        }
        let required = self.expected_node_count()?;
        if required > self.max_nodes {
            return Err(GenerationError::NodeBudgetExceeded {
                required,
                maximum: self.max_nodes,
            });
        }
        Ok(())
    }
}

impl StructureGraph {
    pub fn generate(config: Config) -> Result<Self, GenerationError> {
        config.validate()?;

        let mut nodes = Vec::with_capacity(config.expected_node_count()?);
        let mut members = Vec::new();
        let root = Node {
            id: 0,
            position: Vec3::new(0.0, 0.0, 0.0),
            level: 0,
            hub_family: HubFamily::UniversalFourPortV1,
        };
        nodes.push(root);

        let mut branches = vec![Branch {
            start_node: 0,
            direction: Vec3::new(0.0, 0.0, 1.0),
            frame_u: Vec3::new(1.0, 0.0, 0.0),
            frame_v: Vec3::new(0.0, 1.0, 0.0),
            length: config.root_member_length,
            level: 0,
        }];

        let sin_angle = config.branch_angle_rad.sin();
        let cos_angle = config.branch_angle_rad.cos();
        let cos_twist = config.twist_rad.cos();
        let sin_twist = config.twist_rad.sin();

        for _level in 0..config.depth {
            let mut next_branches = Vec::with_capacity(branches.len().saturating_mul(2));
            for branch in branches {
                let offset = branch
                    .frame_u
                    .scale(cos_twist)
                    .add(branch.frame_v.scale(sin_twist))
                    .scale(sin_angle);
                let left_direction = branch
                    .direction
                    .scale(cos_angle)
                    .add(offset)
                    .normalized()
                    .ok_or(GenerationError::NonFiniteGeometry)?;
                let right_direction = branch
                    .direction
                    .scale(cos_angle)
                    .sub(offset)
                    .normalized()
                    .ok_or(GenerationError::NonFiniteGeometry)?;

                let child_length = branch.length * config.length_ratio;
                let parent_position = nodes
                    .get(branch.start_node as usize)
                    .ok_or(GenerationError::InvalidGraph(
                        "branch references unknown parent",
                    ))?
                    .position;
                let left_position = parent_position.add(left_direction.scale(child_length));
                let right_position = parent_position.add(right_direction.scale(child_length));

                if !left_position.is_finite() || !right_position.is_finite() {
                    return Err(GenerationError::NonFiniteGeometry);
                }

                let left_id =
                    u32::try_from(nodes.len()).map_err(|_| GenerationError::ArithmeticOverflow)?;
                nodes.push(Node {
                    id: left_id,
                    position: left_position,
                    level: branch.level + 1,
                    hub_family: HubFamily::UniversalFourPortV1,
                });

                let right_id =
                    u32::try_from(nodes.len()).map_err(|_| GenerationError::ArithmeticOverflow)?;
                nodes.push(Node {
                    id: right_id,
                    position: right_position,
                    level: branch.level + 1,
                    hub_family: HubFamily::UniversalFourPortV1,
                });

                push_member(
                    &mut members,
                    branch.start_node,
                    left_id,
                    MemberRole::BranchStrut,
                )?;
                push_member(
                    &mut members,
                    branch.start_node,
                    right_id,
                    MemberRole::BranchStrut,
                )?;
                push_member(&mut members, left_id, right_id, MemberRole::CrossBrace)?;

                let left_frame_u =
                    perpendicular_frame(left_direction, branch.frame_u, branch.frame_v)?;
                let right_frame_u =
                    perpendicular_frame(right_direction, branch.frame_u, branch.frame_v)?;
                let left_frame_v = left_direction
                    .cross(left_frame_u)
                    .normalized()
                    .ok_or(GenerationError::NonFiniteGeometry)?;
                let right_frame_v = right_direction
                    .cross(right_frame_u)
                    .normalized()
                    .ok_or(GenerationError::NonFiniteGeometry)?;

                next_branches.push(Branch {
                    start_node: left_id,
                    direction: left_direction,
                    frame_u: left_frame_u,
                    frame_v: left_frame_v,
                    length: child_length,
                    level: branch.level + 1,
                });
                next_branches.push(Branch {
                    start_node: right_id,
                    direction: right_direction,
                    frame_u: right_frame_u,
                    frame_v: right_frame_v,
                    length: child_length,
                    level: branch.level + 1,
                });
            }
            branches = next_branches;
        }

        let graph = Self {
            config,
            nodes,
            members,
        };
        graph.audit()?;
        Ok(graph)
    }

    pub fn audit(&self) -> Result<TopologyMetrics, GenerationError> {
        self.config.validate()?;
        if self.nodes.len() != self.config.expected_node_count()? {
            return Err(GenerationError::InvalidGraph(
                "node count does not match grammar",
            ));
        }
        if self.nodes.is_empty() {
            return Err(GenerationError::InvalidGraph("graph has no nodes"));
        }

        let mut min = Vec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = Vec3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (index, node) in self.nodes.iter().enumerate() {
            if node.id as usize != index || !node.position.is_finite() {
                return Err(GenerationError::NonFiniteGeometry);
            }
            min.x = min.x.min(node.position.x);
            min.y = min.y.min(node.position.y);
            min.z = min.z.min(node.position.z);
            max.x = max.x.max(node.position.x);
            max.y = max.y.max(node.position.y);
            max.z = max.z.max(node.position.z);
        }

        let mut adjacency = vec![Vec::<usize>::new(); self.nodes.len()];
        let mut seen = HashSet::<(u32, u32)>::with_capacity(self.members.len());
        let mut total_length = 0.0;
        let mut min_length = f64::INFINITY;
        let mut max_length: f64 = 0.0;

        for (index, member) in self.members.iter().enumerate() {
            if member.id as usize != index {
                return Err(GenerationError::InvalidGraph(
                    "member IDs are not contiguous",
                ));
            }
            if member.from as usize >= self.nodes.len()
                || member.to as usize >= self.nodes.len()
                || member.from == member.to
            {
                return Err(GenerationError::InvalidGraph("member endpoint is invalid"));
            }
            let key = if member.from < member.to {
                (member.from, member.to)
            } else {
                (member.to, member.from)
            };
            if !seen.insert(key) {
                return Err(GenerationError::DuplicateMember {
                    from: key.0,
                    to: key.1,
                });
            }

            let a = self.nodes[member.from as usize].position;
            let b = self.nodes[member.to as usize].position;
            let length = a.distance(b);
            if !length.is_finite() || length < self.config.min_member_length {
                return Err(GenerationError::DegenerateMember {
                    member_id: member.id,
                    length,
                });
            }
            total_length += length;
            min_length = min_length.min(length);
            max_length = max_length.max(length);
            adjacency[member.from as usize].push(member.to as usize);
            adjacency[member.to as usize].push(member.from as usize);
        }

        if !total_length.is_finite() {
            return Err(GenerationError::NonFiniteGeometry);
        }

        let mut visited = vec![false; self.nodes.len()];
        let mut components = 0usize;
        let mut maximum_degree = 0usize;
        for adjacent in &adjacency {
            maximum_degree = maximum_degree.max(adjacent.len());
        }
        for start in 0..self.nodes.len() {
            if visited[start] {
                continue;
            }
            components += 1;
            let mut stack = vec![start];
            visited[start] = true;
            while let Some(current) = stack.pop() {
                for &neighbor in &adjacency[current] {
                    if !visited[neighbor] {
                        visited[neighbor] = true;
                        stack.push(neighbor);
                    }
                }
            }
        }
        if components != 1 {
            return Err(GenerationError::Disconnected { components });
        }

        let cycle_rank = self
            .members
            .len()
            .checked_sub(self.nodes.len())
            .and_then(|value| value.checked_add(1))
            .ok_or(GenerationError::InvalidGraph(
                "connected graph has invalid edge/node counts",
            ))?;

        Ok(TopologyMetrics {
            node_count: self.nodes.len(),
            member_count: self.members.len(),
            connected_components: components,
            cycle_rank,
            maximum_node_degree: maximum_degree,
            total_member_length: total_length,
            minimum_member_length: min_length,
            maximum_member_length: max_length,
            bounds_min: min,
            bounds_max: max,
            same_build_digest_non_security: self.digest_non_security(),
            claim_ceiling: "geometry/topology only; no structural strength, fractal-dimension, safety, or flight qualification claim",
        })
    }

    /// FNV-1a regression digest only. It is not cryptographic and must never
    /// authenticate a design, receipt, component or engineering decision.
    pub fn digest_non_security(&self) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        fn mix(hash: &mut u64, bytes: &[u8]) {
            for byte in bytes {
                *hash ^= u64::from(*byte);
                *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        mix(&mut hash, &[self.config.depth]);
        for value in [
            self.config.root_member_length,
            self.config.branch_angle_rad,
            self.config.twist_rad,
            self.config.length_ratio,
            self.config.min_member_length,
        ] {
            mix(&mut hash, &value.to_bits().to_le_bytes());
        }
        mix(&mut hash, &(self.config.max_nodes as u64).to_le_bytes());
        for node in &self.nodes {
            mix(&mut hash, &node.id.to_le_bytes());
            mix(&mut hash, &[node.level]);
            mix(&mut hash, &node.position.x.to_bits().to_le_bytes());
            mix(&mut hash, &node.position.y.to_bits().to_le_bytes());
            mix(&mut hash, &node.position.z.to_bits().to_le_bytes());
        }
        for member in &self.members {
            mix(&mut hash, &member.id.to_le_bytes());
            mix(&mut hash, &member.from.to_le_bytes());
            mix(&mut hash, &member.to.to_le_bytes());
            mix(
                &mut hash,
                &[match member.role {
                    MemberRole::BranchStrut => 0,
                    MemberRole::CrossBrace => 1,
                }],
            );
        }
        hash
    }
}

fn perpendicular_frame(
    direction: Vec3,
    preferred_u: Vec3,
    fallback_v: Vec3,
) -> Result<Vec3, GenerationError> {
    let projected = preferred_u.sub(direction.scale(preferred_u.dot(direction)));
    if let Some(unit) = projected.normalized() {
        return Ok(unit);
    }
    let fallback = fallback_v.sub(direction.scale(fallback_v.dot(direction)));
    fallback
        .normalized()
        .ok_or(GenerationError::NonFiniteGeometry)
}

fn push_member(
    members: &mut Vec<Member>,
    from: u32,
    to: u32,
    role: MemberRole,
) -> Result<(), GenerationError> {
    let id = u32::try_from(members.len()).map_err(|_| GenerationError::ArithmeticOverflow)?;
    members.push(Member {
        id,
        from,
        to,
        role,
        interface_family: InterfaceFamily::TaperLockV1,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_four_grammar_has_expected_connected_graph_and_cycle_rank() {
        let graph = StructureGraph::generate(Config::default()).unwrap();
        let metrics = graph.audit().unwrap();
        assert_eq!(metrics.node_count, 31);
        assert_eq!(metrics.member_count, 45);
        assert_eq!(metrics.connected_components, 1);
        assert_eq!(metrics.cycle_rank, 15);
        assert!(metrics.maximum_node_degree >= 3);
        assert!(metrics.total_member_length > 0.0);
    }

    #[test]
    fn same_config_produces_identical_geometry_digest() {
        let left = StructureGraph::generate(Config::default()).unwrap();
        let right = StructureGraph::generate(Config::default()).unwrap();
        assert_eq!(left.digest_non_security(), right.digest_non_security());
        assert_eq!(left.nodes, right.nodes);
        assert_eq!(left.members, right.members);
    }

    #[test]
    fn all_node_coordinates_and_member_lengths_are_finite() {
        let graph = StructureGraph::generate(Config::default()).unwrap();
        let metrics = graph.audit().unwrap();
        assert!(graph.nodes.iter().all(|node| node.position.is_finite()));
        assert!(metrics.minimum_member_length.is_finite());
        assert!(metrics.maximum_member_length.is_finite());
        assert!(metrics.bounds_min.is_finite());
        assert!(metrics.bounds_max.is_finite());
    }

    #[test]
    fn invalid_depth_angle_ratio_and_non_finite_values_are_rejected() {
        let mut config = Config::default();
        config.depth = 0;
        assert_eq!(config.validate(), Err(GenerationError::InvalidDepth));

        let mut config = Config::default();
        config.branch_angle_rad = f64::NAN;
        assert_eq!(config.validate(), Err(GenerationError::InvalidAngle));

        let mut config = Config::default();
        config.length_ratio = 1.0;
        assert_eq!(config.validate(), Err(GenerationError::InvalidLengthRatio));

        let mut config = Config::default();
        config.root_member_length = f64::INFINITY;
        assert_eq!(config.validate(), Err(GenerationError::InvalidLength));
    }

    #[test]
    fn node_budget_is_enforced_before_geometry_allocation() {
        let mut config = Config::default();
        config.depth = 8;
        config.max_nodes = 100;
        assert!(matches!(
            config.validate(),
            Err(GenerationError::NodeBudgetExceeded {
                required: 511,
                maximum: 100
            })
        ));
    }

    #[test]
    fn too_short_cross_brace_is_rejected_by_minimum_member_contract() {
        let mut config = Config::default();
        config.branch_angle_rad = 0.051;
        config.min_member_length = 0.1;
        assert!(matches!(
            StructureGraph::generate(config),
            Err(GenerationError::DegenerateMember { .. })
        ));
    }
}
