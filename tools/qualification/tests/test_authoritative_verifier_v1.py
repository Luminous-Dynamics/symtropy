from __future__ import annotations
import ast, hashlib, importlib, json, subprocess, sys, tempfile, unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT))
import qualification_contract_v1 as contract
import rust_workspace_package_profile_v1 as profile
import authoritative_verifier_v1 as verifier

ZERO40='0'*40; ONE40='1'*40; TWO40='2'*40; ZERO64='0'*64; ONE64='1'*64

def base_contract():
    return {'schema_id':contract.SCHEMA_ID,'contract_id':'pilot.v1','contract_version':1,'subject_repository':'Luminous-Dynamics/symtropy','subject_head_sha':ONE40,'subject_tree_sha':TWO40,'manifest_path':'contracts/pilot.manifest.json','manifest_sha256':ZERO64,'suite_profile_path':'contracts/pilot.suite.json','suite_profile_sha256':ONE64}

def base_profile():
    return {'schema_id':profile.SCHEMA_ID,'package':'symtropy-lifesim-core','integration_test':'negative_observation_reference','edition':'2024','test_path':'crates/domains/symtropy-lifesim-core/tests/negative_observation_reference.rs'}

class ParserTests(unittest.TestCase):
    def test_contract_valid(self): self.assertEqual(contract.validate_contract(base_contract())['contract_id'],'pilot.v1')
    def test_contract_foreign_repo_rejected(self):
        x=base_contract(); x['subject_repository']='evil/repo'
        with self.assertRaises(contract.ContractValidationError): contract.validate_contract(x)
    def test_contract_unknown_field_rejected(self):
        x=base_contract(); x['command']='rm -rf /'
        with self.assertRaises(contract.ContractValidationError): contract.validate_contract(x)
    def test_contract_git_path_rejected(self):
        x=base_contract(); x['manifest_path']='foo/.GIT/config'
        with self.assertRaises(contract.ContractValidationError): contract.validate_contract(x)
    def test_contract_duplicate_key_rejected(self):
        raw=b'{"schema_id":"x","schema_id":"y"}'
        with self.assertRaises(contract.ContractValidationError): contract.load_contract_bytes(raw)
    def test_contract_symlink_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            root=Path(td); (root/'real').write_text('x'); (root/'link').symlink_to(root/'real')
            with self.assertRaises(contract.ContractValidationError): contract.resolve_regular_file(root,'link','x')
    def test_profile_valid(self): self.assertEqual(profile.validate_profile(base_profile())['edition'],'2024')
    def test_profile_command_injection_rejected(self):
        x=base_profile(); x['package']='pkg;echo-pwn'
        with self.assertRaises(profile.SuiteProfileValidationError): profile.validate_profile(x)
    def test_profile_path_must_match_test(self):
        x=base_profile(); x['test_path']='tests/other.rs'
        with self.assertRaises(profile.SuiteProfileValidationError): profile.validate_profile(x)
    def test_profile_unknown_field_rejected(self):
        x=base_profile(); x['cargo_args']=['--all']
        with self.assertRaises(profile.SuiteProfileValidationError): profile.validate_profile(x)

class Runner:
    def __init__(self, table): self.table=table; self.calls=[]
    def __call__(self, argv, cwd):
        key=tuple(argv); self.calls.append(key); rc,out=self.table.get(key,(0,'')); return subprocess.CompletedProcess(argv,rc,out,'')

class EngineTests(unittest.TestCase):
    def test_repo_baseline_dirty_verifier_reports_label(self):
        r=Runner({('git','rev-parse','HEAD'):(0,ONE40+'\n'),('git','rev-parse','HEAD^{tree}'):(0,TWO40+'\n'),('git','status','--porcelain=v1','--untracked-files=all'):(0,' M x\n')})
        with self.assertRaisesRegex(verifier.VerificationError,'verifier checkout'):
            verifier._repo_baseline(r,Path('.'),ONE40,'verifier')
    def test_repo_baseline_dirty_contract_reports_label(self):
        r=Runner({('git','rev-parse','HEAD'):(0,ONE40+'\n'),('git','rev-parse','HEAD^{tree}'):(0,TWO40+'\n'),('git','status','--porcelain=v1','--untracked-files=all'):(0,'?? x\n')})
        with self.assertRaisesRegex(verifier.VerificationError,'contract checkout'):
            verifier._repo_baseline(r,Path('.'),ONE40,'contract')
    def test_repo_postflight_head_change_reports_label(self):
        r=Runner({('git','rev-parse','HEAD'):(0,ZERO40+'\n'),('git','rev-parse','HEAD^{tree}'):(0,TWO40+'\n')})
        with self.assertRaisesRegex(verifier.VerificationError,'subject HEAD/tree'):
            verifier._repo_postflight(r,Path('.'),{'head':ONE40,'tree':TWO40},'subject')
    def test_is_covered(self):
        specs=[{'path':'a/b','kind':'prefix'},{'path':'x.rs','kind':'file'}]
        self.assertTrue(verifier._is_covered(specs,'a/b/c')); self.assertTrue(verifier._is_covered(specs,'x.rs')); self.assertFalse(verifier._is_covered(specs,'a'))
    def test_toolchain_wrong_version_fails(self):
        r=Runner({('rustc','--version'):(0,'rustc 1.95.0 x\n'),('cargo','--version'):(0,'cargo 1.96.0 x\n')})
        with tempfile.TemporaryDirectory() as td:
            e=verifier.Evidence(); self.assertFalse(verifier._toolchain_identity(e,r,Path('.'),Path(td))); self.assertEqual(e.first_failing_step_id,'toolchain_identity')
    def test_record_command_failure_records_first_step(self):
        r=Runner({('rustfmt','--check','x'):(1,'bad\n')})
        with tempfile.TemporaryDirectory() as td:
            e=verifier.Evidence(); self.assertFalse(verifier._record_command(e,r,'format',['rustfmt','--check','x'],Path('.'),Path(td))); self.assertEqual(e.first_failing_step_id,'format'); self.assertEqual(e.steps['focused_test'].status,'not_reached')
    def test_evidence_never_has_qualified_pass(self):
        e=verifier.Evidence(final_result='TheoremExecutedPass'); self.assertNotEqual(e.to_json()['final_result'],'QualifiedPass')
    def test_step_order_frozen(self): self.assertEqual(verifier.STEP_IDS,('subject_identity','toolchain_identity','format','focused_test','strict_clippy','locked_check','immutable_postflight'))
    def test_ast_no_shell_true_or_eval_exec(self):
        tree=ast.parse((ROOT/'authoritative_verifier_v1.py').read_text())
        for n in ast.walk(tree):
            if isinstance(n,ast.Call):
                if isinstance(n.func,ast.Name): self.assertNotIn(n.func.id,{'eval','exec'})
                for kw in n.keywords:
                    if kw.arg=='shell': self.assertFalse(isinstance(kw.value,ast.Constant) and kw.value.value is True)
    def test_no_manifest_command_surface(self):
        src=(ROOT/'authoritative_verifier_v1.py').read_text(); self.assertNotIn('cargo_args',src); self.assertNotIn('shell_fragment',src)

if __name__=='__main__': unittest.main()
