package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"webpki.org/jsoncanonicalizer"
)

func TestGeneratedCorpusVerifierChecksValidAndRejectPanels(t *testing.T) {
	root := t.TempDir()
	if err := os.MkdirAll(filepath.Join(root, "valid"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(filepath.Join(root, "expected"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(filepath.Join(root, "reject"), 0o755); err != nil {
		t.Fatal(err)
	}

	validInput := []byte("{\"a\":1,\"b\":[true,null]}")
	expected, err := jsoncanonicalizer.Transform(validInput)
	if err != nil {
		t.Fatalf("canonicalize valid test case: %v", err)
	}
	if err := os.WriteFile(filepath.Join(root, "valid", "valid.json"), validInput, 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, "expected", "valid.jcs"), expected, 0o644); err != nil {
		t.Fatal(err)
	}

	rejectInput := []byte("{\"duplicate\":1,\"duplicate\":2}")
	if err := os.WriteFile(filepath.Join(root, "reject", "duplicate.bin"), rejectInput, 0o644); err != nil {
		t.Fatal(err)
	}

	manifest := corpusManifest{
		SchemaVersion:           expectedCorpusSchema,
		GeneratorID:             "engineering-scenario-jcs-adversarial-corpus",
		GeneratorVersion:        "0.1.0",
		CanonicalizationProfile: expectedCorpusProfile,
		DeterministicSeedHex:    "4a43535f434f5250",
		GeneratedCaseCount:      1,
		ValidCaseCount:          1,
		RejectCaseCount:         1,
		ValidCases: []validCorpusCase{{
			CaseID:                  "valid-object",
			InputFile:               "valid/valid.json",
			InputSHA256:             digestHex(validInput),
			ExpectedCanonicalFile:   "expected/valid.jcs",
			ExpectedCanonicalSHA256: digestHex(expected),
			ExpectedCanonicalBytes:  len(expected),
		}},
		RejectCases: []rejectCorpusCase{{
			CaseID:                 "duplicate-object-member",
			InputFile:              "reject/duplicate.bin",
			InputSHA256:            digestHex(rejectInput),
			ExpectedRejectionClass: "duplicate_member_name",
		}},
		ClaimCeiling: "test-only",
	}
	manifestBytes, err := json.Marshal(manifest)
	if err != nil {
		t.Fatalf("serialize test manifest: %v", err)
	}
	if err := os.WriteFile(filepath.Join(root, "manifest.json"), manifestBytes, 0o644); err != nil {
		t.Fatal(err)
	}

	resultsPath := filepath.Join(root, "results.jsonl")
	summaryPath := filepath.Join(root, "summary.json")
	summary, err := verifyGeneratedCorpus(root, resultsPath, summaryPath)
	if err != nil {
		t.Fatalf("verify generated fixture corpus: %v", err)
	}
	if summary.ValidCanonicalMatches != 1 ||
		summary.RejectCasesCorrectlyDenied != 1 ||
		summary.FailureCount != 0 ||
		summary.CaseIDCount != 2 {
		t.Fatalf("unexpected corpus summary: %+v", summary)
	}
	resultsBytes, err := os.ReadFile(resultsPath)
	if err != nil {
		t.Fatalf("read result sidecar: %v", err)
	}
	resultLines := strings.Split(strings.TrimSpace(string(resultsBytes)), "\n")
	if len(resultLines) != 2 {
		t.Fatalf("expected two result rows, got %d", len(resultLines))
	}
	var validResult, rejectResult corpusResult
	if err := json.Unmarshal([]byte(resultLines[0]), &validResult); err != nil {
		t.Fatalf("parse valid result row: %v", err)
	}
	if err := json.Unmarshal([]byte(resultLines[1]), &rejectResult); err != nil {
		t.Fatalf("parse reject result row: %v", err)
	}
	if validResult.ActualClassification != "valid_match" {
		t.Fatalf("valid row classification was %q", validResult.ActualClassification)
	}
	if rejectResult.ActualClassification != "rejected_before_canonicalization" ||
		rejectResult.ExpectedRejectionClass == nil ||
		*rejectResult.ExpectedRejectionClass != "duplicate_member_name" ||
		rejectResult.ActualRejectionStage == nil ||
		*rejectResult.ActualRejectionStage != "strict_raw_json_preflight" {
		t.Fatalf("reject row did not separate expected class from observed stage: %+v", rejectResult)
	}
}

func TestCorpusFileRejectsPathTraversal(t *testing.T) {
	root := t.TempDir()
	if _, err := corpusFile(root, "../outside.json"); err == nil {
		t.Fatal("expected parent traversal to be rejected")
	}
	if _, err := corpusFile(root, filepath.Join(root, "absolute.json")); err == nil {
		t.Fatal("expected absolute paths to be rejected")
	}
}
