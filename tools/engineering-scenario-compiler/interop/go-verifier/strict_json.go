package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"math"
	"strconv"
	"unicode/utf8"
)

// validateStrictJCSInput rejects ambiguous or non-I-JSON-shaped input before
// the canonicalizer sees bytes that might otherwise be parsed lossily.
func validateStrictJCSInput(input []byte) error {
	if !utf8.Valid(input) {
		return fmt.Errorf("input is not valid UTF-8")
	}
	if err := rejectUnpairedSurrogateEscapes(input); err != nil {
		return err
	}

	decoder := json.NewDecoder(bytes.NewReader(input))
	decoder.UseNumber()
	if err := consumeJSONValue(decoder); err != nil {
		return err
	}
	if _, err := decoder.Token(); !errors.Is(err, io.EOF) {
		if err != nil {
			return fmt.Errorf("trailing JSON content: %w", err)
		}
		return fmt.Errorf("trailing JSON value")
	}
	return nil
}

func consumeJSONValue(decoder *json.Decoder) error {
	token, err := decoder.Token()
	if err != nil {
		return fmt.Errorf("invalid JSON token: %w", err)
	}

	switch value := token.(type) {
	case json.Delim:
		switch value {
		case '{':
			seen := make(map[string]struct{})
			for decoder.More() {
				keyToken, err := decoder.Token()
				if err != nil {
					return fmt.Errorf("invalid object key: %w", err)
				}
				key, ok := keyToken.(string)
				if !ok {
					return fmt.Errorf("object member name is not a string")
				}
				if _, exists := seen[key]; exists {
					return fmt.Errorf("duplicate object member name %q", key)
				}
				seen[key] = struct{}{}
				if err := consumeJSONValue(decoder); err != nil {
					return err
				}
			}
			closing, err := decoder.Token()
			if err != nil || closing != json.Delim('}') {
				return fmt.Errorf("unterminated JSON object")
			}
		case '[':
			for decoder.More() {
				if err := consumeJSONValue(decoder); err != nil {
					return err
				}
			}
			closing, err := decoder.Token()
			if err != nil || closing != json.Delim(']') {
				return fmt.Errorf("unterminated JSON array")
			}
		default:
			return fmt.Errorf("unexpected JSON delimiter %q", value)
		}
	case json.Number:
		number, err := strconv.ParseFloat(value.String(), 64)
		if err != nil || math.IsInf(number, 0) || math.IsNaN(number) {
			return fmt.Errorf("JSON number %q is not finite IEEE-754 binary64", value)
		}
	case string, bool, nil:
		// These JSON primitive types are valid in the canonicalization model.
	default:
		return fmt.Errorf("unsupported JSON token type %T", token)
	}
	return nil
}

func rejectUnpairedSurrogateEscapes(input []byte) error {
	inString := false
	for index := 0; index < len(input); {
		current := input[index]
		if !inString {
			if current == '"' {
				inString = true
			}
			index++
			continue
		}

		switch current {
		case '"':
			inString = false
			index++
		case '\\':
			if index+1 >= len(input) {
				return fmt.Errorf("unterminated escape sequence")
			}
			escape := input[index+1]
			if escape != 'u' {
				index += 2
				continue
			}

			codeUnit, ok := parseHexCodeUnit(input[index+2:])
			if !ok {
				return fmt.Errorf("invalid Unicode escape")
			}
			index += 6

			switch {
			case codeUnit >= 0xD800 && codeUnit <= 0xDBFF:
				if index+6 > len(input) || input[index] != '\\' || input[index+1] != 'u' {
					return fmt.Errorf("high surrogate is not followed by a low surrogate")
				}
				low, ok := parseHexCodeUnit(input[index+2:])
				if !ok || low < 0xDC00 || low > 0xDFFF {
					return fmt.Errorf("high surrogate is not followed by a low surrogate")
				}
				index += 6
			case codeUnit >= 0xDC00 && codeUnit <= 0xDFFF:
				return fmt.Errorf("unpaired low surrogate")
			}
		default:
			if current < 0x20 {
				return fmt.Errorf("unescaped control character in JSON string")
			}
			index++
		}
	}
	if inString {
		return fmt.Errorf("unterminated JSON string")
	}
	return nil
}

func parseHexCodeUnit(input []byte) (uint16, bool) {
	if len(input) < 4 {
		return 0, false
	}
	var value uint16
	for _, digit := range input[:4] {
		value <<= 4
		switch {
		case digit >= '0' && digit <= '9':
			value += uint16(digit - '0')
		case digit >= 'a' && digit <= 'f':
			value += uint16(digit-'a') + 10
		case digit >= 'A' && digit <= 'F':
			value += uint16(digit-'A') + 10
		default:
			return 0, false
		}
	}
	return value, true
}
