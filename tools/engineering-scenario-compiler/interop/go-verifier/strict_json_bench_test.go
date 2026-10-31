package main

import (
	"os"
	"path/filepath"
	"testing"

	"webpki.org/jsoncanonicalizer"
)

func loadJCSBenchmarkInput(b *testing.B) []byte {
	b.Helper()
	path := os.Getenv("JCS_BENCH_INPUT")
	if path == "" {
		path = filepath.Join("..", "golden", "adversarial-input.json")
	}
	input, err := os.ReadFile(path)
	if err != nil {
		b.Fatalf("read JCS benchmark input %q: %v", path, err)
	}
	if err := validateStrictJCSInput(input); err != nil {
		b.Fatalf("benchmark input failed strict validation: %v", err)
	}
	b.SetBytes(int64(len(input)))
	return input
}

func BenchmarkStrictPreflight(b *testing.B) {
	input := loadJCSBenchmarkInput(b)
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		if err := validateStrictJCSInput(input); err != nil {
			b.Fatalf("strict preflight failed: %v", err)
		}
	}
}

func BenchmarkJCSCanonicalize(b *testing.B) {
	input := loadJCSBenchmarkInput(b)
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		if _, err := jsoncanonicalizer.Transform(input); err != nil {
			b.Fatalf("JCS canonicalization failed: %v", err)
		}
	}
}

func BenchmarkStrictThenJCS(b *testing.B) {
	input := loadJCSBenchmarkInput(b)
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		if err := validateStrictJCSInput(input); err != nil {
			b.Fatalf("strict preflight failed: %v", err)
		}
		if _, err := jsoncanonicalizer.Transform(input); err != nil {
			b.Fatalf("JCS canonicalization failed: %v", err)
		}
	}
}
