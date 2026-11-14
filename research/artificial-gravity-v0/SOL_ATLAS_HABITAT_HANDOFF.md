# Sol-Atlas habitat-layout handoff: rotating artificial-gravity environments

**Status:** interface proposal for architectural coordination. Not a crewed habitat design, safety case, or approved spacecraft layout.

## Purpose

Give Sol-Atlas a stable spatial contract for designing rotating habitat arrangements while Symtropy remains the owner of dynamics, structural simulation and numerical validation. Keep geometry and physical qualification separate but connected by immutable configuration identifiers.

## Coordinate and orientation contract

Every habitat model must declare:
- right-handed inertial and rotating coordinate frames, origins and axis directions;
- rotation axis, angular velocity vector, angular acceleration, phase and sign convention;
- radial distance from the axis for every occupied space and major mass;
- the local apparent-acceleration vector and its direction at each occupied zone;
- the transformation between body/habitat coordinates and simulation coordinates.

For a conventional rotating habitat, the rotating-frame centrifugal apparent acceleration points radially outward. The floor/structure supplies an inward normal reaction to keep occupants in circular motion. In architectural orientation terms, the occupied floor's "down" direction is toward larger radius, away from the spin axis.

Do not encode "1 g everywhere". Store the local field and the spatial envelope.

## Sol-Atlas-owned spatial content

Sol-Atlas should own:
- room/deck/compartment geometry and occupancy boundaries;
- pressurized and unpressurized volumes;
- circulation paths, accessibility envelopes, privacy and quiet/rest spaces;
- fire and pressure compartmentation as supplied by qualified subsystem constraints;
- radiation shelter and storm refuge geometry;
- environmental-control, thermal, water, waste, food and power equipment spaces;
- cargo, spares, workshops, medical and maintenance access;
- emergency egress, isolation and rescue/transfer interfaces;
- construction/assembly sequence, robotic access, inspection and replacement paths;
- lifecycle adaptation and community/human-centred spatial requirements.

These are architectural inputs and constraints; their presence in a model does not qualify the associated life-support or safety function.

## Symtropy input/output boundary

### Sol-Atlas → Symtropy geometry manifest

Bind the following to a design revision and digest:
- solid/shell/beam/cable geometry and material region IDs;
- pressure-boundary and attachment surfaces;
- occupied-zone and floor surfaces;
- mass, centre of mass, inertia tensor and equipment/cargo mass distributions;
- joints, bearings, rotary interfaces, tether paths and structural supports;
- load paths, boundary conditions and allowable model simplifications;
- docking/transfer geometry and declared stationary/rotating boundaries.

### Symtropy → Sol-Atlas physical-field feedback

Return:
- local acceleration field with units, frame, origin and model digest;
- gravity-gradient map across each occupied volume;
- motion-dependent Coriolis and Euler acceleration envelopes;
- structural deformation and allowable floor/clearance displacement;
- modal frequencies, vibration response and operational keep-out regions;
- local stress/strain and uncertainty metadata;
- dynamic clearances, bearing/interface loads and identified fault/degraded zones;
- explicit infeasibilities, unresolved conditions and model-validity boundaries.

Do not return one scalar "gravity" field without geometry and provenance.

## Layout research questions

1. Can room and deck arrangements be optimized against the actual gravity-field distribution, rather than a nominal centreline?
2. How do radial, tangential and axial movement corridors affect Coriolis disturbances and task performance?
3. What are the trade-offs between continuous circumferential circulation and local radial spokes/transfer corridors?
4. How can a stationary hub support docking, cargo and services without introducing unsafe or unqualified rotating interfaces?
5. How do shielding, pressure boundaries, fire compartments and life-support plant shift the centre of mass and structural loads?
6. Can modules be isolated, replaced or expanded without invalidating the remaining habitat's mass balance and field envelope?
7. Can spatial architecture preserve privacy, accessibility, community, and equitable access to windows/communal zones without making every layout a single-purpose machine room?

## Design scorecard

- usable, accessible pressurized volume / total pressurized volume;
- useful floor area and occupancy distribution by local g and gradient band;
- path length, turning radius, vertical/radial transitions and emergency egress time;
- maintenance reachability, inspection visibility and replacement clearance;
- radiation refuge capacity and time-to-reach under declared scenarios;
- mass, centre-of-mass and inertia changes by configuration;
- fire/pressure compartment integrity as verified by subsystem owners;
- vibration/noise exposure by occupied zone;
- privacy, quiet/rest access, accessibility and communal amenity coverage;
- design change propagation and model-to-as-built discrepancy.

No overall score should conceal hard safety constraints or local access failures.

## Lifecycle and evidence contract

Each geometry revision binds:
- source CAD/BIM/scene files and exact digest;
- part/module identities and material/procurement records;
- coordinate frames, unit conventions and revision;
- occupancy and mass assumptions;
- structural/physics model version and declared fidelity;
- build/assembly instructions, inspection records and as-built survey;
- deviations, repairs, replacements and requalification decisions.

Mycelix may anchor evidence identities, provenance, attestations and correction history. It does not certify the structure or authorize operations.

## Programme maturity

- **Layout concept:** spatial arrangement only.
- **Geometry-integrated model:** geometry/mass/field are configuration-bound.
- **Simulation-qualified candidate:** numerical and structural model have declared verification and validation evidence in a stated envelope.
- **Subassembly correlated:** physical coupons/joints/subassemblies correlate to predicted behaviour.
- **Uncrewed integrated demonstrator:** contained, inert-payload hardware validates declared performance.
- **Human/flight qualification:** separate specialist, regulatory, mission and ethics authorities; never inferred from the previous stages.

The current workstream is at reference-kinematics and research-protocol maturity. No habitat structure or human-use configuration is qualified.
