# Validation

Validation date: 2026-09-08

Toolchain:

- rustc 1.97.1 (8bab26f4f 2026-07-14)
- cargo 1.97.1 (c980f4866 2026-06-30)
- Windows, x86-64

## Completed gates

| Command | Outcome |
| --- | --- |
| cargo fmt --check | PASS |
| cargo clippy --all-targets -- -D warnings | PASS |
| cargo test | PASS: 30 tests (20 analysis, 7 CLI, 3 configuration), plus doc tests |
| cargo build --release | PASS |
| python scripts/check_project.py | PASS: project/fixture checks |
| cargo run --release -- demo --out results/demo | PASS |
| python scripts/check_outputs.py results/demo | PASS: six artifact sets |
| second release demo plus SHA-256 comparison | PASS: all 49 files byte-for-byte identical |

The tests cover independent hand-derived perfect, bow, affine, missing-code,
coarse-sampling, partial, constant-code, and reversal fixtures. They also cover CSV
validation/accepted variants, DNL/INL indexing and normalization, calibration signs,
cumulative gap re-anchoring, extrema tie rules, saturation bins, centered fitting,
sampling bounds, diagnostic caps, synthesis threshold equality, output collision
safety, strict-mode exit behavior, synthesis resource limits, every frozen JSON
fixture oracle, 16-bit code-space sizing, exact bounded transfer-point retention,
CSV boolean serialization, source escaping, gap-safe plotting, small-code DNL bars,
resolved bracket markers, and the 2048-point large-series bound.

## CLI acceptance matrix

The exact commands from docs/CLI.md were exercised:

| Case | Expected | Observed |
| --- | ---: | ---: |
| --help | 0 | 0 |
| bow-hand analyze | 0 | 0 |
| missing-code analyze --strict | 3 after reports | 3 after reports |
| reversal analyze | 0 with diagnostic report | 0 |
| non-increasing analyze | 2, no output directory | 2, no output directory |
| bipolar perfect synth | 0 | 0 |

## Visual QA

The generated perfect transfer, bow INL, missing-code DNL, affine, and reversal
plots were parsed as XML. Final bow INL, missing-code DNL, perfect transfer, and
reversal transfer plots were rendered with Microsoft Edge headless and inspected:
titles, adaptive tick labels, legends, separated nominal and corrected panels, line
styles, the -1 truth boundary, and diagnostic gaps were readable. Missing-code DNL
uses gaps/pale markers rather than measured -1 values; reversal output retains the
sampled reversal while omitting corrected references. Representative deterministic
SVGs are retained in examples/.

## Known scope limits

The validated behavior is the deterministic increasing-ramp MVP described in
docs/SPEC.md. Hardware acquisition, stochastic transition fitting, source uncertainty,
hysteresis, dynamic testing, and datasheet pass/fail limits remain intentionally out
of scope.
