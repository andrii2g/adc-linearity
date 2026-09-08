# adc-linearity-audit

CLI tool for auditing static ADC linearity from a known-input ramp.
Measure transition widths, DNL, INL, input-referred offset and gain span error;
identify unobserved codes and observed code reversals; export inspectable SVGs.

**Status: implemented MVP.** The analyzer, deterministic synthetic generator,
machine-readable reports, bounded SVG renderer, and six-scenario demo are available.
See [VALIDATION.md](VALIDATION.md) for the exact toolchain and acceptance evidence.

## Usage

```bash
cargo run --release -- analyze fixtures/perfect-3bit.csv --bits 3 --vmin 0 --vmax 8 --reference all --out results/perfect
cargo run --release -- synth --model bow --bits 12 --vmin 0 --vmax 3.3 --amplitude-lsb 3 --samples-per-lsb 64 --out results/bow-input
cargo run --release -- analyze results/bow-input/sweep.csv --bits 12 --vmin 0 --vmax 3.3 --reference all --out results/bow-audit
cargo run --release -- demo --out results/demo
```

Input CSV contains `input_v,code`. Input voltages must increase strictly. Codes are
unsigned integers. Supply bit depth and nominal analogue range explicitly.
There is no hardware capture and no assumed input-ramp sample rate.

Analyze writes summary.json, transitions.csv, codes.csv, transfer.svg, dnl.svg,
and inl.svg. Synth writes sweep.csv plus independent truth.json. Output directories
must be new or empty; existing nonempty directories are never overwritten.

Use --reference nominal, endpoint, best-fit, or all (the default). --strict returns
exit code 3 after writing a report when the sweep is partial or contains a code
reversal. It is a data-completeness check, not a datasheet limit.

## Interpretation

- Nominal results use q=(vmax-vmin)/2^bits and retain offset and gain effects.
- Endpoint and best-fit results remove different affine components and normalize
  both DNL and INL by the fitted reference slope.
- A multi-code jump leaves crossed transitions and affected widths unresolved.
  An unobserved code is a candidate, never proof of a physically zero-width bin.
- A code reversal invalidates all whole-sweep linearity and calibration estimates,
  while retaining bounded diagnostic events and the sampled transfer plot.
- Sampling brackets describe input-grid resolution only; they are not confidence
  intervals and exclude source error, noise, settling, and hysteresis.

The bow demo is particularly useful: its upper INL panel shows nominal total
transition error, while the lower panel separates endpoint and least-squares INL.
The missing-code demo contrasts synthetic truth (DNL=-1) with the audit, which
correctly reports null point DNL across the unresolved jump. Representative plots
are kept in [examples](examples/README.md).

## Limitations

The MVP analyzes one strictly increasing deterministic ramp. It does not implement
hardware capture, descending/noisy sweep fitting, code-density analysis, dynamic
metrics, FFT/ENOB, hysteresis estimation, fitted-line confidence bands, or automatic
datasheet pass/fail thresholds. Bit depth is limited to 2–16 and synthesis to
5,000,000 samples.

## Documentation

| File | Contents |
| --- | --- |
| [docs/SPEC.md](docs/SPEC.md) | Scope, invariants, validity and coverage |
| [docs/MATH.md](docs/MATH.md) | Equations, indexing, reference conventions, examples |
| [docs/ALGORITHMS.md](docs/ALGORITHMS.md) | Streaming extraction, fitting, uncertainty, gap behavior |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Modules, public contracts and data ownership |
| [docs/CLI.md](docs/CLI.md) | Commands, flags, errors and exit codes |
| [docs/REPORT_FORMAT.md](docs/REPORT_FORMAT.md) | JSON/CSV contract and example |
| [docs/SYNTHETIC_MODELS.md](docs/SYNTHETIC_MODELS.md) | Deterministic threshold models and truth |
| [docs/PLOTS.md](docs/PLOTS.md) | Accessible SVG layout and bounded plotting |
| [docs/TEST_PLAN.md](docs/TEST_PLAN.md) | Hand-calculated fixtures and acceptance matrix |
| [docs/DECISIONS.md](docs/DECISIONS.md) | Resolved ambiguities and future boundaries |
| [docs/SOURCES.md](docs/SOURCES.md) | Background source and convention caveats |
| [VALIDATION.md](VALIDATION.md) | Toolchain, automated checks and visual QA evidence |

`fixtures/expected.json` contains exact, independently specified fixture results.
`cargo test` owns the numerical fixture oracle, input validation matrix, report
schema checks, SVG safety checks, six-scenario demo validation, and byte-for-byte
determinism comparison. No Python runtime is required to build or verify the project.

## Why this project matters

A converter can be repeatable and quiet while having a systematically curved transfer
function. DNL exposes local code-width changes; corrected INL exposes accumulated
shape error; nominal transition error retains offset and gain. Comparing references
shows which errors affine calibration removes and which remain.

Results are static sweep estimates. Input uncertainty, noise, hysteresis and settling
are outside the estimation model. A transition bracket reflects sampling resolution,
not a confidence interval or a complete uncertainty budget.
