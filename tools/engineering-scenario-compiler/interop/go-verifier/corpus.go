package main

import (
	"bufio"
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"webpki.org/jsoncanonicalizer"
)

const expectedCorpusSchema = "engineering-scenario-jcs-corpus-v0.1"
const expectedCorpusProfile = "engineering-scenario-ir-jcs-rfc8785-v1"

type corpusManifest struct {
	SchemaVersion             string              `json:"schema_version"`
	GeneratorID               string              `json:"generator_id"`
	GeneratorVersion          string              `json:"generator_version"`
	CanonicalizationProfile   string              `json:"canonicalization_profile"`
	DeterministicSeedHex      string              `json:"deterministic_seed_hex"`
	GeneratedCaseCount        int                 `json:"generated_case_count"`
	ValidCaseCount            int                 `json:"valid_case_count"`
	RejectCaseCount           int                 `json:"reject_case_count"`
	ValidCases                []validCorpusCase  `json:"valid_cases"`
	RejectCases               []rejectCorpusCase `json:"reject_cases"`
	ClaimCeiling              string              `json:"claim_ceiling"`
}

type validCorpusCase struct {
	CaseID                  string  `json:"case_id"`
	InputFile               string  `json:"input_file"`
	InputSHA256             string  `json:"input_sha256"`
	ExpectedCanonicalFile   string  `json:"expected_canonical_file"`
	ExpectedCanonicalSHA256 string  `json:"expected_canonical_sha256"`
	ExpectedCanonicalBytes  int     `json:"expected_canonical_bytes"`
	EquivalenceGroup        *string `json:"equivalence_group"`
}

type rejectCorpusCase struct {
	CaseID                  string `json:"case_id"`
	InputFile               string `json:"input_file"`
	InputSHA256             string `json:"input_sha256"`
	ExpectedRejectionClass  string `json:"expected_rejection_class"`
}

type corpusResult struct {
	CaseID                  string  `json:"case_id"`
	ExpectedClassification  string  `json:"expected_classification"`
	ActualClassification    string  `json:"actual_classification"`
	InputSHA256             string  `json:"input_sha256"`
	ExpectedCanonicalSHA256 *string `json:"expected_canonical_sha256,omitempty"`
	ActualCanonicalSHA256   *string `json:"actual_canonical_sha256,omitempty"`
	CanonicalBytes          *int    `json:"canonical_bytes,omitempty"`
	ExpectedRejectionClass  *string `json:"expected_rejection_class,omitempty"`
	ActualRejectionStage    *string `json:"actual_rejection_stage,omitempty"`
	Detail                  *string `json:"detail,omitempty"`
}

type corpusSummary struct {
	SchemaVersion              string `json:"schema_version"`
	GeneratorID                string `json:"generator_id"`
	GeneratorVersion           string `json:"generator_version"`
	CanonicalizationProfile    string `json:"canonicalization_profile"`
	DeterministicSeedHex       string `json:"deterministic_seed_hex"`
	CorpusManifestSHA256       string `json:"corpus_manifest_sha256"`
	ValidCaseCount             int    `json:"valid_case_count"`
	RejectCaseCount            int    `json:"reject_case_count"`
	ValidCanonicalMatches      int    `json:"valid_canonical_matches"`
	ValidCanonicalMismatches   int    `json:"valid_canonical_mismatches"`
	ValidStrictRejects         int    `json:"valid_strict_rejects"`
	RejectStrictAccepts        int    `json:"reject_strict_accepts"`
	RejectCasesCorrectlyDenied int    `json:"reject_cases_correctly_denied"`
	CaseIDCount                int    `json:"case_id_count"`
	FailureCount               int    `json:"failure_count"`
	ClaimCeiling               string `json:"claim_ceiling"`
}

func digestHex(input []byte) string {
	digest := sha256.Sum256(input)
	return hex.EncodeToString(digest[:])
}

func corpusFile(root, relative string) (string, error) {
	if relative == "" || filepath.IsAbs(relative) {
		return "", fmt.Errorf("corpus file path must be non-empty and relative: %q", relative)
	}
	clean := filepath.Clean(relative)
	if clean == "." || clean == ".." || strings.HasPrefix(clean, ".."+string(os.PathSeparator)) {
		return "", fmt.Errorf("corpus file path escapes root: %q", relative)
	}
	rootAbsolute, err := filepath.Abs(root)
	if err != nil {
		return "", fmt.Errorf("resolve corpus root: %w", err)
	}
	fileAbsolute, err := filepath.Abs(filepath.Join(rootAbsolute, clean))
	if err != nil {
		return "", fmt.Errorf("resolve corpus path %q: %w", relative, err)
	}
	rel, err := filepath.Rel(rootAbsolute, fileAbsolute)
	if err != nil {
		return "", fmt.Errorf("check corpus path %q: %w", relative, err)
	}
	if rel == ".." || strings.HasPrefix(rel, ".."+string(os.PathSeparator)) {
		return "", fmt.Errorf("corpus file path escapes root: %q", relative)
	}
	return fileAbsolute, nil
}

