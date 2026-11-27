#!/usr/bin/env python3
"""Tiny exhaustive oracle for SAFRB's sequential lexicographic MCF encoding.

Research-only reference. Production planning and certificate generation remain
Rust-first. This script compares NetworkX MCF implementations with exhaustive
integral-flow enumeration; it does not import or exercise the Rust planner.
"""

from __future__ import annotations

import itertools
import random
import statistics
import time
from typing import Dict, Iterable, List, Mapping, Sequence, Tuple

import networkx as nx

Node = str
Arc = Dict[str, object]
Objective = Tuple[int, ...]


def exhaustive_oracle(
    nodes: Sequence[Node],
    arcs: Sequence[Arc],
    source: Node,
    sink: Node,
    service_edges: Sequence[str],
) -> Tuple[Objective, Tuple[int, ...]]:
    """Enumerate every integral arc flow and return the best lexicographic tuple."""
    best_objective: Objective | None = None
    best_flow: Tuple[int, ...] | None = None
    ranges = [range(int(arc["capacity"]) + 1) for arc in arcs]

    for values in itertools.product(*ranges):
        balance = {node: 0 for node in nodes}  # outflow minus inflow
        for arc, flow in zip(arcs, values):
            balance[str(arc["tail"])] += flow
            balance[str(arc["head"])] -= flow

        if any(balance[node] != 0 for node in nodes if node not in (source, sink)):
            continue
        if balance[source] != -balance[sink] or balance[source] < 0:
            continue

        total_flow = balance[source]
        service = tuple(
            values[next(i for i, arc in enumerate(arcs) if arc["id"] == edge_id)]
            for edge_id in service_edges
        )
        route_cost = sum(
            values[i] * int(arc["unit_cost"])
            for i, arc in enumerate(arcs)
            if str(arc["id"]) not in service_edges
        )
        objective = (-total_flow, *(-amount for amount in service), route_cost)

        if best_objective is None or objective < best_objective:
            best_objective = objective
            best_flow = tuple(values)

    if best_objective is None or best_flow is None:
        raise RuntimeError("oracle found no feasible integral flow")
    return best_objective, best_flow


def solve_fixed_flow_mcf(
    nodes: Sequence[Node],
    arcs: Sequence[Arc],
    source: Node,
    sink: Node,
    fixed_total: int,
    frozen_lower_bounds: Mapping[str, int],
    stage_costs: Mapping[str, int],
    implementation: str,
) -> Dict[str, int]:
    """Solve one MCF pass with exact lower-bound-to-node-balance conversion.

    NetworkX demand convention is net inflow minus outflow. For a lower bound
    l on tail -> head, the residual problem therefore increments tail demand
    by l and decrements head demand by l.
    """
    if implementation not in ("capacity_scaling", "network_simplex"):
        raise ValueError(f"unsupported implementation: {implementation}")

    demand = {node: 0 for node in nodes}
    demand[source] = -fixed_total
    demand[sink] = fixed_total

    graph = (
        nx.MultiDiGraph()
        if implementation == "capacity_scaling"
        else nx.DiGraph()
    )
    graph.add_nodes_from(nodes)

    for arc in arcs:
        edge_id = str(arc["id"])
        tail = str(arc["tail"])
        head = str(arc["head"])
        capacity = int(arc["capacity"])
        lower = int(frozen_lower_bounds.get(edge_id, 0))
        upper = lower if edge_id in frozen_lower_bounds else capacity
        if lower < 0 or upper < lower or upper > capacity:
            raise ValueError(f"invalid frozen bound for {edge_id}")

        demand[tail] += lower
        demand[head] -= lower
        residual_capacity = upper - lower
        unit_cost = int(stage_costs.get(edge_id, 0))

        if implementation == "capacity_scaling":
            graph.add_edge(
                tail,
                head,
                key=edge_id,
                capacity=residual_capacity,
                weight=unit_cost,
            )
        else:
            if graph.has_edge(tail, head):
                raise ValueError(
                    "network_simplex experiment requires an identity-preserving "
                    "simple directed graph; parallel arcs must use a separate "
                    "edge-node transformation lane"
                )
            graph.add_edge(
                tail,
                head,
                capacity=residual_capacity,
                weight=unit_cost,
            )

    nx.set_node_attributes(graph, demand, "demand")

    if implementation == "capacity_scaling":
        _, flow_dict = nx.capacity_scaling(graph)
        return {
            str(arc["id"]): int(frozen_lower_bounds.get(str(arc["id"]), 0))
            + int(
                flow_dict[str(arc["tail"])]
                .get(str(arc["head"]), {})
                .get(str(arc["id"]), 0)
            )
            for arc in arcs
        }

    _, flow_dict = nx.network_simplex(graph)
    return {
        str(arc["id"]): int(frozen_lower_bounds.get(str(arc["id"]), 0))
        + int(
            flow_dict[str(arc["tail"])].get(str(arc["head"]), 0)
        )
        for arc in arcs
    }


def verify_flow(
    nodes: Sequence[Node],
    arcs: Sequence[Arc],
    source: Node,
    sink: Node,
    expected_total: int,
    flow: Mapping[str, int],
    lower_bounds: Mapping[str, int],
) -> None:
    """Independently check arc bounds, frozen lower bounds and conservation."""
    expected_ids = {str(arc["id"]) for arc in arcs}
    if set(flow) != expected_ids:
        raise AssertionError("flow witness does not cover every original arc")

    balance = {node: 0 for node in nodes}  # outflow minus inflow
    for arc in arcs:
        edge_id = str(arc["id"])
        amount = int(flow[edge_id])
        capacity = int(arc["capacity"])
        lower = int(lower_bounds.get(edge_id, 0))
        if amount < lower or amount > capacity:
            raise AssertionError(f"arc bounds violated for {edge_id}")
        balance[str(arc["tail"])] += amount
        balance[str(arc["head"])] -= amount

    if balance[source] != expected_total or balance[sink] != -expected_total:
        raise AssertionError("source/sink flow value does not match the fixed total")
    if any(
        balance[node] != 0
        for node in nodes
        if node not in (source, sink)
    ):
        raise AssertionError("flow conservation failed at an intermediate node")


