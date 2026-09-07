# CLI contract

Binary: adc-linearity-audit. clap derive; subcommands analyze, synth, demo.
`--help` and `--version` work without data or output directories. No interactive prompts.
Use invariant decimal notation and accept scientific notation for input voltages.

## analyze

```bash
adc-linearity-audit analyze <CSV> --bits <B> --vmin <V> --vmax <V> \
  [--reference nominal|endpoint|best-fit|all] --out <DIR> [--strict]
```

| Argument | Default / validation |
| --- | --- |
| CSV | Required filesystem path; stdin is not supported in MVP |
| --bits | Required; 2..16 |
| --vmin, --vmax | Required finite voltages; negative values allowed |
| --reference | all; fixed order nominal,endpoint,best_fit in reports |
| --out | Required, new or empty directory |
| --strict | False; after writing diagnostics, exit 3 unless status valid |

`--strict` does not compare numerical error to a device tolerance. It checks sweep
completeness/monotonicity only. No implementation of datasheet pass/fail thresholds.
A valid noiseless but strongly nonlinear ADC still exits 0 under --strict.
Use clap allow_hyphen_values for negative voltage and offset arguments.

Console summary: input basename, rows, bits/range/q, sweep status, transition and
width coverage, step max/mean in LSB, jump/reversal counts, unobserved-code counts,
endpoint offset and gain span (or unavailable reason), and one metrics row per
selected reference. Explicitly label partial maxima and total transition error.
Print report filenames. Summaries go to stdout; warnings and command errors to stderr.
Never print a successful numeric summary when its values are null.

## synth

```bash
adc-linearity-audit synth --model perfect|bow|periodic|missing-code \
  [--bits 12] [--vmin 0] [--vmax 3.3] [--samples-per-lsb 64] \
  [--amplitude-lsb 3] [--periods 8] [--missing-code 2048] \
  [--offset-lsb 0] [--span-error-percent 0] --out <DIR>
```

Applicable defaults: amplitude=3 for bow, amplitude=.4 for periodic; the displayed
help must explain that defaults depend on model. periods=8 only for periodic.
missing code defaults to M/2 only for missing-code. Reject model-inapplicable flags
if explicitly supplied (e.g. --periods on perfect). `--missing-code k` selects one
interior code k in 1..M-2; only one missing code is required in MVP.
Samples per nominal LSB: integer 1..1024; default 64. Require total generated sample
count <=5,000,000 using checked arithmetic. Reject negative/non-finite amplitude,
nonpositive integer periods, nonpositive affine scale, and non-finite transforms.
The model is validated after all transforms; do not sort invalid thresholds.

Output sweep.csv and truth.json. Prints actual span, sample count and truth model.
No analyzer report is emitted by synth; that is analyze's job.

## demo

```bash
adc-linearity-audit demo --out <DIR>
```

Use fixed B=8, vmin=0,vmax=2.56,samples_per_lsb=64. Six directories:
perfect, bow, periodic, missing-code, affine, coarse-perfect. Each contains input/
(sweep.csv,truth.json) and audit/ (all six report outputs). Reference selection all.
Models: bow A=3; periodic A=.4,p=8; missing k=128; affine perfect with offset_lsb=.25
and span_error_percent=1; coarse-perfect uses every 256th sample from the dense
perfect sweep, appending the final sample if necessary. Coarse truth still says
perfect; truth metadata identifies the decimation. Never imply its audit skipped
codes are proven missing. Demo exit 0 if all scenarios generated/analyzed as expected;
expected partial results are not demo failures. Write demo-summary.csv.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Completed; may have partial/non_monotonic report unless --strict |
| 1 | Filesystem I/O or report-generation failure |
| 2 | CLI/config/CSV/synthesis validation error or finite arithmetic failure |
| 3 | --strict quality failure, with successfully written report |

Define this mapping centrally. On code 2 no analyze output directory is created.
Nonempty out directory is rejected before report writing with code 1. Empty output
folders supplied by users are accepted. Avoid creating outputs inside input folders
if it would violate emptiness. Source must never be overwritten.

## Exact command cases to exercise

```bash
cargo run -- --help
cargo run -- analyze fixtures/bow-hand-3bit.csv --bits 3 --vmin 0 --vmax 8 --out results/hand
cargo run -- analyze fixtures/missing-3bit.csv --bits 3 --vmin 0 --vmax 8 --out results/missing --strict
# Previous command returns 3 after writing a partial report.
cargo run -- analyze fixtures/reversal-3bit.csv --bits 3 --vmin 0 --vmax 8 --out results/reversal
cargo run -- analyze fixtures/invalid/non-increasing.csv --bits 3 --vmin 0 --vmax 8 --out results/bad
# Previous command returns 2 and creates no results/bad.
cargo run -- synth --model perfect --bits 3 --vmin -4 --vmax 4 --out results/bipolar
```
