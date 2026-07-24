---
title: Pleasure City Metabolism, Labor, and Twenty-Four-Hour Operations Runtime
version: 0.1
status: implementation-spec
scope: nightlife district simulation, visitor demand, resident life, utilities, staffing, housing, transport, sanitation, medical surge, event schedules, and graceful degradation
owner: design/simulation/economy/cities/production
related:
  - ../canon/VICE_ECONOMIES_PLEASURE_CITIES_AND_NIGHTLIFE_CONTRACT_V0_1.md
  - ECONOMIC_LEDGER_MARKET_AND_INTEGRITY_RUNTIME_V0_1.md
  - BODY_HEALTH_TRAUMA_AND_RECOVERY_RUNTIME_V0_1.md
  - VEHICLE_SPACECRAFT_PHYSICS_AND_OPERATIONS_RUNTIME_V0_1.md
  - AUDIO_ACOUSTICS_AND_MUSIC_STATE_RUNTIME_V0_1.md
---

# Pleasure City Metabolism, Labor, and Twenty-Four-Hour Operations Runtime

## Purpose

This runtime makes nightlife a physical and social production system rather than a land-use bonus.

## Core Principle

> **The district that dazzles at midnight must still move workers home, wash the sheets, clear the drains, reconcile the money, and reopen safely.**

# 1. District State

A nightlife district stores bounded state for:

```text
resident population
visitor population by purpose
venue capacity and type
worker roster and qualifications
open shifts and fatigue
housing availability and rent pressure
water, power, cooling, food, and sanitation demand
transport capacity by hour
medical and harm-reduction capacity
noise, light, waste, heat, and crowd burden
license state
reputation by domain
public revenue and service cost
criminal and informal service presence
weather and event conditions
```

No single attractiveness score determines performance.

# 2. Demand Sources

Visitors arrive through causal demand:

- local leisure;
- regional tourism;
- festivals and events;
- conventions and diplomacy;
- sports and racing;
- pilgrimage or ritual;
- medical or identity services;
- celebrity appearances;
- cheap transport windows;
- displacement from prohibited neighboring jurisdictions;
- reputation, rumor, and media.

Demand is limited by:

- travel time and price;
- route safety;
- lodging;
- destination capacity;
- legal restrictions;
- reputation;
- household obligations;
- economic conditions;
- weather and infrastructure.

# 3. Venue Runtime

A venue declares:

```text
venue_id
ownership_model
activity_classes
adult_only_zones
capacity
opening schedule
staff roles
required qualifications
utility profile
accessibility profile
privacy profile
security model
medical and recovery links
payment and custody model
license conditions
worker governance rights
complaint and appeal path
```

A venue may remain open only when minimum safety and staffing conditions are met, unless operators knowingly violate rules and accept the causal risk.

# 4. Shift and Labor Model

Every staffed function consumes person-hours.

Roles may include:

- hosts;
- performers;
- dealers;
- servers;
- cooks;
- cleaners;
- laundry staff;
- stage and sound technicians;
- security and consent monitors;
- transport workers;
- medics and harm-reduction staff;
- financial and custody staff;
- maintenance crews;
- managers and worker representatives.

A shift record includes:

```text
worker_id
role
start and expected end
actual end
breaks
fatigue
hazard exposure
tips, wages, and withheld compensation
transport home
incident participation
consent and privacy scope
```

Automation can reduce labor but must not make cleaning, care, maintenance, or oversight disappear from the economy.

# 5. Worker Fatigue and Retention

Fatigue affects:

- error likelihood;
- patience and conflict;
- injury risk;
- service quality;
- consent-monitoring reliability;
- driving and security performance;
- absenteeism and turnover.

Retention depends on:

- pay;
- schedule control;
- housing;
- safety;
- harassment response;
- transport;
- career paths;
- worker governance;
- social meaning;
- family compatibility.

A profitable district can still enter a labor collapse.

# 6. Utility Profiles

Venue demand is time-varying.

Examples:

- clubs create sharp evening power, cooling, sound, and transport peaks;
- hotels create continuous water, laundry, food, and waste loads;
- baths and spas consume water, heat, cleaning labor, and medical oversight;
- large events create crowd, sanitation, emergency, and route surges;
- adult venues require privacy, laundry, health access, security, and worker transport;
- substance-heavy districts create late medical, hydration, and quiet-space demand.

