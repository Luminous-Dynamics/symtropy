#!/usr/bin/env python3
"""Exploratory, non-production NetworkX max-flow baseline for SCFK.

This script benchmarks independent NetworkX implementations on seeded
single-commodity directed graphs. It does not invoke or benchmark the Rust
kernel. It is a research baseline only; no production Python service is added.
"""
from __future__ import annotations

import platform
import random
import statistics
import time

import networkx as nx


def make_graph(node_count: int, seed: int, kind: str) -> nx.DiGraph:
    rng = random.Random(seed)
    graph = nx.DiGraph()
    graph.add_nodes_from(range(node_count))

    if kind == "sparse":
        probability = 4 / node_count
        for source in range(node_count):
            for target in range(node_count):
                if source != target and rng.random() < probability:
                    graph.add_edge(source, target, capacity=rng.randint(1, 1000))
    elif kind == "dense":
        probability = 12 / node_count
        for source in range(node_count):
            for target in range(node_count):
                if source != target and rng.random() < probability:
                    graph.add_edge(source, target, capacity=rng.randint(1, 1000))
    elif kind == "layered":
        width = max(4, node_count // 10)
        for layer in range(9):
            start = layer * width
            stop = min((layer + 1) * width, node_count)
            next_start = (layer + 1) * width
            next_stop = min((layer + 2) * width, node_count)
            for source in range(start, stop):
                for target in range(next_start, next_stop):
                    if rng.random() < 0.12:
                        graph.add_edge(source, target, capacity=rng.randint(1, 1000))
    else:
        raise ValueError(f"unsupported graph kind: {kind}")

    # Add a directed serial backbone so every instance has an s-t path.
    for node in range(node_count - 1):
        if not graph.has_edge(node, node + 1):
            graph.add_edge(node, node + 1, capacity=rng.randint(1, 1000))
    return graph


def measure(
    graph: nx.DiGraph,
    algorithm,
    repetitions: int,
    source: int,
    sink: int,
) -> tuple[int, float, float, float]:
    durations: list[float] = []
    values: list[int] = []
    for _ in range(repetitions):
        started = time.perf_counter()
        value = nx.maximum_flow_value(
            graph, source, sink, flow_func=algorithm
        )
        durations.append(time.perf_counter() - started)
        values.append(int(value))

    if len(set(values)) != 1:
        raise AssertionError(f"algorithm returned non-deterministic values: {values}")
    return values[0], statistics.median(durations), min(durations), max(durations)


def main() -> None:
    print(f"python={platform.python_version()} networkx={nx.__version__}")
    cases = [
        (100, "sparse"),
        (400, "sparse"),
        (1000, "sparse"),
        (400, "dense"),
        (1000, "dense"),
        (1000, "layered"),
    ]
    algorithms = [
        ("preflow_push", nx.algorithms.flow.preflow_push, 5),
        ("dinitz", nx.algorithms.flow.dinitz, 5),
        ("edmonds_karp", nx.algorithms.flow.edmonds_karp, 3),
    ]

    for node_count, kind in cases:
        seed = 20_261_126 + node_count + len(kind)
        graph = make_graph(node_count, seed, kind)
        print(
            f"CASE kind={kind} nodes={node_count} edges={graph.number_of_edges()} "
            f"seed={seed}"
        )
        expected_value: int | None = None
        for name, algorithm, repetitions in algorithms:
            value, median, minimum, maximum = measure(
                graph, algorithm, repetitions, 0, node_count - 1
            )
            if expected_value is None:
                expected_value = value
            elif value != expected_value:
                raise AssertionError(
                    f"{name} disagrees with prior solver: {value} != {expected_value}"
                )
            print(
                f"  {name}: flow={value} repetitions={repetitions} "
                f"median_ms={median * 1000:.3f} "
                f"min_ms={minimum * 1000:.3f} "
                f"max_ms={maximum * 1000:.3f}"
            )


if __name__ == "__main__":
    main()
