# Behavioral specification

## Purpose and boundaries

Analyze one deterministic, increasing-input ADC ramp. Quantify local bin widths and
static transition deviations from specified straight lines. Provide clear evidence
of coverage and resolution. Read CSV only; do not capture hardware or modify data.
No ENOB, noise statistics, FFT, dynamic stimulus model, automatic calibration,
polynomial correction, hysteresis model, or code-density method in this version.

## Configuration

Bits: integer 2..16 inclusive. M=1usize<<bits. Range endpoints are finite f64 with
vmax>vmin; their difference and q=(vmax-vmin)/M must be finite, positive and representable.
Require each successive ideal internal transition to be strictly increasing in f64;
reject a nominal range lost to floating-point precision. Voltages are volts. Codes
are integers 0..M-1. CSV values may extend below vmin or above vmax to bracket real
offset/gain shifts; this does not change the nominal reference or make data invalid.
Store configuration, requested reference names and quantizer convention in reports.

## CSV rules

UTF-8, comma-separated, exactly the two named columns input_v and code (either order).
Permit an optional leading UTF-8 BOM and whitespace around headers/field values.
Reject missing/duplicate/extra headers, ragged records, blank fields, comments,
non-finite voltages, fractional or negative codes, oversized integers, code overflow,
and rows with malformed quoting. CSV quoting and CRLF are handled by the csv crate.
Blank physical lines may be ignored by csv; record numbers refer to logical CSV
records (header=1). Blank quoted fields are errors. Require at least two data records.
Analogue values must be strictly increasing in original record order. Do not sort.

## Four distinct observations

1. Seen code: appeared in at least one row; not evidence of its full width.
2. Covered transition: some adjacent increasing-input samples bracket crossing k.
3. Resolved transition: the crossing was a single-code step k-1 -> k.
4. Unresolved transition: it occurred inside a jump over multiple codes.

Unseen transitions before/after the sweep have status unobserved. A covered transition
bracket is retained even if unresolved; midpoint estimates exist only for resolved
transitions. A finite step cannot prove a missing code. Categorize unobserved codes
inside [min_observed_code,max_observed_code] as missing-code candidates; unobserved
codes outside that interval are untested. End codes may be untested, not missing.
Zero exact width is supported in synthetic truth but never inferred from a jump.

## Sweep-level status

| status | Rule | Consequence |
| --- | --- | --- |
| valid | No code reversal, all M-1 transitions resolved | Full-range estimates available |
| partial | No reversal, but some transitions unresolved or unobserved | Local available metrics only |
| non_monotonic | At least one code decrease | All references and calibration values unavailable |

A constant-code sweep is partial with no linearity estimates. Sparse full-range jumps
are partial, not valid. A code reversal is a measured sequence property; noise can
produce it. Never state that hardware is intrinsically nonmonotonic based on this alone.
Malformed CSV/config is a command error, not a serialized audit status.

For non_monotonic, continue scanning to count jumps, reversals and seen codes. Retain
only the first crossing bracket for each transition as diagnostic context; invalidate
all transition point estimates, widths and reference results at finalization. The
transfer plot still displays observed data. First-crossing brackets are not claimed
to describe unique physical transitions. Code candidate lists describe observations
only and are marked non-diagnostic in this status.

## Availability and coverage

Nominal reference is available whenever at least one resolved transition exists.
Endpoint reference needs resolved transitions 1 and M-1. Best fit needs at least
three resolved transitions at distinct k, a finite positive fitted slope and stable
arithmetic. Three is a quality floor: two points would trivially define a line.
Endpoint calibration is computed regardless of requested reference list, provided
its two endpoint transitions are resolved and the sweep has no reversals.

An available reference can have partial metrics: report evaluated transition and
width counts, expected counts M-1 and M-2, and scope full_range or partial.
A reference can be available with zero valid DNL widths. Its DNL summary is null.
No pass/fail assessment against an unspecified ADC datasheet. No extrapolated full-range
maximum when coverage is incomplete. Summary maximum means maximum among evaluated bins.

## Data caps and determinism

Store M code counters and M-1 transition slots. Retain at most 1000 jump records and
1000 reversal records separately, while counting all events with checked u64 counters.
Record the number omitted. Diagnostics sorted by record occurrence; codes ascending;
references in nominal, endpoint, best_fit order after filtering selection.
Retain <=4 points per 1024 voltage plot buckets, plus first/last input point.
Step stats: min, max, online mean, count; no exact median. Plot state stays bounded.
No timestamps, absolute source paths, platform-specific separators or random seeds
are needed in default machine-readable outputs. Repeated commands on the same inputs
produce numerically identical output on one toolchain/platform. Cross-platform sin()
last-bit differences are allowed in synthesized data; compare numerically, not bytewise.

## Deliverables per command

Analyze: summary.json, transitions.csv, codes.csv, transfer.svg, dnl.svg, inl.svg.
Synth: sweep.csv, truth.json (contains model/config, thresholds and exact model metrics).
Demo: six deterministic subfolders, each with input/ and audit/; demo-summary.csv.
Use new/empty output directory only; explain failed/partial writes on stderr and exit 1.
Create no artifact on argument/config/CSV validation failure. Do parsing/analysis before
creating the analyze output directory. On report I/O failure, partial files may remain;
never claim the command succeeded. Files must always be finite numeric text or null.