Utility shortfalls produce specific degradation rather than generic dissatisfaction.

# 7. Housing and Displacement

Nightlife success can increase:

- rent;
- speculative ownership;
- short-stay conversion;
- worker commute distance;
- resident displacement;
- noise conflict;
- informal sleeping arrangements;
- landlord leverage over workers.

Housing is not counted as solved because hotel capacity is high.

Players may respond through:

- worker housing;
- rent rules;
- community land trusts;
- transport investment;
- zoning;
- mixed-use construction;
- visitor levies;
- unrestricted development;
- displacement compensation.

Each choice has tradeoffs and constituency effects.

# 8. Transport Cycle

Transport demand peaks before opening, at major event turnover, and after closing.

The runtime tracks:

- visitor arrivals;
- worker arrivals and safe return;
- accessible vehicles;
- impaired-driver risk;
- pedestrian crowding;
- emergency-route availability;
- late-night fares;
- informal transport;
- weather disruption.

Closing a venue without moving its workers and visitors is not a completed safety action.

# 9. Sanitation and Cleanup

Night operations create:

- food and packaging waste;
- wastewater;
- laundry;
- broken objects;
- bodily-fluid cleanup;
- hazardous sharps or chemicals;
- street litter;
- blocked drains;
- pests;
- odor and air-quality burden.

Cleanup is scheduled, staffed, supplied, and inspected.

Reduced-detail simulation may aggregate waste flows but must preserve cost, capacity, worker exposure, and public-health outcomes.

# 10. Medical Surge and Recovery

The district predicts and responds to:

- intoxication;
- dehydration;
- injury;
- panic and sensory overload;
- medication interruption;
- assault reports;
- chronic-condition exacerbation;
- heat or crowd stress;
- missing persons;
- worker exhaustion.

Capacity includes:

- first aid;
- mobile response;
- quiet rooms;
- testing services;
- transport to clinics;
- continuity medication;
- private reporting;
- language access;
- accessible care.

# 11. Event Scheduling

An event declares:

```text
event_id
organizer
expected attendance
confidence interval
venue set
route plan
staff plan
utility reservation
medical plan
noise and light envelope
resident notification
cleanup plan
cancellation rules
weather contingencies
```

Overbooking shared infrastructure creates explicit conflicts.

# 12. Public Finance

Revenue may include:

- venue taxes;
- hotel and visitor levies;
- licensing fees;
- public venue income;
- transport fares;
- concessions;
- fines;
- land rents.

Costs include:

- sanitation;
- transport;
- medical services;
- regulation;
- public safety;
- housing mitigation;
- infrastructure maintenance;
- cultural grants;
- recovery and treatment.

The ledger must show whether the district is profitable only because costs are shifted to workers, households, neighboring regions, or future maintenance.

# 13. Day-Night Scheduler

The runtime processes four overlapping windows:

```text
setup and delivery
public operation
closing and dispersal
cleanup and recovery
```

Some venues remain continuous. Their staffing and maintenance rotate rather than disappearing.

# 14. Simulation Levels of Detail

## Full Local

Named workers, visitors, incidents, queues, and venue operations.

## District

Aggregated cohorts with named critical participants preserved.

## Regional

Visitor flows, revenue, labor demand, public-health burden, crime pressure, and reputation deltas.

## Long Absence

Periodic summaries preserve:

- ownership changes;
- major incidents;
- worker organization;
- infrastructure debt;
- displacement;
- policy changes;
- cultural seasons;
- named-person continuity.

# 15. Graceful Degradation

When performance budgets are exceeded, degrade in this order:

1. cosmetic crowd variation;
2. individual visitor path detail;
3. noncritical dialogue;
4. venue-interior background animation;
5. aggregated minor transactions.

Never degrade away:

- consent boundaries;
- named-person safety;
- custody;
- utility conservation;
- worker shifts and pay;
- emergency access;
- authoritative incidents;
- privacy.

# 16. Acceptance Tests

Required tests include:

- a venue cannot operate indefinitely without staff or utilities;
- closing time causes transport and cleanup demand;
- tourism growth can raise rent and commute burden;
- public revenue cannot exceed actual transactions and fees;
- understaffing increases specific risk;
- reduced-detail mode preserves material and labor outcomes;
- a one-week absence produces deterministic city-state changes;
- a district can remain culturally vibrant while becoming economically unviable;
- a district can remain profitable while failing resident legitimacy.
