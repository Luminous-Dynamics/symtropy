package main

import (
	"bytes"
	"os"
	"path/filepath"
	"testing"

	"webpki.org/jsoncanonicalizer"
)

func TestRFC8785GoldenVector(t *testing.T) {
	inputPath := filepath.Join("..", "golden", "basic-input.json")
	expectedPath := filepath.Join("..", "golden", "basic-output.jcs")

	input, err := os.ReadFile(inputPath)
	if err != nil {
		t.Fatalf("read JCS input vector: %v", err)
	}
	expected, err := os.ReadFile(expectedPath)
	if err != nil {
		t.Fatalf("read JCS expected vector: %v", err)
	}

	actual, err := jsoncanonicalizer.Transform(input)
	if err != nil {
		t.Fatalf("independent JCS transformation: %v", err)
	}
	if !bytes.Equal(actual, bytes.TrimSpace(expected)) {
		t.Fatalf("canonical bytes differ\nactual:   %s\nexpected: %s", actual, bytes.TrimSpace(expected))
	}
}
