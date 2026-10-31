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

	if err := validateStrictJCSInput(input); err != nil {
		t.Fatalf("strict JCS input validation: %v", err)
	}

	actual, err := jsoncanonicalizer.Transform(input)
	if err != nil {
		t.Fatalf("independent JCS transformation: %v", err)
	}
	if !bytes.Equal(actual, bytes.TrimSpace(expected)) {
		t.Fatalf("canonical bytes differ\nactual:   %s\nexpected: %s", actual, bytes.TrimSpace(expected))
	}
}


func TestStrictJCSInputRejectsAmbiguousAndInvalidJSON(t *testing.T) {
	tests := []struct {
		name  string
		input []byte
	}{
		{name: "duplicate top-level member", input: []byte("{\"x\":1,\"x\":2}")},
		{name: "duplicate nested member", input: []byte("{\"outer\":{\"x\":1,\"x\":2}}")},
		{name: "escaped duplicate member name", input: []byte("{\"a\":1,\"\\u0061\":2}")},
		{name: "invalid UTF-8", input: []byte{0xff, 0x7b, 0x7d}},
		{name: "unpaired high surrogate", input: []byte("{\"x\":\"\\uD800\"}")},
		{name: "unpaired low surrogate", input: []byte("{\"x\":\"\\uDC00\"}")},
		{name: "non-finite binary64 overflow", input: []byte("1e9999")},
		{name: "trailing JSON value", input: []byte("{} {}")},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			if err := validateStrictJCSInput(test.input); err == nil {
				t.Fatalf("expected strict validation to reject %q", test.input)
			}
		})
	}
}

func TestStrictJCSInputAcceptsValidSurrogatePairAndNestedValues(t *testing.T) {
	input := []byte("{\"outer\":[{\"symbol\":\"\\uD834\\uDD1E\"},-0,1e30,true,null]}")
	if err := validateStrictJCSInput(input); err != nil {
		t.Fatalf("valid JCS input rejected: %v", err)
	}
}
