    if classified_checks != all_checks:
        _fail("relationship checks are not exactly covered by semantic bindings")

    declared_graph_fields = {
        (operand[1], operand[2])
        for spec in GRAPH_RELATIONSHIP_CONTRACTS
        for pair in spec["checks"]
        for operand in pair
        if operand[0] == "graph"
    }
    # Graph operands are properties of individual relationship checks, not
    # of a binding's source-operand kind. A binding may legitimately compare
    # its retained value operand against a graph projection (for example,
    # contract_commit_sha and subject_head_sha), so using
    # relationship_operand_kind == "graph" here would incorrectly erase those
    # graph fields from the canonical relationship catalog.
    required_graph_fields = {
        (operand[1], operand[2])
        for spec in GRAPH_RELATIONSHIP_CONTRACTS
        for pair in spec["checks"]
        for operand in pair
        if operand[0] == "graph"
    }
    if declared_graph_fields != required_graph_fields:
        _fail("relationship contract graph-field coverage is not canonical")

    for binding, source in SEMANTIC_BINDING_SOURCES.items():
        target, field = source