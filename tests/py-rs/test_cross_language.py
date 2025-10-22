#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "tobytes",
#     "click>=8.0",
#     "pyyaml>=6.0",
#     "numpy>=1.20",
#     "polars",
# ]
#
# [tool.uv.sources]
# tobytes = { path = "../../py", editable = true }
# ///
"""Cross-language test harness for Python and Rust tobytes implementations."""

import json
import subprocess
import sys
from pathlib import Path

import click
import numpy as np
import polars as pl
import tobytes
import yaml


def build_rust_binary():
    """Build the Rust test harness binary."""
    test_dir = Path(__file__).parent
    print("Building Rust test harness...")

    result = subprocess.run(
        ["cargo", "build", "--release"],
        cwd=test_dir,
        capture_output=True,
        text=True
    )

    if result.returncode != 0:
        print(f"Failed to build Rust binary:")
        print(result.stderr)
        sys.exit(1)

    bin_path = test_dir / "target" / "release" / "test_harness"
    if not bin_path.exists():
        raise FileNotFoundError(f"Rust binary not found at {bin_path} after build")

    print(f"Build complete: {bin_path}\n")
    return bin_path


def test_py_encode_rs_decode(test_values: list, case_file: Path, rust_binary: Path, verbose: bool = False, py_prepare: str = None):
    """Test: Python encodes -> Rust decodes."""
    codec = tobytes.Codec()

    # Python: encode each test value to bytes with length prefix
    encoded_data = bytearray()
    for value in test_values:
        # Apply py_prepare transformation if specified
        if py_prepare:
            namespace = {'value': value}
            exec(f'value = {py_prepare}', globals(), namespace)
            value = namespace['value']

        encoded = codec.dumps(value)
        # Write 4-byte length prefix (big-endian)
        encoded_data.extend(len(encoded).to_bytes(4, 'big'))
        # Write encoded bytes
        encoded_data.extend(encoded)

    # Rust: decode the byte stream
    result = subprocess.run(
        [str(rust_binary), "decode", str(case_file)],
        input=bytes(encoded_data),
        capture_output=True
    )

    if result.returncode != 0:
        error_msg = result.stderr.decode('utf-8', errors='replace') if result.stderr else result.stdout.decode('utf-8', errors='replace')
        raise RuntimeError(f"Rust decode failed: {error_msg}")

    decoded_json = json.loads(result.stdout.decode('utf-8'))
    return decoded_json


def test_rs_encode_py_decode(test_values: list, case_file: Path, rust_binary: Path, verbose: bool = False):
    """Test: Rust encodes -> Python decodes."""
    # Rust: encode test values to byte stream
    result = subprocess.run(
        [str(rust_binary), "encode", str(case_file)],
        capture_output=True
    )

    if result.returncode != 0:
        error_msg = result.stderr.decode('utf-8', errors='replace') if result.stderr else result.stdout.decode('utf-8', errors='replace')
        raise RuntimeError(f"Rust encode failed: {error_msg}")

    # Python: decode each byte chunk from the stream
    codec = tobytes.Codec()
    decoded_values = []

    data = result.stdout
    offset = 0
    while offset < len(data):
        # Read 4-byte length prefix (big-endian)
        if offset + 4 > len(data):
            break
        length = int.from_bytes(data[offset:offset+4], 'big')
        offset += 4

        # Read encoded bytes
        if offset + length > len(data):
            break
        encoded_bytes = data[offset:offset+length]
        offset += length

        # Decode
        decoded = codec.loads(encoded_bytes)
        decoded_values.append(decoded)

    return decoded_values


def normalize_value(value):
    """Normalize values for comparison (handle both dict and simple types)."""
    if isinstance(value, dict):
        return value
    return value


def compare_values(original, decoded, path="", py_compare="expected == actual"):
    """Compare original and decoded values, handling type conversions."""
    namespace = {'expected': original, 'actual': decoded}
    try:
        result = eval(py_compare, globals(), namespace)
        if not result:
            print(f"Comparison failed at {path}: expected={original}, actual={decoded}")
        return result
    except Exception as e:
        print(f"Comparison error at {path}: {e}")
        return False


