

## Authority handoff v0

The authority handoff is a deliberately smaller boundary after admission. It validates the complete admission envelope again and emits only the declarative identity needed by the existing QUAL-001B authority:

- admission digest;
- expected result;
- exact subject repository/head/tree;
- exact qualification-contract identity;
- exact verifier-release identity;
- exact proof-execution identity.

It emits no shell command, Cargo argument, workflow selection, or executable policy. It does not decide whether the verifier release is approved and cannot produce QualifiedPass or QualifiedFail.

Run:

```text
python3 formal/provenance/handoff.py <graph.json> <impact.json> <admission.json>
```

The handoff is therefore a transport/admission boundary, not a second authority system. Approval and qualification remain owned by the existing default-branch verifier.
