package main

import (
	"bytes"
	"crypto/sha256"
	"fmt"
	"io"
	"os"

	"webpki.org/jsoncanonicalizer"
)

func readBoundedFile(path string, maxBytes int64) ([]byte, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, fmt.Errorf("open %s: %w", path, err)
	}
	defer file.Close()

	info, err := file.Stat()
	if err != nil {
		return nil, fmt.Errorf("stat %s: %w", path, err)
	}
	if !info.Mode().IsRegular() {
		return nil, fmt.Errorf("%s is not a regular file", path)
	}
	if info.Size() > maxBytes {
		return nil, fmt.Errorf("%s exceeds %d-byte limit", path, maxBytes)
	}

	data, err := io.ReadAll(io.LimitReader(file, maxBytes+1))
	if err != nil {
		return nil, fmt.Errorf("bounded read %s: %w", path, err)
	}
	if int64(len(data)) > maxBytes {
		return nil, fmt.Errorf("%s exceeded %d-byte limit while reading", path, maxBytes)
	}
	return data, nil
}

func main() {
	if len(os.Args) > 1 && os.Args[1] == "--corpus" {
		if len(os.Args) != 5 {
			fmt.Fprintln(os.Stderr, "usage: jcs-verifier --corpus <corpus-root> <results.jsonl> <summary.json>")
			os.Exit(64)
		}
		summary, err := verifyGeneratedCorpus(os.Args[2], os.Args[3], os.Args[4])
		if err != nil {
			fmt.Fprintf(os.Stderr, "generated JCS corpus verification failed: %v\n", err)
			os.Exit(1)
		}
		fmt.Printf("valid_case_count=%d\n", summary.ValidCaseCount)
		fmt.Printf("reject_case_count=%d\n", summary.RejectCaseCount)
		fmt.Printf("valid_canonical_matches=%d\n", summary.ValidCanonicalMatches)
		fmt.Printf("reject_cases_correctly_denied=%d\n", summary.RejectCasesCorrectlyDenied)
		fmt.Printf("failure_count=%d\n", summary.FailureCount)
		return
	}
	if len(os.Args) != 3 {
		fmt.Fprintln(os.Stderr, "usage: jcs-verifier <input.json> <expected.jcs>")
		os.Exit(64)
	}

	input, err := readBoundedFile(os.Args[1], maxJCSInputBytes)
	if err != nil {
		fmt.Fprintf(os.Stderr, "read input: %v\n", err)
		os.Exit(2)
	}
	expected, err := readBoundedFile(os.Args[2], maxJCSInputBytes)
	if err != nil {
		fmt.Fprintf(os.Stderr, "read expected canonical bytes: %v\n", err)
		os.Exit(2)
	}

	if err := validateStrictJCSInput(input); err != nil {
		fmt.Fprintf(os.Stderr, "strict JCS input validation failed: %v\n", err)
		os.Exit(2)
	}

	actual, err := jsoncanonicalizer.Transform(input)
	if err != nil {
		fmt.Fprintf(os.Stderr, "independent Go JCS transform failed: %v\n", err)
		os.Exit(2)
	}
	if !bytes.Equal(actual, bytes.TrimSpace(expected)) {
		fmt.Fprintln(os.Stderr, "independent Go JCS output differs from the frozen canonical bytes")
		fmt.Fprintf(os.Stderr, "actual:   %s\n", actual)
		fmt.Fprintf(os.Stderr, "expected: %s\n", bytes.TrimSpace(expected))
		os.Exit(1)
	}

	digest := sha256.Sum256(actual)
	fmt.Printf("implementation=cyberphone-go-jcs\n")
	fmt.Printf("canonical_bytes=%d\n", len(actual))
	fmt.Printf("canonical_sha256=%x\n", digest)
}
