# Test and acceptance plan

The implemented suite uses Rust's built-in test framework with no RNG or network.
Numerical tests use hand-derived values plus interval invariants; generator-to-analyzer
round trips are supplemental rather than the sole oracle.

## Frozen fixture matrix

Every file uses --bits 3 --vmin 0 --vmax 8. Expected values are in expected.json.
Dense hand fixtures use dyadic sample positions so their threshold midpoints are exact.
They are test artifacts, not the same grid as the future synth command.

| Fixture | Purpose and exact expectation |
| --- | --- |
| perfect-3bit.csv | T=1..7, q=1, 64 samples, all INL/DNL zero; brackets +/-1/16 |
| bow-hand-3bit.csv | T=[1,2,3.25,4.5,5.25,6,7]; endpoint peak .5; OLS b=1,a=1/7 |
| affine-3bit.csv | T_k=.25+1.125k; offset .375; span error .75 V / 12.5%; corrected errors zero |
| missing-3bit.csv | True thresholds [1,2,3,3,5,6,7]; code 3 absent, T3/T4 unresolved; audit width3 null |
| coarse-perfect-3bit.csv | Ideal floor samples .5,2.5,4.5,6.5,7.5; codes 1,3,5 absent despite perfect truth |
| partial-3bit.csv | Observe codes 2..5; only T3,T4,T5 resolved; endpoint/calibration unavailable |
| constant-3bit.csv | Three samples with code3; no transitions, no available refs |
| reversal-3bit.csv | Codes 0,1,0,2; all metrics invalidated, one reversal and one jump |

The hand bow is deliberately a piecewise threshold profile, not the analytic quadratic
model. It provides easy exact OLS/cumulative answers independent of sin or synthesis.
Affinely transformed fixture extends above vmax: this must be accepted.

## Numerical test matrix

| ID | Test | Required assertion |
| --- | --- | --- |
| M01 | Nominal sizing | B=3 -> 8 codes,7 transitions,6 widths,q=1 |
| M02 | Perfect midpoint | Every T midpoint exact; width bounds [.875,1.125] |
| M03 | Bow widths | [1,1.25,1.25,.75,.75,1] |
| M04 | Endpoint cumulative | [0,0,.25,.5,.25,0,0] |
| M05 | Best-fit start | First INL=-1/7, max_abs=5/14; never zero-anchor it |
| M06 | Affine calibration | offset=.375 != intercept .25; span=12.5%, nominal DNL=.125 |
| M07 | Fitted normalization | W/b-1=0 for affine; W/q-1=.125 remains separately reported |
| M08 | Missing ambiguity | Point T3,T4 and widths2,3,4 null; bounds retained; candidate=[3] |
| M09 | Coarse counterexample | candidates=[1,3,5], truth missing=[]; no DNL=-1 inference |
| M10 | Partial endpoints | Endpoint unavailable despite 3 resolved middle transitions |
| M11 | Reversal gate | All references/calibration unavailable; transfer diagnostics retained |
| M12 | Gap runs | Missing case has runs [1,2] and [5,6,7]; reanchor at k5 |
| M13 | True zero bin | synth model code(x) skips exactly code3 when T3=T4; true DNL3=-1 |
| M14 | Equality at threshold | Threshold quantizer emits higher code, including equal thresholds |
| M15 | OLS centering | Large voltage origin with representable small span stays stable |
| M16 | Absent metrics | Zero valid widths -> null DNL summary, never zero maximum |
| M17 | Extrema ties | Lowest index wins min/max/max_abs ties |
| M18 | Saturation bins | Count them as observed but widths and all DNL values remain null |
| M19 | Bounds coverage | Resolved truth is inside brackets, true widths inside width bounds |
| M20 | Cumulative identity | Each contiguous run cumulative/direct match within scaled tolerance |
| M21 | Reference choice | Calibration identical for nominal/endpoint/best-fit/all selection |
| M22 | Dense sampling | Narrow bins eventually resolve; coarse absent code is never proven missing |

For exact dyadic examples, compare midpoint/width values exactly when appropriate.
For OLS and residual arithmetic, use abs(actual-expected)<=1e-12*max(1,abs(expected)).
For long cumulative runs, allow 1e-10*max(1,max_abs_direct) with compensated accumulation;
choose larger tolerance only with an explained error bound, not to hide indexing bugs.
For generated nominal values use sample-bracket-derived bounds. For fitted values,
compare to direct exact truth with a documented perturbation allowance including line
fit error; do not reuse a nominal-only uncertainty bound as a fitted confidence bound.

## Validation failures

Test missing/extra/duplicate headers, ragged records, blank numeric cells, code -1,
code 1.5, code8 for B3, oversized integer, NaN, Infinity, repeated/decreasing input,
one-row input, unsupported bits, vmax<=vmin, subtraction overflow, unrepresentable q,
and malformed quoting. All return 2 with logical record/context and no report artifacts.
Test UTF-8 BOM, reversed column order, surrounding whitespace, scientific notation
and CRLF as accepted inputs. These variants need not all be permanent fixture files.

## Resource and output behavior

Use a generated iterator (not a stored huge CSV) to test >1000 jumps/reversals and
ensure retained counts capped but totals exact. On first reversal, stop later per-k
expansion to avoid pathological work. Test M=65536 sizing and a large streamed input;
assert collection lengths/caps rather than a brittle machine-dependent RSS threshold.
Reject synth counts >5M before allocation/write. Check numeric overflow paths.

Output directory reuse returns 1 without altering existing sentinel bytes. Read-only
or invalid paths return 1. --strict partial/non_monotonic returns 3 only after full
report output. A report-writing failure takes precedence (1). No stdout error stack
trace. --help/--version return 0. Tests must not rely on Unix-only shell features.
Use std::process::Command + env!("CARGO_BIN_EXE_adc-linearity-audit") in integration tests.
Unique temporary directories can use process ID plus atomic counter; clean only those
owned by the test. No extra dev dependency is needed.

## Artifact tests

- Deserialize summary JSON and compare every available fixture field to expected.json.
- Verify all requested reference objects exist even when unavailable.
- Read generated CSV using csv crate; all row counts, headers and nulls agree with JSON.
- Validate SVG structure and forbidden-element rules in Rust tests; inspect rendered images.
- Escape a filename such as a&b.csv in SVG; no injected XML or external resource.
- Gap tests check missing bins do not become polylines crossing null regions.
- Repeat a demo in two fresh directories and compare outputs on one platform.
- At least one separate test must demonstrate that perfect/coarse input has candidates
  while synthetic truth reports zero missing codes.

## Required final commands

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
cargo run --release -- demo --out results/demo
```

Also execute the CLI examples in docs/CLI.md, checking intended nonzero exit statuses.
The Rust integration suite owns the golden fixture comparison, demo structure and
semantics, SVG safety checks, and deterministic two-run artifact comparison.
