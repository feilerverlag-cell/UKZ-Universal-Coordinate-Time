
# UKZ 0.5.0 – Test Report

**Universal Coordinate Time**
**Author:** Dirk Leopold Feiler
**Date:** 2026-11-05
**Environment:** Android / Termux

## Test Summary

| Test Suite | Result |
|---|---:|
| Rust tests | 9 passed |
| Python tests | 10 passed |
| Total | 19 passed |
| Build check | Successful |

## Build Verification

Command:

```bash
cargo check --workspace
```

Result: Successful.

## Rust Tests

Command:

```bash
cargo test --workspace
```

Result: 9 tests passed.

## Python Tests

Command:

```bash
python -m unittest discover -s python -p "test*.py"
```

Result: 10 tests passed.

## Verified Test Areas

- Integer-based UKZ time representation
- Boundary and range handling
- Negative time values
- UTC and TAI epoch alignment
- Leap-second handling
- Invalid leap-second rejection
- UKZ conformance vectors
- Temporal monotonicity
- Rust and Python reference comparison

## Technical Note

These results document the software build and tests
performed in the Android Termux environment.

They do not constitute a complete mathematical,
scientific or normative certification of UKZ.

Further testing and validation remain part
of the ongoing development.

---

**UKZ 0.5.0**
Universal Coordinate Time  
Dirk Leopold Feiler