def run_test_case(case_file: Path, rust_binary: Path, verbose: bool = False):
    """Run both directions of testing for a single test case."""
    case_name = case_file.name
    results = []

    with open(case_file) as f:
        test_case = yaml.safe_load(f)

    test_values = test_case['tests']
    description = test_case.get('description', '')
    py_prepare = test_case.get('py_prepare')
    py_compare = test_case.get('py_compare', 'expected == actual')
    if verbose:
        print(f"\n{click.style('Testing', bold=True)} {case_name}: {description}")
        print(f"  Rust type: {test_case['rust_type']}")
        print(f"  Test values: {test_values}")
        if py_prepare:
            print(f"  py_prepare: {py_prepare}")
        if py_compare != 'expected == actual':
            print(f"  py_compare: {py_compare}")

    # Test 1: Python encode -> Rust decode
    try:
        decoded_from_rust = test_py_encode_rs_decode(test_values, case_file, rust_binary, verbose, py_prepare)
        if verbose:
            print(f"  Py->Rs decoded: {decoded_from_rust}")

        # Compare each value (compare original test values with decoded)
        all_match = True
        for i, (original, decoded) in enumerate(zip(test_values, decoded_from_rust)):
            if not compare_values(original, decoded, f"test[{i}]", py_compare):
                all_match = False
                break

        if not all_match:
            if verbose:
                print(f"  {click.style('✗', fg='red', bold=True)} Python encode -> Rust decode: FAILED")
            else:
                print(f"{click.style(case_name, fg='cyan')} {click.style('py->rs', fg='yellow')} {click.style('✗ FAILED', fg='red', bold=True)}")
            return False

        if verbose:
            print(f"  {click.style('✓', fg='green', bold=True)} Python encode -> Rust decode: PASSED")
        else:
            results.append(('py->rs', True, None))
    except Exception as e:
        if verbose:
            print(f"  {click.style('✗', fg='red', bold=True)} Python encode -> Rust decode: {e}")
        else:
            print(f"{click.style(case_name, fg='cyan')} {click.style('py->rs', fg='yellow')} {click.style('✗ FAILED', fg='red', bold=True)} ({e})")
        return False

    # Test 2: Rust encode -> Python decode
    try:
        decoded_from_py = test_rs_encode_py_decode(test_values, case_file, rust_binary, verbose)
        if verbose:
            print(f"  Rs->Py decoded: {decoded_from_py}")

        # Compare each value (compare original test values with decoded)
        all_match = True
        for i, (original, decoded) in enumerate(zip(test_values, decoded_from_py)):
            if not compare_values(original, decoded, f"test[{i}]", py_compare):
                all_match = False
                break

        if not all_match:
            if verbose:
                print(f"  {click.style('✗', fg='red', bold=True)} Rust encode -> Python decode: FAILED")
            else:
                print(f"{click.style(case_name, fg='cyan')} {click.style('rs->py', fg='yellow')} {click.style('✗ FAILED', fg='red', bold=True)}")
            return False

        if verbose:
            print(f"  {click.style('✓', fg='green', bold=True)} Rust encode -> Python decode: PASSED")
        else:
            results.append(('rs->py', True, None))
    except Exception as e:
        if verbose:
            print(f"  {click.style('✗', fg='red', bold=True)} Rust encode -> Python decode: {e}")
        else:
            print(f"{click.style(case_name, fg='cyan')} {click.style('rs->py', fg='yellow')} {click.style('✗ FAILED', fg='red', bold=True)} ({e})")
        return False

    # Print compact results for non-verbose mode
    if not verbose:
        for direction, passed, error in results:
            status = click.style('✓ PASSED', fg='green', bold=True)
            print(f"{click.style(case_name, fg='cyan')} {click.style(direction, fg='yellow')} {status}")
    else:
        print(f"  {click.style('✅ All tests PASSED', fg='green', bold=True)} for {case_name}")

    return True


@click.command()
@click.option(
    "-v",
    "--verbose",
    is_flag=True,
    help="Enable verbose output showing original and decoded values"
)
def main(verbose: bool):
    """Run all test cases for cross-language tobytes compatibility."""
    test_dir = Path(__file__).parent
    cases_dir = test_dir / "cases"

    if not cases_dir.exists():
        print(f"Error: Test cases directory not found: {cases_dir}")
        sys.exit(1)

    rust_binary = build_rust_binary()

    test_cases = sorted(cases_dir.glob("*.yaml"))
    if not test_cases:
        print(f"No test cases found in {cases_dir}")
        sys.exit(1)

    print(f"Found {len(test_cases)} test case(s)")

    passed = 0
    failed = 0

    for case_file in test_cases:
        if run_test_case(case_file, rust_binary, verbose):
            passed += 1
        else:
            failed += 1

    print(f"\n{'='*50}")
    print(f"Results: {passed} passed, {failed} failed out of {len(test_cases)} total")
    print(f"{'='*50}")

    if failed > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
