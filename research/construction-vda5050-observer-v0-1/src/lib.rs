#![forbid(unsafe_code)]

//! Read-only VDA 5050 v3.0.0 message observer for sealed-trace replay.
//!
//! This is not an MQTT client, dispatcher, robot controller, safety monitor,
//! authentication service, or construction acceptance authority.

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value, json};
use sha2::{Digest, Sha256};
use std::fmt;

const MAX_PAYLOAD_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VdaTopic {
    Order,
    State,
    Factsheet,
    ZoneSet,
    Connection,
    InstantActions,
    Responses,
}

impl VdaTopic {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Order => "order",
            Self::State => "state",
            Self::Factsheet => "factsheet",
            Self::ZoneSet => "zoneSet",
            Self::Connection => "connection",
            Self::InstantActions => "instantActions",
            Self::Responses => "responses",
        }
    }

    fn parse(value: &str) -> Result<Self, ObserverError> {
        match value {
            "order" => Ok(Self::Order),
            "state" => Ok(Self::State),
            "factsheet" => Ok(Self::Factsheet),
            "zoneSet" => Ok(Self::ZoneSet),
            "connection" => Ok(Self::Connection),
            "instantActions" => Ok(Self::InstantActions),
            "responses" => Ok(Self::Responses),
            other => Err(ObserverError::UnsupportedTopic(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicIdentity {
    pub manufacturer: String,
    pub serial_number: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VdaHeader {
    pub header_id: u64,
    pub timestamp: String,
    pub version: String,
    pub manufacturer: String,
    pub serial_number: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteNodeObservation {
    pub node_id: String,
    pub sequence_id: u64,
    pub released: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteEdgeObservation {
    pub edge_id: String,
    pub sequence_id: u64,
    pub released: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionStateObservation {
    pub action_id: String,
    pub action_type: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneRequestObservation {
    pub request_id: String,
    pub request_type: String,
    pub zone_id: String,
    pub zone_set_id: String,
    pub request_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationMapObservation {
    pub map_id: String,
    pub map_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetZoneObservation {
    pub zone_id: String,
    pub zone_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstantActionObservation {
    pub action_id: String,
    pub action_type: String,
    pub blocking_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneResponseObservation {
    pub request_id: String,
    pub grant_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observation {
    TransportOrderReceived {
        order_id: String,
        order_update_id: u64,
        nodes: Vec<RouteNodeObservation>,
        edges: Vec<RouteEdgeObservation>,
        action_count: usize,
    },
    RobotStateReported {
        order_id: String,
        order_update_id: u64,
        last_node_id: String,
        last_node_sequence_id: u64,
        nodes: Vec<RouteNodeObservation>,
        edges: Vec<RouteEdgeObservation>,
        action_states: Vec<ActionStateObservation>,
        instant_action_states: Vec<ActionStateObservation>,
        zone_requests: Vec<ZoneRequestObservation>,
        navigation_maps: Vec<NavigationMapObservation>,
        driving: bool,
        operating_mode: String,
        error_count: usize,
        power_supply_reported: bool,
        safety_state_reported: bool,
    },
    FactsheetCapabilityClaimReported {
        capability_payload_sha256: String,
        type_specification_present: bool,
    },
    FleetZoneSetReported {
        map_id: String,
        zone_set_id: String,
        zones: Vec<FleetZoneObservation>,
    },
    RobotConnectionStateReported {
        connection_state: String,
    },
    InstantActionsReported {
        actions: Vec<InstantActionObservation>,
    },
    FleetZoneResponsesReported {
        responses: Vec<ZoneResponseObservation>,
    },
}

impl Observation {
    /// Output contains no task acceptance, safety, motion-authority or
    /// authentication result.
    pub fn summary_json(&self) -> Value {
        match self {
            Self::TransportOrderReceived {
                order_id,
                order_update_id,
                nodes,
                edges,
                action_count,
            } => json!({
                "kind": "transport_order_received",
                "order_id": order_id,
                "order_update_id": order_update_id,
                "node_count": nodes.len(),
                "edge_count": edges.len(),
                "action_count": action_count
            }),
            Self::RobotStateReported {
                order_id,
                order_update_id,
                last_node_id,
                last_node_sequence_id,
                nodes,
                edges,
                action_states,
                instant_action_states,
                zone_requests,
                navigation_maps,
                driving,
                operating_mode,
                error_count,
                power_supply_reported,
                safety_state_reported,
            } => json!({
                "kind": "robot_state_reported",
                "order_id": order_id,
                "order_update_id": order_update_id,
                "last_node_id": last_node_id,
                "last_node_sequence_id": last_node_sequence_id,
                "nodes": nodes.iter().map(|item| json!({
                    "node_id": item.node_id,
                    "sequence_id": item.sequence_id,
                    "released": item.released
                })).collect::<Vec<_>>(),
                "edges": edges.iter().map(|item| json!({
                    "edge_id": item.edge_id,
                    "sequence_id": item.sequence_id,
                    "released": item.released
                })).collect::<Vec<_>>(),
                "action_states": action_states.iter().map(|item| json!({
                    "action_id": item.action_id,
                    "action_type": item.action_type,
                    "status": item.status
                })).collect::<Vec<_>>(),
                "instant_action_states": instant_action_states.iter().map(|item| json!({
                    "action_id": item.action_id,
                    "action_type": item.action_type,
                    "status": item.status
                })).collect::<Vec<_>>(),
                "driving": driving,
                "operating_mode": operating_mode,
                "error_count": error_count,
                "power_supply_reported": power_supply_reported,
                "zone_requests": zone_requests.iter().map(|item| json!({
                    "request_id": item.request_id,
                    "request_type": item.request_type,
                    "zone_id": item.zone_id,
                    "zone_set_id": item.zone_set_id,
                    "request_status": item.request_status
                })).collect::<Vec<_>>(),
                "navigation_maps": navigation_maps.iter().map(|item| json!({
                    "map_id": item.map_id,
                    "map_version": item.map_version
                })).collect::<Vec<_>>(),
                "safety_state_reported": safety_state_reported
            }),
            Self::FactsheetCapabilityClaimReported {
                capability_payload_sha256,
                type_specification_present,
            } => json!({
                "kind": "factsheet_capability_claim_reported",
                "capability_payload_sha256": capability_payload_sha256,
                "type_specification_present": type_specification_present
            }),
            Self::FleetZoneSetReported {
                map_id,
                zone_set_id,
                zones,
            } => json!({
                "kind": "fleet_zone_set_reported",
                "map_id": map_id,
                "zone_set_id": zone_set_id,
                "zones": zones.iter().map(|item| json!({
                    "zone_id": item.zone_id,
                    "zone_type": item.zone_type
                })).collect::<Vec<_>>()
            }),
            Self::RobotConnectionStateReported { connection_state } => json!({
                "kind": "robot_connection_state_reported",
                "connection_state": connection_state
            }),
            Self::InstantActionsReported { actions } => json!({
                "kind": "instant_actions_reported",
                "actions": actions.iter().map(|item| json!({
                    "action_id": item.action_id,
                    "action_type": item.action_type,
                    "blocking_type": item.blocking_type
                })).collect::<Vec<_>>()
            }),
            Self::FleetZoneResponsesReported { responses } => json!({
                "kind": "fleet_zone_responses_reported",
                "responses": responses.iter().map(|item| json!({
                    "request_id": item.request_id,
                    "grant_type": item.grant_type
                })).collect::<Vec<_>>()
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedMessage {
    pub topic: VdaTopic,
    pub header: VdaHeader,
    pub topic_identity: Option<TopicIdentity>,
    pub payload_sha256: String,
    pub observation: Observation,
}

impl DecodedMessage {
    pub fn summary_json(&self) -> Value {
        json!({
            "protocol": "VDA-5050",
            "version": self.header.version,
            "topic": self.topic.as_str(),
            "header": {
                "header_id": self.header.header_id,
                "timestamp": self.header.timestamp,
                "manufacturer": self.header.manufacturer,
                "serial_number": self.header.serial_number
            },
            "topic_identity": self.topic_identity.as_ref().map(|identity| json!({
                "manufacturer": identity.manufacturer,
                "serial_number": identity.serial_number
            })),
            "payload_sha256": self.payload_sha256,
            "observation": self.observation.summary_json()
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObserverError {
    Json(String),
    DuplicateKey(String),
    UnsupportedTopic(String),
    UnsupportedVersion(String),
    TopicVersionMismatch(String),
    TopicIdentityMismatch { field: &'static str },
    PayloadTooLarge { actual: usize, maximum: usize },
    Shape(String),
}

impl fmt::Display for ObserverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(message) => write!(f, "invalid JSON: {message}"),
            Self::DuplicateKey(key) => write!(f, "duplicate JSON object key: {key}"),
            Self::UnsupportedTopic(topic) => write!(f, "unsupported VDA topic: {topic}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported VDA payload version: {version}")
            }
            Self::TopicVersionMismatch(version) => {
                write!(f, "topic major version does not match VDA 3.0: {version}")
            }
            Self::TopicIdentityMismatch { field } => {
                write!(f, "topic identity does not match payload {field}")
            }
            Self::PayloadTooLarge { actual, maximum } => {
                write!(f, "payload size {actual} exceeds observer limit {maximum}")
            }
            Self::Shape(message) => write!(f, "invalid VDA message shape: {message}"),
        }
    }
}

impl std::error::Error for ObserverError {}

/// Decode one exact MQTT topic/payload pair. A suggested local five-level VDA
/// topic is recognized, while cloud-specific broker hierarchies remain valid
/// when the topic name and identity are supplied explicitly.
pub fn decode_vda_message(
    topic_path: &str,
    topic_identity: Option<TopicIdentity>,
    payload: &[u8],
) -> Result<DecodedMessage, ObserverError> {
    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err(ObserverError::PayloadTooLarge {
            actual: payload.len(),
            maximum: MAX_PAYLOAD_BYTES,
        });
    }
    let parsed_topic = parse_topic_path(topic_path)?;
    let value = parse_unique_json(payload)?;
    let root = as_object(&value, "message")?;
    let header = parse_header(root)?;
    if header.version != "3.0.0" {
        return Err(ObserverError::UnsupportedVersion(header.version.clone()));
    }
    let identity = merge_and_check_identity(parsed_topic.identity, topic_identity, &header)?;
    let observation = match parsed_topic.topic {
        VdaTopic::Order => parse_order(root)?,
        VdaTopic::State => parse_state(root)?,
        VdaTopic::Factsheet => parse_factsheet(root, payload)?,
        VdaTopic::ZoneSet => parse_zone_set(root)?,
        VdaTopic::Connection => parse_connection(root)?,
        VdaTopic::InstantActions => parse_instant_actions(root)?,
        VdaTopic::Responses => parse_responses(root)?,
    };
    Ok(DecodedMessage {
        topic: parsed_topic.topic,
        header,
        topic_identity: identity,
        payload_sha256: sha256_hex(payload),
        observation,
    })
}

struct ParsedTopicPath {
    topic: VdaTopic,
    identity: Option<TopicIdentity>,
}

fn parse_topic_path(path: &str) -> Result<ParsedTopicPath, ObserverError> {
    let segments: Vec<&str> = path.split('/').collect();
    if segments.iter().any(|segment| segment.is_empty()) {
        return Err(ObserverError::Shape(
            "MQTT topic path contains an empty level".to_owned(),
        ));
    }
    if segments.len() == 5 && segments[1].starts_with('v') {
        if segments[1] != "v3" {
            return Err(ObserverError::TopicVersionMismatch(segments[1].to_owned()));
        }
        return Ok(ParsedTopicPath {
            topic: VdaTopic::parse(segments[4])?,
            identity: Some(TopicIdentity {
                manufacturer: segments[2].to_owned(),
                serial_number: segments[3].to_owned(),
            }),
        });
    }
    let topic_name = segments
        .last()
        .ok_or_else(|| ObserverError::Shape("MQTT topic path is empty".to_owned()))?;
    Ok(ParsedTopicPath {
        topic: VdaTopic::parse(topic_name)?,
        identity: None,
    })
}

fn merge_and_check_identity(
    from_topic: Option<TopicIdentity>,
    supplied: Option<TopicIdentity>,
    header: &VdaHeader,
) -> Result<Option<TopicIdentity>, ObserverError> {
    if let Some(identity) = &from_topic {
        if identity.manufacturer != header.manufacturer {
            return Err(ObserverError::TopicIdentityMismatch {
                field: "manufacturer",
            });
        }
        if identity.serial_number != header.serial_number {
            return Err(ObserverError::TopicIdentityMismatch {
                field: "serialNumber",
            });
        }
    }
    if let Some(identity) = &supplied {
        if identity.manufacturer != header.manufacturer {
            return Err(ObserverError::TopicIdentityMismatch {
                field: "manufacturer",
            });
        }
        if identity.serial_number != header.serial_number {
            return Err(ObserverError::TopicIdentityMismatch {
                field: "serialNumber",
            });
        }
    }
    if let (Some(path_identity), Some(explicit_identity)) = (&from_topic, &supplied) {
        if path_identity != explicit_identity {
            return shape("explicit topic identity differs from five-level MQTT topic identity");
        }
    }
    Ok(supplied.or(from_topic))
}

fn parse_header(root: &Map<String, Value>) -> Result<VdaHeader, ObserverError> {
    let header_id = unsigned(field(root, "headerId")?, "headerId")?;
    let timestamp = string(field(root, "timestamp")?, "timestamp")?.to_owned();
    if !looks_like_rfc3339(&timestamp) {
        return shape("timestamp must be an RFC3339 date-time string");
    }
    Ok(VdaHeader {
        header_id,
        timestamp,
        version: non_empty_string(field(root, "version")?, "version")?.to_owned(),
        manufacturer: non_empty_string(field(root, "manufacturer")?, "manufacturer")?.to_owned(),
        serial_number: non_empty_string(field(root, "serialNumber")?, "serialNumber")?.to_owned(),
    })
}

fn parse_order(root: &Map<String, Value>) -> Result<Observation, ObserverError> {
    let order_id = non_empty_string(field(root, "orderId")?, "orderId")?.to_owned();
    let order_update_id = unsigned(field(root, "orderUpdateId")?, "orderUpdateId")?;
    let nodes = parse_route_nodes(as_array(field(root, "nodes")?, "nodes")?)?;
    let edges = parse_route_edges(as_array(field(root, "edges")?, "edges")?)?;
    validate_order_edges(field(root, "edges")?)?;
    let action_count = validate_and_count_order_actions(root)?;
    Ok(Observation::TransportOrderReceived {
        order_id,
        order_update_id,
        nodes,
        edges,
        action_count,
    })
}

fn parse_state(root: &Map<String, Value>) -> Result<Observation, ObserverError> {
    let order_id = string(field(root, "orderId")?, "orderId")?.to_owned();
    let order_update_id = unsigned(field(root, "orderUpdateId")?, "orderUpdateId")?;
    let last_node_id = string(field(root, "lastNodeId")?, "lastNodeId")?.to_owned();
    let last_node_sequence_id = unsigned(field(root, "lastNodeSequenceId")?, "lastNodeSequenceId")?;
    let nodes = parse_route_nodes(as_array(field(root, "nodeStates")?, "nodeStates")?)?;
    let edges = parse_route_edges(as_array(field(root, "edgeStates")?, "edgeStates")?)?;
    let action_states =
        parse_action_states(as_array(field(root, "actionStates")?, "actionStates")?)?;
    let instant_action_states = parse_action_states(as_array(
        field(root, "instantActionStates")?,
        "instantActionStates",
    )?)?;
    let driving = boolean(field(root, "driving")?, "driving")?;
    let operating_mode =
        non_empty_string(field(root, "operatingMode")?, "operatingMode")?.to_owned();
    let errors = as_array(field(root, "errors")?, "errors")?;
    let power_supply = as_object(field(root, "powerSupply")?, "powerSupply")?;
    let state_of_charge = number(
        field(power_supply, "stateOfCharge")?,
        "powerSupply.stateOfCharge",
    )?;
    if !(0.0..=100.0).contains(&state_of_charge) {
        return shape("powerSupply.stateOfCharge must be in the range 0 to 100");
    }
    boolean(field(power_supply, "charging")?, "powerSupply.charging")?;
    let safety_state = as_object(field(root, "safetyState")?, "safetyState")?;
    let active_emergency_stop = non_empty_string(
        field(safety_state, "activeEmergencyStop")?,
        "safetyState.activeEmergencyStop",
    )?;
    if !["MANUAL", "REMOTE", "NONE"].contains(&active_emergency_stop) {
        return shape("safetyState.activeEmergencyStop is outside the VDA 5050 v3.0.0 enumeration");
    }
    boolean(
        field(safety_state, "fieldViolation")?,
        "safetyState.fieldViolation",
    )?;
    let zone_requests = match root.get("zoneRequests") {
        Some(value) => parse_zone_requests(as_array(value, "zoneRequests")?)?,
        None => Vec::new(),
    };
    let navigation_maps = match root.get("maps") {
        Some(value) => parse_navigation_maps(as_array(value, "maps")?)?,
        None => Vec::new(),
    };
    Ok(Observation::RobotStateReported {
        order_id,
        order_update_id,
        last_node_id,
        last_node_sequence_id,
        nodes,
        edges,
        action_states,
        instant_action_states,
        zone_requests,
        navigation_maps,
        driving,
        operating_mode,
        error_count: errors.len(),
        power_supply_reported: true,
        safety_state_reported: !safety_state.is_empty() || root.contains_key("safetyState"),
    })
}

fn parse_factsheet(
    root: &Map<String, Value>,
    payload: &[u8],
) -> Result<Observation, ObserverError> {
    let type_specification = as_object(field(root, "typeSpecification")?, "typeSpecification")?;
    non_empty_string(
        field(type_specification, "seriesName")?,
        "typeSpecification.seriesName",
    )?;
    non_empty_string(
        field(type_specification, "mobileRobotKinematics")?,
        "typeSpecification.mobileRobotKinematics",
    )?;
    non_empty_string(
        field(type_specification, "mobileRobotClass")?,
        "typeSpecification.mobileRobotClass",
    )?;
    number(
        field(type_specification, "maximumLoadMass")?,
        "typeSpecification.maximumLoadMass",
    )?;
    as_array(
        field(type_specification, "localizationTypes")?,
        "typeSpecification.localizationTypes",
    )?;
    as_array(
        field(type_specification, "navigationTypes")?,
        "typeSpecification.navigationTypes",
    )?;

    let physical = as_object(field(root, "physicalParameters")?, "physicalParameters")?;
    for key in [
        "minimumSpeed",
        "maximumSpeed",
        "maximumAcceleration",
        "maximumDeceleration",
        "minimumHeight",
        "maximumHeight",
        "width",
        "length",
    ] {
        number(field(physical, key)?, &format!("physicalParameters.{key}"))?;
    }
    let limits = as_object(field(root, "protocolLimits")?, "protocolLimits")?;
    as_object(
        field(limits, "maximumStringLengths")?,
        "protocolLimits.maximumStringLengths",
    )?;
    as_object(
        field(limits, "maximumArrayLengths")?,
        "protocolLimits.maximumArrayLengths",
    )?;
    let timing = as_object(field(limits, "timing")?, "protocolLimits.timing")?;
    number(
        field(timing, "minimumOrderInterval")?,
        "timing.minimumOrderInterval",
    )?;
    number(
        field(timing, "minimumStateInterval")?,
        "timing.minimumStateInterval",
    )?;
    let features = as_object(field(root, "protocolFeatures")?, "protocolFeatures")?;
    as_array(
        field(features, "optionalParameters")?,
        "protocolFeatures.optionalParameters",
    )?;
    as_array(
        field(features, "mobileRobotActions")?,
        "protocolFeatures.mobileRobotActions",
    )?;
    as_object(field(root, "mobileRobotGeometry")?, "mobileRobotGeometry")?;
    as_object(field(root, "loadSpecification")?, "loadSpecification")?;
    Ok(Observation::FactsheetCapabilityClaimReported {
        capability_payload_sha256: sha256_hex(payload),
        type_specification_present: true,
    })
}

fn parse_zone_set(root: &Map<String, Value>) -> Result<Observation, ObserverError> {
    let zone_set = as_object(field(root, "zoneSet")?, "zoneSet")?;
    let map_id = non_empty_string(field(zone_set, "mapId")?, "zoneSet.mapId")?.to_owned();
    let zone_set_id =
        non_empty_string(field(zone_set, "zoneSetId")?, "zoneSet.zoneSetId")?.to_owned();
    let zones = as_array(field(zone_set, "zones")?, "zoneSet.zones")?;
    let mut parsed = Vec::with_capacity(zones.len());
    for zone in zones {
        let zone = as_object(zone, "zoneSet.zones[]")?;
        let zone_id = non_empty_string(field(zone, "zoneId")?, "zone.zoneId")?.to_owned();
        let zone_type = non_empty_string(field(zone, "zoneType")?, "zone.zoneType")?.to_owned();
        if ![
            "BLOCKED",
            "LINE_GUIDED",
            "RELEASE",
            "COORDINATED_REPLANNING",
            "SPEED_LIMIT",
            "ACTION",
            "PRIORITY",
            "PENALTY",
            "DIRECTED",
            "BIDIRECTED",
        ]
        .contains(&zone_type.as_str())
        {
            return shape("zoneType is outside the VDA 5050 v3.0.0 enumeration");
        }
        let vertices = as_array(field(zone, "vertices")?, "zone.vertices")?;
        if vertices.len() < 3 {
            return shape("zone vertices must contain at least three points");
        }
        for vertex in vertices {
            let vertex = as_object(vertex, "zone.vertices[]")?;
            number(field(vertex, "x")?, "vertex.x")?;
            number(field(vertex, "y")?, "vertex.y")?;
        }
        parsed.push(FleetZoneObservation { zone_id, zone_type });
    }
    Ok(Observation::FleetZoneSetReported {
        map_id,
        zone_set_id,
        zones: parsed,
    })
}

fn parse_connection(root: &Map<String, Value>) -> Result<Observation, ObserverError> {
    let connection_state =
        non_empty_string(field(root, "connectionState")?, "connectionState")?.to_owned();
    if !["ONLINE", "OFFLINE", "HIBERNATING", "CONNECTION_BROKEN"]
        .contains(&connection_state.as_str())
    {
        return shape("connectionState is outside the VDA 5050 v3.0.0 enumeration");
    }
    Ok(Observation::RobotConnectionStateReported { connection_state })
}

fn parse_instant_actions(root: &Map<String, Value>) -> Result<Observation, ObserverError> {
    let actions = as_array(field(root, "actions")?, "actions")?;
    let mut parsed = Vec::with_capacity(actions.len());
    for action in actions {
        let action = as_object(action, "actions[]")?;
        let action_id = non_empty_string(field(action, "actionId")?, "actionId")?.to_owned();
        let action_type = non_empty_string(field(action, "actionType")?, "actionType")?.to_owned();
        let blocking_type =
            non_empty_string(field(action, "blockingType")?, "blockingType")?.to_owned();
        if blocking_type != "NONE" {
            return shape("VDA 5050 v3 instantActions blockingType must be NONE");
        }
        parsed.push(InstantActionObservation {
            action_id,
            action_type,
            blocking_type,
        });
    }
    Ok(Observation::InstantActionsReported { actions: parsed })
}

fn parse_responses(root: &Map<String, Value>) -> Result<Observation, ObserverError> {
    let responses = as_array(field(root, "responses")?, "responses")?;
    let mut parsed = Vec::with_capacity(responses.len());
    for response in responses {
        let response = as_object(response, "responses[]")?;
        let request_id = non_empty_string(field(response, "requestId")?, "requestId")?.to_owned();
        let grant_type = non_empty_string(field(response, "grantType")?, "grantType")?.to_owned();
        if !["GRANTED", "QUEUED", "REVOKED", "REJECTED"].contains(&grant_type.as_str()) {
            return shape("grantType is outside the VDA 5050 v3.0.0 enumeration");
        }
        parsed.push(ZoneResponseObservation {
            request_id,
            grant_type,
        });
    }
    Ok(Observation::FleetZoneResponsesReported { responses: parsed })
}

fn parse_route_nodes(values: &[Value]) -> Result<Vec<RouteNodeObservation>, ObserverError> {
    let mut result = Vec::with_capacity(values.len());
    for value in values {
        let object = as_object(value, "nodes[]")?;
        result.push(RouteNodeObservation {
            node_id: non_empty_string(field(object, "nodeId")?, "nodeId")?.to_owned(),
            sequence_id: unsigned(field(object, "sequenceId")?, "sequenceId")?,
            released: boolean(field(object, "released")?, "released")?,
        });
    }
    Ok(result)
}

fn parse_route_edges(values: &[Value]) -> Result<Vec<RouteEdgeObservation>, ObserverError> {
    let mut result = Vec::with_capacity(values.len());
    for value in values {
        let object = as_object(value, "edges[]")?;
        result.push(RouteEdgeObservation {
            edge_id: non_empty_string(field(object, "edgeId")?, "edgeId")?.to_owned(),
            sequence_id: unsigned(field(object, "sequenceId")?, "sequenceId")?,
            released: boolean(field(object, "released")?, "released")?,
        });
    }
    Ok(result)
}

fn parse_action_states(values: &[Value]) -> Result<Vec<ActionStateObservation>, ObserverError> {
    let mut result = Vec::with_capacity(values.len());
    for value in values {
        let object = as_object(value, "actionStates[]")?;
        let action_id = non_empty_string(field(object, "actionId")?, "actionId")?.to_owned();
        let status = non_empty_string(field(object, "actionStatus")?, "actionStatus")?.to_owned();
        if ![
            "WAITING",
            "INITIALIZING",
            "RUNNING",
            "PAUSED",
            "RETRIABLE",
            "FINISHED",
            "FAILED",
        ]
        .contains(&status.as_str())
        {
            return shape("actionStatus is outside the VDA 5050 v3.0.0 enumeration");
        }
        let action_type = object
            .get("actionType")
            .map(|value| string(value, "actionType").map(str::to_owned))
            .transpose()?;
        result.push(ActionStateObservation {
            action_id,
            action_type,
            status,
        });
    }
    Ok(result)
}

fn parse_zone_requests(values: &[Value]) -> Result<Vec<ZoneRequestObservation>, ObserverError> {
    let mut result = Vec::with_capacity(values.len());
    for value in values {
        let object = as_object(value, "zoneRequests[]")?;
        let request_id = non_empty_string(field(object, "requestId")?, "requestId")?.to_owned();
        let request_type =
            non_empty_string(field(object, "requestType")?, "requestType")?.to_owned();
        let zone_id = non_empty_string(field(object, "zoneId")?, "zoneId")?.to_owned();
        let zone_set_id = non_empty_string(field(object, "zoneSetId")?, "zoneSetId")?.to_owned();
        let request_status =
            non_empty_string(field(object, "requestStatus")?, "requestStatus")?.to_owned();
        if !["ACCESS", "REPLANNING"].contains(&request_type.as_str()) {
            return shape("zone request type is outside the VDA 5050 v3.0.0 enumeration");
        }
        if !["REQUESTED", "GRANTED", "REVOKED", "EXPIRED"].contains(&request_status.as_str()) {
            return shape("zone request status is outside the VDA 5050 v3.0.0 enumeration");
        }
        result.push(ZoneRequestObservation {
            request_id,
            request_type,
            zone_id,
            zone_set_id,
            request_status,
        });
    }
    Ok(result)
}

fn parse_navigation_maps(values: &[Value]) -> Result<Vec<NavigationMapObservation>, ObserverError> {
    let mut result = Vec::with_capacity(values.len());
    for value in values {
        let object = as_object(value, "maps[]")?;
        let map_id = non_empty_string(field(object, "mapId")?, "mapId")?.to_owned();
        let map_version = object
            .get("mapVersion")
            .map(|value| string(value, "mapVersion").map(str::to_owned))
            .transpose()?;
        result.push(NavigationMapObservation {
            map_id,
            map_version,
        });
    }
    Ok(result)
}

fn validate_order_edges(value: &Value) -> Result<(), ObserverError> {
    for edge in as_array(value, "edges")? {
        let edge = as_object(edge, "edges[]")?;
        non_empty_string(field(edge, "startNodeId")?, "edge.startNodeId")?;
        non_empty_string(field(edge, "endNodeId")?, "edge.endNodeId")?;
    }
    Ok(())
}

fn validate_and_count_order_actions(root: &Map<String, Value>) -> Result<usize, ObserverError> {
    let mut count = 0usize;
    for list_name in ["nodes", "edges"] {
        for value in as_array(field(root, list_name)?, list_name)? {
            let object = as_object(value, list_name)?;
            let actions = as_array(field(object, "actions")?, "actions")?;
            for action in actions {
                let action = as_object(action, "actions[]")?;
                non_empty_string(field(action, "actionId")?, "action.actionId")?;
                non_empty_string(field(action, "actionType")?, "action.actionType")?;
                let blocking_type =
                    non_empty_string(field(action, "blockingType")?, "action.blockingType")?;
                if !["NONE", "SOFT", "SINGLE", "HARD"].contains(&blocking_type) {
                    return shape("action.blockingType is outside the VDA 5050 v3.0.0 enumeration");
                }
            }
            count = count.saturating_add(actions.len());
        }
    }
    Ok(count)
}

fn parse_unique_json(input: &[u8]) -> Result<Value, ObserverError> {
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    let parsed = StrictValue::deserialize(&mut deserializer).map_err(|error| {
        let message = error.to_string();
        if message.contains("duplicate object key") {
            ObserverError::DuplicateKey(message)
        } else {
            ObserverError::Json(message)
        }
    })?;
    deserializer
        .end()
        .map_err(|error| ObserverError::Json(error.to_string()))?;
    Ok(parsed.0)
}

struct StrictValue(Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictVisitor)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = StrictValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object keys")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Number(Number::from(value))))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Number(Number::from(value))))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let number = Number::from_f64(value).ok_or_else(|| E::custom("non-finite JSON number"))?;
        Ok(StrictValue(Value::Number(number)))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(StrictValue(Value::String(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::String(value)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        StrictValue::deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<StrictValue>()? {
            values.push(value.0);
        }
        Ok(StrictValue(Value::Array(values)))
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut object = Map::new();
        while let Some((key, value)) = access.next_entry::<String, StrictValue>()? {
            if object.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate object key {key:?}")));
            }
            object.insert(key, value.0);
        }
        Ok(StrictValue(Value::Object(object)))
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn looks_like_rfc3339(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 20
        && bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && bytes.get(10) == Some(&b'T')
        && bytes.get(13) == Some(&b':')
        && bytes.get(16) == Some(&b':')
        && bytes
            .get(0..4)
            .is_some_and(|part| part.iter().all(u8::is_ascii_digit))
        && bytes
            .get(5..7)
            .is_some_and(|part| part.iter().all(u8::is_ascii_digit))
        && bytes
            .get(8..10)
            .is_some_and(|part| part.iter().all(u8::is_ascii_digit))
        && bytes
            .get(11..13)
            .is_some_and(|part| part.iter().all(u8::is_ascii_digit))
        && bytes
            .get(14..16)
            .is_some_and(|part| part.iter().all(u8::is_ascii_digit))
        && bytes
            .get(17..19)
            .is_some_and(|part| part.iter().all(u8::is_ascii_digit))
        && (value.ends_with('Z')
            || value.contains('+')
            || value.rfind('-').is_some_and(|index| index > 10))
}

fn as_object<'a>(value: &'a Value, name: &str) -> Result<&'a Map<String, Value>, ObserverError> {
    value
        .as_object()
        .ok_or_else(|| ObserverError::Shape(format!("{name} must be an object")))
}

fn as_array<'a>(value: &'a Value, name: &str) -> Result<&'a [Value], ObserverError> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| ObserverError::Shape(format!("{name} must be an array")))
}

fn field<'a>(object: &'a Map<String, Value>, name: &str) -> Result<&'a Value, ObserverError> {
    object
        .get(name)
        .ok_or_else(|| ObserverError::Shape(format!("missing required field {name}")))
}

fn string<'a>(value: &'a Value, name: &str) -> Result<&'a str, ObserverError> {
    value
        .as_str()
        .ok_or_else(|| ObserverError::Shape(format!("{name} must be a string")))
}

fn non_empty_string<'a>(value: &'a Value, name: &str) -> Result<&'a str, ObserverError> {
    let value = string(value, name)?;
    if value.trim().is_empty() {
        return shape(&format!("{name} must not be empty"));
    }
    Ok(value)
}

fn unsigned(value: &Value, name: &str) -> Result<u64, ObserverError> {
    value
        .as_u64()
        .ok_or_else(|| ObserverError::Shape(format!("{name} must be a non-negative integer")))
}

fn boolean(value: &Value, name: &str) -> Result<bool, ObserverError> {
    value
        .as_bool()
        .ok_or_else(|| ObserverError::Shape(format!("{name} must be a boolean")))
}

fn number(value: &Value, name: &str) -> Result<f64, ObserverError> {
    value
        .as_f64()
        .ok_or_else(|| ObserverError::Shape(format!("{name} must be a number")))
}

fn shape<T>(message: &str) -> Result<T, ObserverError> {
    Err(ObserverError::Shape(message.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = r#""headerId":17,"timestamp":"2026-11-05T10:00:00.000Z","version":"3.0.0","manufacturer":"ACME","serialNumber":"R-01""#;

    fn message(topic: &str, body: &str) -> DecodedMessage {
        let payload = format!("{{{HEADER},{body}}}");
        decode_vda_message(topic, None, payload.as_bytes()).expect("valid VDA message")
    }

    #[test]
    fn order_is_transport_order_not_construction_task() {
        let decoded = message(
            "order",
            r#""orderId":"transport-1","orderUpdateId":2,"nodes":[{"nodeId":"n0","sequenceId":0,"released":true,"actions":[]}],"edges":[{"edgeId":"e0","sequenceId":1,"released":true,"actions":[],"startNodeId":"n0","endNodeId":"n1"}]"#,
        );
        assert_eq!(decoded.payload_sha256.len(), 64);
        assert!(matches!(
            decoded.observation,
            Observation::TransportOrderReceived { order_id, order_update_id: 2, .. }
                if order_id == "transport-1"
        ));
        assert_eq!(
            decoded.summary_json()["observation"]["kind"],
            "transport_order_received"
        );
    }

    #[test]
    fn five_level_topic_identity_must_match_payload_header() {
        let payload = format!(r#"{{{HEADER},"connectionState":"ONLINE"}}"#);
        assert!(matches!(
            decode_vda_message("vda5050/v3/OTHER/R-01/connection", None, payload.as_bytes()),
            Err(ObserverError::TopicIdentityMismatch {
                field: "manufacturer"
            })
        ));
        assert!(
            decode_vda_message("vda5050/v3/ACME/R-01/connection", None, payload.as_bytes()).is_ok()
        );
    }

    #[test]
    fn cloud_topic_hierarchy_is_supported_with_explicit_identity() {
        let payload = format!(r#"{{{HEADER},"connectionState":"ONLINE"}}"#);
        let identity = TopicIdentity {
            manufacturer: "ACME".to_owned(),
            serial_number: "R-01".to_owned(),
        };
        let decoded = decode_vda_message(
            "cloud/tenant/site/vda/connection",
            Some(identity),
            payload.as_bytes(),
        )
        .expect("cloud-style topic");
        assert!(matches!(
            decoded.observation,
            Observation::RobotConnectionStateReported { connection_state } if connection_state == "ONLINE"
        ));
    }

    #[test]
    fn action_finished_is_executor_report_only() {
        let decoded = message(
            "state",
            r#""orderId":"transport-1","orderUpdateId":2,"lastNodeId":"n0","lastNodeSequenceId":0,"nodeStates":[{"nodeId":"n0","sequenceId":0,"released":true}],"edgeStates":[],"driving":false,"actionStates":[{"actionId":"a1","actionStatus":"FINISHED"}],"instantActionStates":[],"powerSupply":{"stateOfCharge":90,"charging":false},"operatingMode":"AUTOMATIC","errors":[],"safetyState":{"activeEmergencyStop":"NONE","fieldViolation":false}"#,
        );
        match &decoded.observation {
            Observation::RobotStateReported {
                action_states,
                safety_state_reported,
                ..
            } => {
                assert_eq!(action_states[0].status, "FINISHED");
                assert!(*safety_state_reported);
            }
            other => panic!("unexpected observation: {other:?}"),
        }
        assert!(!decoded.summary_json().to_string().contains("accepted"));
        assert!(!decoded.summary_json().to_string().contains("authorized"));
    }

    #[test]
    fn zone_grant_is_fleet_traffic_observation_only() {
        let decoded = message(
            "state",
            r#""orderId":"transport-1","orderUpdateId":2,"lastNodeId":"n0","lastNodeSequenceId":0,"nodeStates":[],"edgeStates":[],"driving":false,"actionStates":[],"instantActionStates":[],"powerSupply":{"stateOfCharge":90,"charging":false},"operatingMode":"AUTOMATIC","errors":[],"safetyState":{"activeEmergencyStop":"NONE","fieldViolation":false},"zoneRequests":[{"requestId":"z1","requestType":"ACCESS","zoneId":"zone-1","zoneSetId":"set-1","requestStatus":"GRANTED"}]"#,
        );
        match decoded.observation {
            Observation::RobotStateReported { zone_requests, .. } => {
                assert_eq!(zone_requests[0].request_status, "GRANTED");
            }
            other => panic!("unexpected observation: {other:?}"),
        }
    }

    #[test]
    fn map_version_is_navigation_map_only() {
        let decoded = message(
            "state",
            r#""orderId":"transport-1","orderUpdateId":2,"lastNodeId":"n0","lastNodeSequenceId":0,"nodeStates":[],"edgeStates":[],"driving":false,"actionStates":[],"instantActionStates":[],"powerSupply":{"stateOfCharge":90,"charging":false},"operatingMode":"AUTOMATIC","errors":[],"safetyState":{"activeEmergencyStop":"NONE","fieldViolation":false},"maps":[{"mapId":"floor-2","mapVersion":"map-17"}]"#,
        );
        match decoded.observation {
            Observation::RobotStateReported {
                navigation_maps, ..
            } => {
                assert_eq!(navigation_maps[0].map_version.as_deref(), Some("map-17"));
            }
            other => panic!("unexpected observation: {other:?}"),
        }
        assert!(
            !decoded
                .summary_json()
                .to_string()
                .contains("design_revision")
        );
    }

    #[test]
    fn factsheet_remains_a_reported_claim() {
        let decoded = message(
            "factsheet",
            r#""typeSpecification":{"seriesName":"ACME","mobileRobotKinematics":"DIFFERENTIAL","mobileRobotClass":"CARRIER","maximumLoadMass":100.0,"localizationTypes":["NATURAL"],"navigationTypes":["AUTONOMOUS"]},"physicalParameters":{"minimumSpeed":0.0,"maximumSpeed":1.0,"maximumAcceleration":0.5,"maximumDeceleration":0.5,"minimumHeight":0.2,"maximumHeight":2.0,"width":0.8,"length":1.2},"protocolLimits":{"maximumStringLengths":{},"maximumArrayLengths":{},"timing":{"minimumOrderInterval":0.1,"minimumStateInterval":0.1}},"protocolFeatures":{"optionalParameters":[],"mobileRobotActions":[]},"mobileRobotGeometry":{},"loadSpecification":{}}"#,
        );
        assert!(matches!(
            decoded.observation,
            Observation::FactsheetCapabilityClaimReported {
                type_specification_present: true,
                ..
            }
        ));
        assert!(!decoded.summary_json().to_string().contains("qualified"));
        assert!(!decoded.summary_json().to_string().contains("authenticated"));
    }

    #[test]
    fn zone_set_is_fleet_navigation_data_not_site_safety_permission() {
        let decoded = message(
            "zoneSet",
            r#""zoneSet":{"mapId":"map-1","zoneSetId":"set-1","zones":[{"zoneId":"blocked-1","zoneType":"BLOCKED","vertices":[{"x":0,"y":0},{"x":1,"y":0},{"x":1,"y":1}]}]}"#,
        );
        assert!(matches!(
            decoded.observation,
            Observation::FleetZoneSetReported { ref zones, .. } if zones[0].zone_type == "BLOCKED"
        ));
        assert!(
            !decoded
                .summary_json()
                .to_string()
                .contains("safety_authorized")
        );
    }

    #[test]
    fn connection_topic_reports_protocol_state_only() {
        let decoded = message("connection", r#""connectionState":"CONNECTION_BROKEN""#);
        assert!(matches!(
            decoded.observation,
            Observation::RobotConnectionStateReported { connection_state } if connection_state == "CONNECTION_BROKEN"
        ));
    }

    #[test]
    fn instant_actions_are_requests_not_execution_results() {
        let decoded = message(
            "instantActions",
            r#""actions":[{"actionId":"a1","actionType":"pause","blockingType":"NONE"}]"#,
        );
        assert!(matches!(
            decoded.observation,
            Observation::InstantActionsReported { ref actions }
                if actions.len() == 1 && actions[0].action_type == "pause"
        ));
    }

    #[test]
    fn responses_are_zone_response_observations_not_safety_permits() {
        let decoded = message(
            "responses",
            r#""responses":[{"requestId":"z1","grantType":"GRANTED"}]"#,
        );
        assert!(matches!(
            decoded.observation,
            Observation::FleetZoneResponsesReported { ref responses }
                if responses[0].grant_type == "GRANTED"
        ));
        assert!(!decoded.summary_json().to_string().contains("safety_permit"));
    }

    #[test]
    fn non_v3_payload_is_rejected() {
        let payload =
            format!(r#"{{{HEADER},"connectionState":"ONLINE"}}"#).replace("3.0.0", "2.1.0");
        assert!(matches!(
            decode_vda_message("connection", None, payload.as_bytes()),
            Err(ObserverError::UnsupportedVersion(_))
        ));
    }

    #[test]
    fn topic_major_version_mismatch_is_rejected() {
        let payload = format!(r#"{{{HEADER},"connectionState":"ONLINE"}}"#);
        assert!(matches!(
            decode_vda_message("vda5050/v2/ACME/R-01/connection", None, payload.as_bytes()),
            Err(ObserverError::TopicVersionMismatch(_))
        ));
    }

    #[test]
    fn duplicate_json_keys_are_rejected() {
        let payload =
            format!(r#"{{{HEADER},"connectionState":"ONLINE","connectionState":"OFFLINE"}}"#);
        assert!(matches!(
            decode_vda_message("connection", None, payload.as_bytes()),
            Err(ObserverError::DuplicateKey(_))
        ));
    }

    #[test]
    fn invalid_action_status_is_rejected() {
        let payload = format!(
            r#"{{{HEADER},"orderId":"transport-1","orderUpdateId":2,"lastNodeId":"n0","lastNodeSequenceId":0,"nodeStates":[],"edgeStates":[],"driving":false,"actionStates":[{{"actionId":"a1","actionStatus":"ACCEPTED"}}],"instantActionStates":[],"powerSupply":{{"stateOfCharge":90,"charging":false}},"operatingMode":"AUTOMATIC","errors":[],"safetyState":{{"activeEmergencyStop":"NONE","fieldViolation":false}}}}"#
        );
        assert!(matches!(
            decode_vda_message("state", None, payload.as_bytes()),
            Err(ObserverError::Shape(_))
        ));
    }
    #[test]
    fn unsupported_topic_is_typed() {
        assert!(matches!(
            VdaTopic::parse("visualization"),
            Err(ObserverError::UnsupportedTopic(topic)) if topic == "visualization"
        ));
    }

    #[test]
    fn raw_payload_digest_changes_when_whitespace_changes() {
        let one = format!(r#"{{{HEADER},"connectionState":"ONLINE"}}"#);
        let two = format!(r#"{{ {HEADER},"connectionState":"ONLINE"}}"#);
        let a = decode_vda_message("connection", None, one.as_bytes()).expect("valid");
        let b = decode_vda_message("connection", None, two.as_bytes()).expect("valid");
        assert_ne!(a.payload_sha256, b.payload_sha256);
    }

    #[test]
    fn payload_size_limit_is_enforced() {
        let payload = vec![b' '; MAX_PAYLOAD_BYTES + 1];
        assert!(matches!(
            decode_vda_message("order", None, &payload),
            Err(ObserverError::PayloadTooLarge { .. })
        ));
    }
}
