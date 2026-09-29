import copy
import pathlib
import sys
import unittest
ROOT = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))
from admission import build_admission, digest, validate_admission  # noqa: E402
from impact import analyze  # noqa: E402
SCHEMA = "luminous.formal-provenance.v0"
A="a"*64; B="b"*64; C="c"*64; D="d"*64; E="e"*64; F="f"*64

def node(kind, node_id, label, **extra):
    value={"schema_version":SCHEMA,"kind":kind,"id":node_id,"label":label}; value.update(extra); return value

def valid_graph():
    return {"schema_version":SCHEMA,"nodes":[
        node("Theorem","theorem:t","Theorem",metadata={"semantic_digest":A}),
        node("ProofArtifact","proof:p","Proof",content_digest=B,metadata={"semantic_target_digest":A}),
        node("ProofChecker","checker:c","Checker",content_digest=C),
        node("SourceCommit","source:commit","Source commit",content_digest=D,provenance={"repository":"Luminous-Dynamics/symtropy","commit_sha":D,"tree_sha":E}),
        node("SourceTree","source:tree","Source tree",content_digest=E,provenance={"repository":"Luminous-Dynamics/symtropy","commit_sha":D,"tree_sha":E}),
        node("ProofExecution","execution:x","Execution",content_digest=F,metadata={"artifact_id":"proof:p","checker_id":"checker:c","source_commit_id":"source:commit","source_tree_id":"source:tree","source_commit_sha":D,"source_tree_sha":E,"subject_head":F,"subject_tree":E,"status":"Passed","result":"Pass"}),
        node("QualificationContract","contract:q","Contract",content_digest=A),
        node("VerifierRelease","verifier:v","Verifier",content_digest=B),
        node("QualificationEvidence","evidence:q","Evidence",content_digest=C,metadata={"contract_id":"contract:q","contract_digest":A,"verifier_release_id":"verifier:v","verifier_release_digest":B,"subject_head":F,"subject_tree":E,"execution_id":"execution:x","result":"Pass"}),
    ],"edges":[
        {"schema_version":SCHEMA,"id":"edge:proves","source":"proof:p","target":"theorem:t","relation":"proves"},
        {"schema_version":SCHEMA,"id":"edge:checked","source":"proof:p","target":"checker:c","relation":"checked_by"},
    ]}

class AdmissionTests(unittest.TestCase):
    def make(self):
        graph=valid_graph(); impact=analyze(graph,["execution:x"])
        return graph, impact, build_admission(graph,impact,subject_repository="Luminous-Dynamics/symtropy",expected_result="QualifiedPass")
    def test_round_trip_and_canonical_digest(self):
        graph,impact,envelope=self.make()
        self.assertEqual(validate_admission(graph,impact,envelope),envelope)
        unsigned={k:envelope[k] for k in envelope if k!="admission_digest"}
        self.assertEqual(envelope["admission_digest"],digest(unsigned))
        reordered=copy.deepcopy(envelope)
        reordered["bindings"]=list(reversed(reordered["bindings"]))
        reordered["admission_digest"]=digest({k:reordered[k] for k in reordered if k!="admission_digest"})
        self.assertEqual(validate_admission(graph,impact,reordered),envelope)
    def test_tampered_frontier_is_rejected(self):
        graph,impact,envelope=self.make()
        envelope["impact"]["changed_node_ids"]=["proof:p"]
        envelope["admission_digest"]=digest({k:envelope[k] for k in envelope if k!="admission_digest"})
        with self.assertRaises(ValueError): validate_admission(graph,impact,envelope)
    def test_tampered_binding_is_rejected(self):
        graph,impact,envelope=self.make()
        envelope["bindings"][0]["identity_digest"]="9"*64
        envelope["admission_digest"]=digest({k:envelope[k] for k in envelope if k!="admission_digest"})
        with self.assertRaises(ValueError): validate_admission(graph,impact,envelope)
    def test_evidence_contract_and_verifier_digests_are_required(self):
        graph, impact, envelope = self.make()
        graph["nodes"][-1]["metadata"]["contract_digest"] = "9" * 64
        with self.assertRaises(ValueError):
            build_admission(graph, impact, subject_repository="Luminous-Dynamics/symtropy", expected_result="QualifiedPass")

    def test_evidence_result_must_match_expected_result(self):
        graph, impact, _ = self.make()
        graph["nodes"][-1]["metadata"]["result"] = "Fail"
        with self.assertRaises(ValueError):
            build_admission(graph, impact, subject_repository="Luminous-Dynamics/symtropy", expected_result="QualifiedPass")

    def test_exactly_one_evidence_binding_is_required(self):
        graph, impact, _ = self.make()
        graph["nodes"].append(node("QualificationEvidence", "evidence:extra", "Extra", content_digest=A, metadata={
            "contract_id": "contract:q", "contract_digest": A,
            "verifier_release_id": "verifier:v", "verifier_release_digest": B,
            "subject_head": F, "subject_tree": E,
            "execution_id": "execution:x", "result": "Pass",
        }))
        with self.assertRaises(ValueError):
            build_admission(graph, impact, subject_repository="Luminous-Dynamics/symtropy", expected_result="QualifiedPass")

    def test_empty_changed_roots_fail_closed(self):
        graph=valid_graph()
        with self.assertRaises(ValueError):
            build_admission(graph,{"schema_version":SCHEMA,"impact_version":"luminous.formal-impact.v0","impact_status":"NoImpact","changed_node_ids":[],"records":[]},subject_repository="Luminous-Dynamics/symtropy",expected_result="QualifiedPass")

if __name__=="__main__": unittest.main()
