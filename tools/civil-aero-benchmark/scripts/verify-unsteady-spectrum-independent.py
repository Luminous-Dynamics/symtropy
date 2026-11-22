#!/usr/bin/env python3
"""Independent direct-DFT oracle for the Rust Welch PSD synthetic known-tone fixture."""
from __future__ import annotations

import csv
import math
import sys
from pathlib import Path

FS = 512.0
DT = 1.0 / FS
SAMPLE_COUNT = 1024
SEGMENT_LENGTH = 256
OVERLAP = 128
AMPLITUDE = 2.0
TONE_HZ = 32.0
OFFSET = 1.25


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def close(actual: float, expected: float, message: str, tolerance: float = 2e-9) -> None:
    require(math.isclose(actual, expected, rel_tol=tolerance, abs_tol=tolerance),
            f"{message}: {actual} != {expected}")


def direct_welch() -> tuple[list[float], list[float], int]:
    samples = [
        OFFSET + AMPLITUDE * math.sin(2.0 * math.pi * TONE_HZ * n / FS)
        for n in range(SAMPLE_COUNT)
    ]
    window = [
        0.5 * (1.0 - math.cos(2.0 * math.pi * n / SEGMENT_LENGTH))
        for n in range(SEGMENT_LENGTH)
    ]
    window_energy = sum(value * value for value in window)
    stride = SEGMENT_LENGTH - OVERLAP
    starts = range(0, SAMPLE_COUNT - SEGMENT_LENGTH + 1, stride)
    bin_count = SEGMENT_LENGTH // 2 + 1
    averaged = [0.0] * bin_count
    segment_count = 0

    for start in starts:
        segment = samples[start:start + SEGMENT_LENGTH]
        mean = sum(segment) / SEGMENT_LENGTH
        detrended_windowed = [
            (segment[n] - mean) * window[n]
            for n in range(SEGMENT_LENGTH)
        ]
        for k in range(bin_count):
            real = 0.0
            imaginary = 0.0
            for n, value in enumerate(detrended_windowed):
                angle = 2.0 * math.pi * k * n / SEGMENT_LENGTH
                real += value * math.cos(angle)
                imaginary -= value * math.sin(angle)
            density = DT * (real * real + imaginary * imaginary) / window_energy
            if k != 0 and k != SEGMENT_LENGTH // 2:
                density *= 2.0
            averaged[k] += density
        segment_count += 1

    averaged = [value / segment_count for value in averaged]
    frequencies = [k * FS / SEGMENT_LENGTH for k in range(bin_count)]
    return frequencies, averaged, segment_count


def stationarity() -> tuple[float, float]:
    samples = [
        OFFSET + AMPLITUDE * math.sin(2.0 * math.pi * TONE_HZ * n / FS)
        for n in range(SAMPLE_COUNT)
    ]
    windows = []
    width = SAMPLE_COUNT // 8
    for index in range(8):
        block = samples[index * width:(index + 1) * width]
        mean = sum(block) / len(block)
        rms = math.sqrt(sum((value - mean) ** 2 for value in block) / len(block))
        windows.append((mean, rms))
    mean_drift = max(row[0] for row in windows) - min(row[0] for row in windows)
    mean_rms = sum(row[1] for row in windows) / len(windows)
    rms_drift = (max(row[1] for row in windows) - min(row[1] for row in windows)) / mean_rms
    return mean_drift, rms_drift


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: verify-unsteady-spectrum-independent.py psd.csv", file=sys.stderr)
        return 2
    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    frequencies, expected_psd, segment_count = direct_welch()
    require(len(rows) == len(frequencies), "frequency-bin count mismatch")
    require(segment_count == 7, "known overlap/segment count mismatch")
    for index, row in enumerate(rows):
        close(float(row["frequency_hz"]), frequencies[index], f"frequency bin {index}")
        close(float(row["psd_per_hz"]), expected_psd[index], f"PSD bin {index}")

    df = FS / SEGMENT_LENGTH
    integrated = sum(expected_psd) * df
    time_series = [
        OFFSET + AMPLITUDE * math.sin(2.0 * math.pi * TONE_HZ * n / FS)
        for n in range(SAMPLE_COUNT)
    ]
    global_mean = sum(time_series) / len(time_series)
    time_rms = math.sqrt(sum((value - global_mean) ** 2 for value in time_series) / len(time_series))
    peak_index = max(range(1, len(expected_psd)), key=lambda index: expected_psd[index])
    band = sum(
        expected_psd[index] * df
        for index, frequency in enumerate(frequencies)
        if 30.0 <= frequency <= 34.0
    )
    mean_drift, rms_drift = stationarity()
    close(frequencies[peak_index], 32.0, "dominant peak")
    close(integrated, 2.0, "integrated PSD mean-square")
    close(time_rms, math.sqrt(2.0), "time-domain RMS")
    close(integrated, time_rms**2, "PSD/time-domain Parseval consistency")
    close(band, 2.0, "30-34 Hz band power")
    close(mean_drift, 0.0, "stationarity mean drift")
    close(rms_drift, 0.0, "stationarity RMS drift")
    print(
        "independent_unsteady_spectrum_oracle=pass samples=1024 segments=7 bins=129 "
        "peak_hz=32 integrated_mean_square=2 time_rms=sqrt2 parseval=pass"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError) as error:
        print(f"independent_unsteady_spectrum_oracle=fail reason={error}", file=sys.stderr)
        raise SystemExit(1)
