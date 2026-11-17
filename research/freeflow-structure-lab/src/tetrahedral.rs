//! Face-attached tetrahedral bay grammar.
//!
//! Each child bay is a regular tetrahedral pin-jointed cell that shares an
//! entire non-collinear triangular face with an already assembled bay. This
//! avoids the one-point attachment hinge mechanism of a simple branching
//! tree in the ideal geometric model. It is not a strength or flight proof.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use crate::Vec3;

#[derive(Clone, Debug, PartialEq)]
pub struct TetrahedralConfig {
    /// Number of recursive attachment generations after the root cell.
    pub depth: u8,
    pub root_edge_length_m: f64,
    pub max_nodes: usize,
    pub min_member_length_m: f64,
    pub min_face_area_m2: f64,
    pub max_ports_per_node: usize,
    pub min_port_separation_rad: f64,
}

impl Default for TetrahedralConfig {
    fn default() -> Self {
        Self {
            depth: 4,
            root_edge_length_m: 1.0,
            max_nodes: 1_024,
            min_member_length_m: 1.0e-5,
            min_face_area_m2: 1.0e-8,
            max_ports_per_node: 16,
            min_port_separation_rad: 0.05,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TetrahedralError {
    InvalidDepth,
    InvalidEdgeLength,
    InvalidMemberMinimum,
    InvalidFaceAreaMinimum,
    InvalidPortBudget,
    InvalidPortSeparation,
    NodeBudgetExceeded {
        required: usize,
        maximum: usize,
    },
    ArithmeticOverflow,
    NonFiniteGeometry,
    DegenerateFace {
        cell_id: u32,
        area_bits: u64,
    },
    DegenerateCell {
        cell_id: u32,
    },
    InvalidAttachment {
        cell_id: u32,
    },
    DuplicateMember {
        a: u32,
        b: u32,
    },
    InvalidMember {
        member_id: u32,
    },
    Disconnected {
        components: usize,
    },
    PortBudgetExceeded {
        node_id: u32,
        required: usize,
        maximum: usize,
    },
    PortsTooClose {
        node_id: u32,
        angle_rad: f64,
        minimum_rad: f64,
    },
    InvalidGraph(&'static str),
}

impl fmt::Display for TetrahedralError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDepth => write!(f, "tetrahedral depth must be in 1..=8"),
            Self::InvalidEdgeLength => write!(f, "root edge length must be finite and positive"),
            Self::InvalidMemberMinimum => {
                write!(f, "minimum member length must be finite and positive")
            }
            Self::InvalidFaceAreaMinimum => {
                write!(f, "minimum face area must be finite and positive")
            }
            Self::InvalidPortBudget => write!(f, "maximum ports per node must be at least 3"),
            Self::InvalidPortSeparation => {
                write!(f, "minimum port separation must be finite and in [0, pi]")
            }
            Self::NodeBudgetExceeded { required, maximum } => write!(
                f,
                "node budget exceeded: required {required}, maximum {maximum}"
            ),
            Self::ArithmeticOverflow => write!(f, "tetrahedral count arithmetic overflow"),
            Self::NonFiniteGeometry => write!(f, "tetrahedral geometry contains non-finite values"),
            Self::DegenerateFace { cell_id, area_bits } => write!(
                f,
                "cell {cell_id} has a degenerate attachment face (area bits {area_bits})"
            ),
            Self::DegenerateCell { cell_id } => {
                write!(f, "cell {cell_id} has zero or invalid volume")
            }
            Self::InvalidAttachment { cell_id } => write!(
                f,
                "cell {cell_id} does not attach through its parent's triangular face"
            ),
            Self::DuplicateMember { a, b } => {
                write!(f, "duplicate member between nodes {a} and {b}")
            }
            Self::InvalidMember { member_id } => {
                write!(f, "member {member_id} has invalid endpoints or length")
            }
            Self::Disconnected { components } => {
                write!(f, "assembly is disconnected: {components} components")
            }
            Self::PortBudgetExceeded {
                node_id,
                required,
                maximum,
            } => write!(
                f,
                "node {node_id} requires {required} ports, maximum is {maximum}"
            ),
            Self::PortsTooClose {
                node_id,
                angle_rad,
                minimum_rad,
            } => write!(
                f,
                "node {node_id} has port angle {angle_rad} below minimum {minimum_rad}"
            ),
            Self::InvalidGraph(message) => write!(f, "invalid tetrahedral graph: {message}"),
        }
    }
}
impl Error for TetrahedralError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TetraAttachmentInterface {
    TriangularThreePointV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TetraMemberInterface {
    IndexedPinEndV1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TetraNode {
    pub id: u32,
    pub position: Vec3,
    pub generation: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TetraMember {
    pub id: u32,
    pub from: u32,
    pub to: u32,
    pub interface: TetraMemberInterface,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TetraCell {
    pub id: u32,
    pub vertices: [u32; 4],
    pub generation: u8,
    pub parent_cell: Option<u32>,
    pub attachment_face: Option<[u32; 3]>,
    pub attachment_interface: Option<TetraAttachmentInterface>,
}

#[derive(Clone, Copy)]
struct FrontierFace {
    vertices: [u32; 3],
    opposite: u32,
    parent_cell: u32,
    generation: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TetrahedralMetrics {
    pub node_count: usize,
    pub member_count: usize,
    pub cell_count: usize,
    pub connected_components: usize,
    pub cycle_rank: usize,
    pub maximum_node_degree: usize,
    pub total_member_length_m: f64,
    pub minimum_member_length_m: f64,
    pub maximum_member_length_m: f64,
    pub minimum_attachment_face_area_m2: f64,
    pub minimum_cell_volume_m3: f64,
    pub blocked_attachment_faces: usize,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    pub same_build_digest_non_security: u64,
    pub ideal_attachment_contract: &'static str,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
/// Adaptive face-attached assembly. Faces blocked by connector-port limits
/// are counted and left unexpanded rather than exceeding the interface budget.
pub struct TetrahedralAssembly {
    pub config: TetrahedralConfig,
    pub nodes: Vec<TetraNode>,
    pub members: Vec<TetraMember>,
    pub cells: Vec<TetraCell>,
    pub blocked_attachment_faces: usize,
}

impl TetrahedralConfig {
    pub fn expected_new_cell_count(&self) -> Result<usize, TetrahedralError> {
        if !(1..=8).contains(&self.depth) {
            return Err(TetrahedralError::InvalidDepth);
        }
        1usize
            .checked_shl(u32::from(self.depth) + 1)
            .and_then(|count| count.checked_sub(2))
            .ok_or(TetrahedralError::ArithmeticOverflow)
    }

    pub fn expected_cell_count(&self) -> Result<usize, TetrahedralError> {
        self.expected_new_cell_count()?
            .checked_add(1)
            .ok_or(TetrahedralError::ArithmeticOverflow)
    }

    pub fn expected_node_count(&self) -> Result<usize, TetrahedralError> {
        self.expected_new_cell_count()?
            .checked_add(4)
            .ok_or(TetrahedralError::ArithmeticOverflow)
    }

    pub fn expected_member_count(&self) -> Result<usize, TetrahedralError> {
        self.expected_new_cell_count()?
            .checked_mul(3)
            .and_then(|count| count.checked_add(6))
            .ok_or(TetrahedralError::ArithmeticOverflow)
    }

    pub fn validate(&self) -> Result<(), TetrahedralError> {
        if !self.root_edge_length_m.is_finite() || self.root_edge_length_m <= 0.0 {
            return Err(TetrahedralError::InvalidEdgeLength);
        }
        if !self.min_member_length_m.is_finite() || self.min_member_length_m <= 0.0 {
            return Err(TetrahedralError::InvalidMemberMinimum);
        }
        if !self.min_face_area_m2.is_finite() || self.min_face_area_m2 <= 0.0 {
            return Err(TetrahedralError::InvalidFaceAreaMinimum);
        }
        if self.max_ports_per_node < 3 {
            return Err(TetrahedralError::InvalidPortBudget);
        }
        if !self.min_port_separation_rad.is_finite()
            || !(0.0..=std::f64::consts::PI).contains(&self.min_port_separation_rad)
        {
            return Err(TetrahedralError::InvalidPortSeparation);
        }
        let required = self.expected_node_count()?;
        if required > self.max_nodes {
            return Err(TetrahedralError::NodeBudgetExceeded {
                required,
                maximum: self.max_nodes,
            });
        }
        Ok(())
    }
}

impl TetrahedralAssembly {
    pub fn generate(config: TetrahedralConfig) -> Result<Self, TetrahedralError> {
        config.validate()?;

        let edge = config.root_edge_length_m;
        let root_height = (2.0_f64 / 3.0).sqrt() * edge;
        let root_base_y = 3.0_f64.sqrt() * edge / 6.0;
        let root_top_y = 3.0_f64.sqrt() * edge / 3.0;

        let root_nodes = [
            Vec3::new(-edge * 0.5, -root_base_y, 0.0),
            Vec3::new(edge * 0.5, -root_base_y, 0.0),
            Vec3::new(0.0, root_top_y, 0.0),
            Vec3::new(0.0, 0.0, root_height),
        ];

        let mut nodes = Vec::with_capacity(config.expected_node_count()?);
        for (id, position) in root_nodes.into_iter().enumerate() {
            nodes.push(TetraNode {
                id: u32::try_from(id).map_err(|_| TetrahedralError::ArithmeticOverflow)?,
                position,
                generation: 0,
            });
        }

        let root_cell = TetraCell {
            id: 0,
            vertices: [0, 1, 2, 3],
            generation: 0,
            parent_cell: None,
            attachment_face: None,
            attachment_interface: None,
        };
        let mut cells = vec![root_cell];
        let mut members = Vec::with_capacity(config.expected_member_count()?);
        let mut seen_edges = HashSet::with_capacity(config.expected_member_count()?);
        let mut node_degrees = vec![3usize; 4];
        let mut blocked_attachment_faces = 0usize;

        for (a, b) in [(0, 1), (1, 2), (2, 0), (0, 3), (1, 3), (2, 3)] {
            push_member(&mut members, &mut seen_edges, a, b)?;
        }

        let mut frontier = vec![
            FrontierFace {
                vertices: [0, 1, 3],
                opposite: 2,
                parent_cell: 0,
                generation: 1,
            },
            FrontierFace {
                vertices: [1, 2, 3],
                opposite: 0,
                parent_cell: 0,
                generation: 1,
            },
        ];

        for _generation in 1..=config.depth {
            let mut next_frontier = Vec::with_capacity(frontier.len().saturating_mul(2));
            for (branch_index, face) in frontier.into_iter().enumerate() {
                let [a, b, c] = face.vertices;
                let point_a = node_position(&nodes, a)?;
                let point_b = node_position(&nodes, b)?;
                let point_c = node_position(&nodes, c)?;
                let opposite = node_position(&nodes, face.opposite)?;

                let cross = (point_b - point_a).cross(point_c - point_a);
                let twice_area = cross.norm();
                let area = 0.5 * twice_area;
                if !area.is_finite() || area < config.min_face_area_m2 {
                    return Err(TetrahedralError::DegenerateFace {
                        cell_id: face.parent_cell,
                        area_bits: area.to_bits(),
                    });
                }
                let mut normal = cross
                    .normalized()
                    .ok_or(TetrahedralError::NonFiniteGeometry)?;
                let centroid = (point_a + point_b + point_c).scale(1.0 / 3.0);
                if normal.dot(opposite - centroid) > 0.0 {
                    normal = normal.scale(-1.0);
                }
                let height = (2.0_f64 / 3.0).sqrt() * edge;
                let apex_position = centroid + normal.scale(height);
                if !apex_position.is_finite() {
                    return Err(TetrahedralError::NonFiniteGeometry);
                }

                // Do not create a hub whose connector count exceeds the
                // configured manufacturing interface budget.
                if [a, b, c]
                    .iter()
                    .any(|vertex| node_degrees[*vertex as usize] >= config.max_ports_per_node)
                {
                    blocked_attachment_faces += 1;
                    continue;
                }

                let apex_id =
                    u32::try_from(nodes.len()).map_err(|_| TetrahedralError::ArithmeticOverflow)?;
                let cell_id =
                    u32::try_from(cells.len()).map_err(|_| TetrahedralError::ArithmeticOverflow)?;

                nodes.push(TetraNode {
                    id: apex_id,
                    position: apex_position,
                    generation: face.generation,
                });
                for endpoint in [a, b, c] {
                    push_member(&mut members, &mut seen_edges, endpoint, apex_id)?;
                    node_degrees[endpoint as usize] += 1;
                }
                node_degrees.push(3);

                cells.push(TetraCell {
                    id: cell_id,
                    vertices: [a, b, c, apex_id],
                    generation: face.generation,
                    parent_cell: Some(face.parent_cell),
                    attachment_face: Some([a, b, c]),
                    attachment_interface: Some(TetraAttachmentInterface::TriangularThreePointV1),
                });

                // A tetrahedron has three faces other than its parent face.
                // Alternate which two become active to spread the branching
                // grammar instead of repeatedly selecting the same corner.
                let child_faces = if branch_index % 2 == 0 {
                    [
                        FrontierFace {
                            vertices: [a, b, apex_id],
                            opposite: c,
                            parent_cell: cell_id,
                            generation: face.generation + 1,
                        },
                        FrontierFace {
                            vertices: [b, c, apex_id],
                            opposite: a,
                            parent_cell: cell_id,
                            generation: face.generation + 1,
                        },
                    ]
                } else {
                    [
                        FrontierFace {
                            vertices: [b, c, apex_id],
                            opposite: a,
                            parent_cell: cell_id,
                            generation: face.generation + 1,
                        },
                        FrontierFace {
                            vertices: [c, a, apex_id],
                            opposite: b,
                            parent_cell: cell_id,
                            generation: face.generation + 1,
                        },
                    ]
                };
                next_frontier.extend(child_faces);
            }
            frontier = next_frontier;
        }

        let assembly = Self {
            config,
            nodes,
            members,
            cells,
            blocked_attachment_faces,
        };
        assembly.audit()?;
        Ok(assembly)
    }

    pub fn audit(&self) -> Result<TetrahedralMetrics, TetrahedralError> {
        self.config.validate()?;
        if self.cells.is_empty()
            || self.cells.len() > self.config.expected_cell_count()?
            || self.nodes.len()
                != self
                    .cells
                    .len()
                    .checked_add(3)
                    .ok_or(TetrahedralError::ArithmeticOverflow)?
            || self.members.len()
                != self
                    .cells
                    .len()
                    .checked_mul(3)
                    .and_then(|count| count.checked_add(3))
                    .ok_or(TetrahedralError::ArithmeticOverflow)?
        {
            return Err(TetrahedralError::InvalidGraph(
                "adaptive counts do not match the declared attachment grammar",
            ));
        }

        let mut bounds_min = Vec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut bounds_max = Vec3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (index, node) in self.nodes.iter().enumerate() {
            if node.id as usize != index || !node.position.is_finite() {
                return Err(TetrahedralError::NonFiniteGeometry);
            }
            bounds_min.x = bounds_min.x.min(node.position.x);
            bounds_min.y = bounds_min.y.min(node.position.y);
            bounds_min.z = bounds_min.z.min(node.position.z);
            bounds_max.x = bounds_max.x.max(node.position.x);
            bounds_max.y = bounds_max.y.max(node.position.y);
            bounds_max.z = bounds_max.z.max(node.position.z);
        }

        let edge_tolerance = self.config.root_edge_length_m.max(1.0) * 1e-8;
        let mut adjacency = vec![Vec::<usize>::new(); self.nodes.len()];
        let mut seen = HashSet::with_capacity(self.members.len());
        let mut total_length = 0.0;
        let mut minimum_length = f64::INFINITY;
        let mut maximum_length: f64 = 0.0;

        for (index, member) in self.members.iter().enumerate() {
            if member.id as usize != index
                || member.from as usize >= self.nodes.len()
                || member.to as usize >= self.nodes.len()
                || member.from == member.to
            {
                return Err(TetrahedralError::InvalidMember {
                    member_id: member.id,
                });
            }
            let key = if member.from < member.to {
                (member.from, member.to)
            } else {
                (member.to, member.from)
            };
            if !seen.insert(key) {
                return Err(TetrahedralError::DuplicateMember { a: key.0, b: key.1 });
            }
            let length = self.nodes[member.from as usize]
                .position
                .distance(self.nodes[member.to as usize].position);
            if !length.is_finite()
                || length < self.config.min_member_length_m
                || (length - self.config.root_edge_length_m).abs() > edge_tolerance
            {
                return Err(TetrahedralError::InvalidMember {
                    member_id: member.id,
                });
            }
            total_length += length;
            minimum_length = minimum_length.min(length);
            maximum_length = maximum_length.max(length);
            adjacency[member.from as usize].push(member.to as usize);
            adjacency[member.to as usize].push(member.from as usize);
        }

        let mut min_face_area = f64::INFINITY;
        let mut min_cell_volume = f64::INFINITY;
        for (index, cell) in self.cells.iter().enumerate() {
            if cell.id as usize != index {
                return Err(TetrahedralError::InvalidGraph(
                    "cell IDs are not contiguous",
                ));
            }
            let mut unique = HashSet::with_capacity(4);
            for vertex in cell.vertices {
                if vertex as usize >= self.nodes.len() || !unique.insert(vertex) {
                    return Err(TetrahedralError::InvalidGraph(
                        "cell has repeated or unknown vertex",
                    ));
                }
            }

            let [a, b, c, d] = cell.vertices;
            let pa = node_position(&self.nodes, a)?;
            let pb = node_position(&self.nodes, b)?;
            let pc = node_position(&self.nodes, c)?;
            let pd = node_position(&self.nodes, d)?;
            let volume = ((pb - pa).cross(pc - pa)).dot(pd - pa).abs() / 6.0;
            if !volume.is_finite() || volume <= 0.0 {
                return Err(TetrahedralError::DegenerateCell { cell_id: cell.id });
            }
            min_cell_volume = min_cell_volume.min(volume);

            let face_area = 0.5 * (pb - pa).cross(pc - pa).norm();
            if !face_area.is_finite() || face_area < self.config.min_face_area_m2 {
                return Err(TetrahedralError::DegenerateFace {
                    cell_id: cell.id,
                    area_bits: face_area.to_bits(),
                });
            }
            min_face_area = min_face_area.min(face_area);

            // Every tetrahedron must have all six edges represented.
            for (u, v) in [(a, b), (a, c), (a, d), (b, c), (b, d), (c, d)] {
                let key = if u < v { (u, v) } else { (v, u) };
                if !seen.contains(&key) {
                    return Err(TetrahedralError::InvalidGraph(
                        "tetrahedral cell edge is missing",
                    ));
                }
            }

            match (
                cell.parent_cell,
                cell.attachment_face,
                cell.attachment_interface,
            ) {
                (None, None, None) if cell.id == 0 => {}
                (
                    Some(parent),
                    Some(face),
                    Some(TetraAttachmentInterface::TriangularThreePointV1),
                ) if parent < cell.id => {
                    let parent_cell = self
                        .cells
                        .get(parent as usize)
                        .ok_or(TetrahedralError::InvalidAttachment { cell_id: cell.id })?;
                    if !face
                        .iter()
                        .all(|vertex| parent_cell.vertices.contains(vertex))
                        || !face.iter().all(|vertex| cell.vertices.contains(vertex))
                    {
                        return Err(TetrahedralError::InvalidAttachment { cell_id: cell.id });
                    }
                    let fp0 = node_position(&self.nodes, face[0])?;
                    let fp1 = node_position(&self.nodes, face[1])?;
                    let fp2 = node_position(&self.nodes, face[2])?;
                    let area = 0.5 * (fp1 - fp0).cross(fp2 - fp0).norm();
                    if !area.is_finite() || area < self.config.min_face_area_m2 {
                        return Err(TetrahedralError::InvalidAttachment { cell_id: cell.id });
                    }
                }
                _ => return Err(TetrahedralError::InvalidAttachment { cell_id: cell.id }),
            }
        }

        let mut components = 0usize;
        let mut visited = vec![false; self.nodes.len()];
        let mut maximum_degree = 0usize;
        for (start, adjacent) in adjacency.iter().enumerate() {
            maximum_degree = maximum_degree.max(adjacent.len());
            if adjacent.len() > self.config.max_ports_per_node {
                return Err(TetrahedralError::PortBudgetExceeded {
                    node_id: self.nodes[start].id,
                    required: adjacent.len(),
                    maximum: self.config.max_ports_per_node,
                });
            }
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
            return Err(TetrahedralError::Disconnected { components });
        }

        // Check angular room for indexed connector ports. This is only a
        // geometric minimum-separation check, not a connector strength model.
        for (node_index, neighbors) in adjacency.iter().enumerate() {
            let origin = self.nodes[node_index].position;
            let directions = neighbors
                .iter()
                .map(|&neighbor| {
                    (self.nodes[neighbor].position - origin)
                        .normalized()
                        .ok_or(TetrahedralError::NonFiniteGeometry)
                })
                .collect::<Result<Vec<_>, _>>()?;
            for left in 0..directions.len() {
                for right in (left + 1)..directions.len() {
                    let cosine = directions[left].dot(directions[right]).clamp(-1.0, 1.0);
                    let angle = cosine.acos();
                    if angle < self.config.min_port_separation_rad {
                        return Err(TetrahedralError::PortsTooClose {
                            node_id: self.nodes[node_index].id,
                            angle_rad: angle,
                            minimum_rad: self.config.min_port_separation_rad,
                        });
                    }
                }
            }
        }

        if !total_length.is_finite() || !min_face_area.is_finite() || !min_cell_volume.is_finite() {
            return Err(TetrahedralError::NonFiniteGeometry);
        }
        let cycle_rank = self
            .members
            .len()
            .checked_sub(self.nodes.len())
            .and_then(|count| count.checked_add(components))
            .ok_or(TetrahedralError::InvalidGraph(
                "invalid edge/node/component counts",
            ))?;

        Ok(TetrahedralMetrics {
            node_count: self.nodes.len(),
            member_count: self.members.len(),
            cell_count: self.cells.len(),
            connected_components: components,
            cycle_rank,
            maximum_node_degree: maximum_degree,
            total_member_length_m: total_length,
            minimum_member_length_m: minimum_length,
            maximum_member_length_m: maximum_length,
            minimum_attachment_face_area_m2: min_face_area,
            minimum_cell_volume_m3: min_cell_volume,
            blocked_attachment_faces: self.blocked_attachment_faces,
            bounds_min,
            bounds_max,
            same_build_digest_non_security: self.digest_non_security(),
            ideal_attachment_contract: "root regular tetrahedron plus child regular tetrahedra attached across three non-collinear shared vertices; ideal pin-jointed kinematic construction only",
            claim_ceiling: "geometry/topology only; no joint stiffness, stress, buckling, fatigue, fracture, manufacturing, safety, or flight qualification",
        })
    }

    /// FNV-1a regression digest only. It is not cryptographic.
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
            self.config.root_edge_length_m,
            self.config.min_member_length_m,
            self.config.min_face_area_m2,
            self.config.min_port_separation_rad,
        ] {
            mix(&mut hash, &value.to_bits().to_le_bytes());
        }
        mix(&mut hash, &(self.config.max_nodes as u64).to_le_bytes());
        mix(
            &mut hash,
            &(self.config.max_ports_per_node as u64).to_le_bytes(),
        );
        mix(
            &mut hash,
            &(self.blocked_attachment_faces as u64).to_le_bytes(),
        );
        for node in &self.nodes {
            mix(&mut hash, &node.id.to_le_bytes());
            mix(&mut hash, &[node.generation]);
            for coordinate in [node.position.x, node.position.y, node.position.z] {
                mix(&mut hash, &coordinate.to_bits().to_le_bytes());
            }
        }
        for member in &self.members {
            mix(&mut hash, &member.id.to_le_bytes());
            mix(&mut hash, &member.from.to_le_bytes());
            mix(&mut hash, &member.to.to_le_bytes());
        }
        for cell in &self.cells {
            mix(&mut hash, &cell.id.to_le_bytes());
            for vertex in cell.vertices {
                mix(&mut hash, &vertex.to_le_bytes());
            }
            mix(&mut hash, &[cell.generation]);
            mix(
                &mut hash,
                &cell.parent_cell.unwrap_or(u32::MAX).to_le_bytes(),
            );
            if let Some(face) = cell.attachment_face {
                for vertex in face {
                    mix(&mut hash, &vertex.to_le_bytes());
                }
            }
        }
        hash
    }
}

fn node_position(nodes: &[TetraNode], id: u32) -> Result<Vec3, TetrahedralError> {
    nodes
        .get(id as usize)
        .filter(|node| node.id == id)
        .map(|node| node.position)
        .ok_or(TetrahedralError::InvalidGraph("unknown node ID"))
}

fn push_member(
    members: &mut Vec<TetraMember>,
    seen: &mut HashSet<(u32, u32)>,
    from: u32,
    to: u32,
) -> Result<(), TetrahedralError> {
    let key = if from < to { (from, to) } else { (to, from) };
    if !seen.insert(key) {
        return Err(TetrahedralError::DuplicateMember { a: key.0, b: key.1 });
    }
    let id = u32::try_from(members.len()).map_err(|_| TetrahedralError::ArithmeticOverflow)?;
    members.push(TetraMember {
        id,
        from,
        to,
        interface: TetraMemberInterface::IndexedPinEndV1,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_four_assembly_respects_port_budget_with_adaptive_cell_count() {
        let assembly = TetrahedralAssembly::generate(TetrahedralConfig::default()).unwrap();
        let metrics = assembly.audit().unwrap();
        assert!(metrics.cell_count > 1);
        assert!(metrics.cell_count <= 31);
        assert_eq!(metrics.node_count, metrics.cell_count + 3);
        assert_eq!(metrics.member_count, metrics.cell_count * 3 + 3);
        assert_eq!(metrics.connected_components, 1);
        assert_eq!(
            metrics.cycle_rank,
            metrics.member_count - metrics.node_count + 1
        );
        assert!(metrics.maximum_node_degree <= 16);
        assert!(metrics.blocked_attachment_faces > 0);
    }

    #[test]
    fn three_port_budget_keeps_only_root_cell_without_overloaded_hubs() {
        let config = TetrahedralConfig {
            max_ports_per_node: 3,
            ..TetrahedralConfig::default()
        };
        let assembly = TetrahedralAssembly::generate(config).unwrap();
        let metrics = assembly.audit().unwrap();
        assert_eq!(metrics.cell_count, 1);
        assert_eq!(metrics.node_count, 4);
        assert_eq!(metrics.member_count, 6);
        assert!(metrics.blocked_attachment_faces > 0);
        assert_eq!(metrics.maximum_node_degree, 3);
    }

    #[test]
    fn every_cell_is_a_regular_tetrahedron_and_every_child_uses_a_face() {
        let assembly = TetrahedralAssembly::generate(TetrahedralConfig::default()).unwrap();
        let edge = assembly.config.root_edge_length_m;
        for cell in &assembly.cells {
            let [a, b, c, d] = cell.vertices;
            let points = [a, b, c, d].map(|id| assembly.nodes[id as usize].position);
            for (left, right) in [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)] {
                assert!((points[left].distance(points[right]) - edge).abs() < 1e-8);
            }
            if let Some(parent_id) = cell.parent_cell {
                let parent = &assembly.cells[parent_id as usize];
                let face = cell.attachment_face.unwrap();
                assert!(face.iter().all(|vertex| parent.vertices.contains(vertex)));
            }
        }
    }

    #[test]
    fn same_configuration_produces_same_build_digest() {
        let left = TetrahedralAssembly::generate(TetrahedralConfig::default()).unwrap();
        let right = TetrahedralAssembly::generate(TetrahedralConfig::default()).unwrap();
        assert_eq!(left.digest_non_security(), right.digest_non_security());
        assert_eq!(left.nodes, right.nodes);
        assert_eq!(left.members, right.members);
        assert_eq!(left.cells, right.cells);
    }

    #[test]
    fn node_budget_is_checked_before_allocation() {
        let config = TetrahedralConfig {
            depth: 8,
            max_nodes: 100,
            ..TetrahedralConfig::default()
        };
        assert_eq!(
            config.validate(),
            Err(TetrahedralError::NodeBudgetExceeded {
                required: 514,
                maximum: 100,
            })
        );
    }

    #[test]
    fn invalid_dimensions_and_port_contracts_are_rejected() {
        let config = TetrahedralConfig {
            root_edge_length_m: f64::NAN,
            ..TetrahedralConfig::default()
        };
        assert_eq!(config.validate(), Err(TetrahedralError::InvalidEdgeLength));

        let config = TetrahedralConfig {
            max_ports_per_node: 2,
            ..TetrahedralConfig::default()
        };
        assert_eq!(config.validate(), Err(TetrahedralError::InvalidPortBudget));

        let config = TetrahedralConfig {
            min_port_separation_rad: f64::NAN,
            ..TetrahedralConfig::default()
        };
        assert_eq!(
            config.validate(),
            Err(TetrahedralError::InvalidPortSeparation)
        );
    }
}