func readCorpusFile(root, relative, expectedSHA string) ([]byte, error) {
	path, err := corpusFile(root, relative)
	if err != nil {
		return nil, err
	}
	input, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read corpus file %q: %w", relative, err)
	}
	actualSHA := digestHex(input)
	if actualSHA != expectedSHA {
		return nil, fmt.Errorf(
			"corpus file %q SHA-256 mismatch: expected %s, got %s",
			relative,
			expectedSHA,
			actualSHA,
		)
	}
	return input, nil
}

func writeCorpusResult(writer *bufio.Writer, result corpusResult) error {
	encoded, err := json.Marshal(result)
	if err != nil {
		return fmt.Errorf("serialize corpus result %q: %w", result.CaseID, err)
	}
	if _, err := writer.Write(encoded); err != nil {
		return fmt.Errorf("write corpus result %q: %w", result.CaseID, err)
	}
	if err := writer.WriteByte('\n'); err != nil {
		return fmt.Errorf("terminate corpus result %q: %w", result.CaseID, err)
	}
	return nil
}

// verifyGeneratedCorpus applies strict raw-input validation before invoking
// the independent Go canonicalizer. Expected-reject cases never reach JCS.
func verifyGeneratedCorpus(root, resultPath, summaryPath string) (corpusSummary, error) {
	manifestPath := filepath.Join(root, "manifest.json")
	manifestBytes, err := os.ReadFile(manifestPath)
	if err != nil {
		return corpusSummary{}, fmt.Errorf("read generated corpus manifest: %w", err)
	}
	var manifest corpusManifest
	if err := json.Unmarshal(manifestBytes, &manifest); err != nil {
		return corpusSummary{}, fmt.Errorf("parse generated corpus manifest: %w", err)
	}
	if manifest.SchemaVersion != expectedCorpusSchema {
		return corpusSummary{}, fmt.Errorf("unexpected corpus schema %q", manifest.SchemaVersion)
	}
	if manifest.CanonicalizationProfile != expectedCorpusProfile {
		return corpusSummary{}, fmt.Errorf("unexpected JCS profile %q", manifest.CanonicalizationProfile)
	}
	if manifest.ValidCaseCount != len(manifest.ValidCases) ||
		manifest.RejectCaseCount != len(manifest.RejectCases) ||
		manifest.GeneratedCaseCount <= 0 {
		return corpusSummary{}, fmt.Errorf("corpus manifest counts do not match its case arrays")
	}

	summary := corpusSummary{
		SchemaVersion:           manifest.SchemaVersion,
		GeneratorID:             manifest.GeneratorID,
		GeneratorVersion:        manifest.GeneratorVersion,
		CanonicalizationProfile: manifest.CanonicalizationProfile,
		DeterministicSeedHex:    manifest.DeterministicSeedHex,
		CorpusManifestSHA256:    digestHex(manifestBytes),
		ValidCaseCount:          manifest.ValidCaseCount,
		RejectCaseCount:         manifest.RejectCaseCount,
		ClaimCeiling:            "cross-language JCS byte agreement and strict rejection for this frozen generated corpus only",
	}
	if manifest.GeneratorID != "engineering-scenario-jcs-adversarial-corpus" ||
		manifest.GeneratorVersion != "0.1.0" ||
		manifest.DeterministicSeedHex == "" {
		return summary, fmt.Errorf("corpus generator identity or seed is not recognized")
	}

	if err := os.MkdirAll(filepath.Dir(resultPath), 0o755); err != nil {
		return summary, fmt.Errorf("create result directory: %w", err)
	}
	if err := os.MkdirAll(filepath.Dir(summaryPath), 0o755); err != nil {
		return summary, fmt.Errorf("create summary directory: %w", err)
	}
	resultsFile, err := os.Create(resultPath)
	if err != nil {
		return summary, fmt.Errorf("create corpus results JSONL: %w", err)
	}
	defer resultsFile.Close()
	writer := bufio.NewWriter(resultsFile)
	seen := make(map[string]struct{}, manifest.ValidCaseCount+manifest.RejectCaseCount)

	for _, testCase := range manifest.ValidCases {
		if _, exists := seen[testCase.CaseID]; exists || testCase.CaseID == "" {
			return summary, fmt.Errorf("duplicate or empty corpus case ID %q", testCase.CaseID)
		}
		seen[testCase.CaseID] = struct{}{}
		input, err := readCorpusFile(root, testCase.InputFile, testCase.InputSHA256)
		if err != nil {
			return summary, err
		}
		expected, err := readCorpusFile(root, testCase.ExpectedCanonicalFile, testCase.ExpectedCanonicalSHA256)
		if err != nil {
			return summary, err
		}
		if len(expected) != testCase.ExpectedCanonicalBytes {
			return summary, fmt.Errorf("expected canonical byte length mismatch for %q", testCase.CaseID)
		}
		if err := validateStrictJCSInput(input); err != nil {
			summary.ValidStrictRejects++
			detail := err.Error()
			if writeErr := writeCorpusResult(writer, corpusResult{
				CaseID:                 testCase.CaseID,
				ExpectedClassification: "valid",
				ActualClassification:   "strict_rejected",
				InputSHA256:            testCase.InputSHA256,
				ExpectedCanonicalSHA256: &testCase.ExpectedCanonicalSHA256,
				Detail:                 &detail,
			}); writeErr != nil {
				return summary, writeErr
			}
			continue
		}
		actual, err := jsoncanonicalizer.Transform(input)
		if err != nil {
			summary.ValidCanonicalMismatches++
			detail := err.Error()
			if writeErr := writeCorpusResult(writer, corpusResult{
				CaseID:                 testCase.CaseID,
				ExpectedClassification: "valid",
				ActualClassification:   "canonicalization_error",
				InputSHA256:            testCase.InputSHA256,
				ExpectedCanonicalSHA256: &testCase.ExpectedCanonicalSHA256,
				Detail:                 &detail,
			}); writeErr != nil {
				return summary, writeErr
			}
			continue
		}
		actualSHA := digestHex(actual)
		canonicalBytes := len(actual)
		classification := "valid_match"
		detail := ""
		if !bytes.Equal(actual, expected) || actualSHA != testCase.ExpectedCanonicalSHA256 {
			classification = "canonical_mismatch"
			detail = "independent Go output differs from Rust serde_jcs bytes"
			summary.ValidCanonicalMismatches++
		} else {
			summary.ValidCanonicalMatches++
		}
		result := corpusResult{
			CaseID:                  testCase.CaseID,
			ExpectedClassification:  "valid",
			ActualClassification:    classification,
			InputSHA256:             testCase.InputSHA256,
			ExpectedCanonicalSHA256: &testCase.ExpectedCanonicalSHA256,
			ActualCanonicalSHA256:   &actualSHA,
			CanonicalBytes:          &canonicalBytes,
		}
		if detail != "" {
			result.Detail = &detail
		}
		if err := writeCorpusResult(writer, result); err != nil {
			return summary, err
		}
	}

	for _, testCase := range manifest.RejectCases {
		if _, exists := seen[testCase.CaseID]; exists || testCase.CaseID == "" {
			return summary, fmt.Errorf("duplicate or empty corpus case ID %q", testCase.CaseID)
		}
		seen[testCase.CaseID] = struct{}{}
		input, err := readCorpusFile(root, testCase.InputFile, testCase.InputSHA256)
		if err != nil {
			return summary, err
		}
		rejection := validateStrictJCSInput(input)
		if rejection == nil {
			summary.RejectStrictAccepts++
			detail := "strict preflight accepted an input classified by the Rust generator as reject"
			if err := writeCorpusResult(writer, corpusResult{
				CaseID:                 testCase.CaseID,
				ExpectedClassification: "reject",
				ActualClassification:   "false_accept",
				InputSHA256:            testCase.InputSHA256,
				Detail:                 &detail,
			}); err != nil {
				return summary, err
			}
			continue
		}
		summary.RejectCasesCorrectlyDenied++
		reason := rejection.Error()
		rejectionStage := "strict_raw_json_preflight"
		if err := writeCorpusResult(writer, corpusResult{
			CaseID:                 testCase.CaseID,
			ExpectedClassification: "reject",
			ActualClassification:   "rejected_before_canonicalization",
			InputSHA256:            testCase.InputSHA256,
			ExpectedRejectionClass: &testCase.ExpectedRejectionClass,
			ActualRejectionStage:   &rejectionStage,
			Detail:                 &reason,
		}); err != nil {
			return summary, err
		}
	}
	if err := writer.Flush(); err != nil {
		return summary, fmt.Errorf("flush corpus results: %w", err)
	}
	if err := resultsFile.Sync(); err != nil {
		return summary, fmt.Errorf("sync corpus results: %w", err)
	}
	summary.CaseIDCount = len(seen)
	summary.FailureCount = summary.ValidCanonicalMismatches +
		summary.ValidStrictRejects +
		summary.RejectStrictAccepts
	summaryBytes, err := json.MarshalIndent(summary, "", "  ")
	if err != nil {
		return summary, fmt.Errorf("serialize corpus summary: %w", err)
	}
	if err := os.WriteFile(summaryPath, summaryBytes, 0o644); err != nil {
		return summary, fmt.Errorf("write corpus summary: %w", err)
	}
	if summary.CaseIDCount != summary.ValidCaseCount+summary.RejectCaseCount {
		return summary, fmt.Errorf("corpus result count mismatch")
	}
	if summary.FailureCount != 0 {
		return summary, fmt.Errorf(
			"JCS corpus qualification failed: valid_mismatches=%d valid_rejected=%d reject_accepted=%d",
			summary.ValidCanonicalMismatches,
			summary.ValidStrictRejects,
			summary.RejectStrictAccepts,
		)
	}
	return summary, nil
}