def sequential_lexicographic_mcf(
    nodes: Sequence[Node],
    arcs: Sequence[Arc],
    source: Node,
    sink: Node,
    service_edges: Sequence[str],
    implementation: str,
) -> Objective:
    """Maximise flow, then each service tier, then minimise route cost."""
    maximum_graph = nx.DiGraph()
    maximum_graph.add_nodes_from(nodes)
    for arc in arcs:
        tail = str(arc["tail"])
        head = str(arc["head"])
        capacity = int(arc["capacity"])
        existing = maximum_graph.get_edge_data(tail, head, {}).get("capacity", 0)
        maximum_graph.add_edge(tail, head, capacity=existing + capacity)

    maximum_flow = int(
        nx.maximum_flow_value(maximum_graph, source, sink, capacity="capacity")
    )

    frozen: Dict[str, int] = {}
    for service_edge in service_edges:
        tier_flow = solve_fixed_flow_mcf(
            nodes,
            arcs,
            source,
            sink,
            maximum_flow,
            frozen,
            {service_edge: -1},
            implementation,
        )
        verify_flow(
            nodes, arcs, source, sink, maximum_flow, tier_flow, frozen
        )
        frozen[service_edge] = tier_flow[service_edge]

    route_costs = {
        str(arc["id"]): int(arc["unit_cost"])
        for arc in arcs
        if str(arc["id"]) not in service_edges
    }
    final_flow = solve_fixed_flow_mcf(
        nodes,
        arcs,
        source,
        sink,
        maximum_flow,
        frozen,
        route_costs,
        implementation,
    )
    verify_flow(nodes, arcs, source, sink, maximum_flow, final_flow, frozen)
    if any(final_flow[edge_id] != amount for edge_id, amount in frozen.items()):
        raise AssertionError("a supposedly frozen priority tier changed in the final pass")

    service_totals = tuple(final_flow[edge_id] for edge_id in service_edges)
    total_cost = sum(
        final_flow[str(arc["id"])] * int(arc["unit_cost"])
        for arc in arcs
        if str(arc["id"]) not in service_edges
    )
    return (-maximum_flow, *(-amount for amount in service_totals), total_cost)


def make_instance(seed: int) -> Tuple[List[Node], List[Arc]]:
    """Create a small acyclic two-tier network with bounded integer capacities."""
    rng = random.Random(0x5AFB0000 + seed)
    nodes = ["s", "a", "b", "p0", "p1", "t"]
    topology = [
        ("e0", "s", "a"),
        ("e1", "s", "b"),
        ("e2", "a", "p0"),
        ("e3", "b", "p0"),
        ("e4", "a", "p1"),
        ("e5", "b", "p1"),
        ("d0", "p0", "t"),
        ("d1", "p1", "t"),
    ]
    arcs: List[Arc] = []
    for edge_id, tail, head in topology:
        capacity = rng.randint(1, 2)
        unit_cost = 0 if edge_id.startswith("d") else rng.randint(0, 7)
        arcs.append(
            {
                "id": edge_id,
                "tail": tail,
                "head": head,
                "capacity": capacity,
                "unit_cost": unit_cost,
            }
        )
    return nodes, arcs


def percentile(values: Sequence[float], quantile: float) -> float:
    ordered = sorted(values)
    index = min(len(ordered) - 1, int((len(ordered) - 1) * quantile))
    return ordered[index] * 1000.0


def main() -> None:
    seeds = 100
    source, sink = "s", "t"
    service_edges = ("d0", "d1")
    implementations = ("capacity_scaling", "network_simplex")
    timings = {name: [] for name in implementations}

    for seed in range(seeds):
        nodes, arcs = make_instance(seed)
        expected, _ = exhaustive_oracle(
            nodes, arcs, source, sink, service_edges
        )

        for implementation in implementations:
            start = time.perf_counter()
            observed = sequential_lexicographic_mcf(
                nodes,
                arcs,
                source,
                sink,
                service_edges,
                implementation,
            )
            timings[implementation].append(time.perf_counter() - start)
            if observed != expected:
                raise AssertionError(
                    f"seed={seed} implementation={implementation} "
                    f"expected={expected} observed={observed}"
                )

    print(f"networkx_version={nx.__version__}")
    print(f"independent_tiny_instances={seeds}")
    for implementation, durations in timings.items():
        print(
            f"{implementation}: parity={seeds}/{seeds}, "
            f"median_ms={statistics.median(durations) * 1000.0:.3f}, "
            f"p95_ms={percentile(durations, 0.95):.3f}, "
            f"p99_ms={percentile(durations, 0.99):.3f}"
        )
    print(
        "scope=6_nodes,8_arcs,2_priority_tiers,integer_capacity_1_to_2,"
        "exhaustive_integral_flow_oracle,sequential_lower_bound_mcf"
    )
    print(
        "claim_ceiling=objective_encoding_correctness_only;"
        "not_rust_benchmark_or_field_qualification"
    )


if __name__ == "__main__":
    main()
