package main

import (
	"bytes"
	"crypto/sha256"
	"fmt"
	"os"

	"webpki.org/jsoncanonicalizer"
)

func main() {
	if len(os.Args) != 3 {
		fmt.Fprintln(os.Stderr, "usage: jcs-verifier <input.json> <expected.jcs>")
		os.Exit(64)
	}

	input, err := os.ReadFile(os.Args[1])
	if err != nil {
		fmt.Fprintf(os.Stderr, "read input: %v\n", err)
		os.Exit(2)
	}
	expected, err := os.ReadFile(os.Args[2])
	if err != nil {
		fmt.Fprintf(os.Stderr, "read expected canonical bytes: %v\n", err)
		os.Exit(2)
	}

	if err := validateStrictJCSInput(input); err != nil {
		fmt.Fprintf(os.Stderr, "strict JCS input validation failed: %v\\n", err)
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
