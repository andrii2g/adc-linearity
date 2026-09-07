# Algorithms and numerical behavior

## Streaming acquisition

Validate config first. Reader emits Sample {record,input_v,code}. Maintain previous
sample, checked u64 sample/step counters, seen_count[M], first and last point,
first-crossing transition slots[M-1], bounded diagnostics and transfer plot buckets.
For each pair:

1. Reject input_v <= previous.input_v; reject non-finite differences.
2. Update min/max and incremental mean of positive voltage steps.
3. If code increases: count a jump event only when delta_code>1; for each crossed k,
   fill its slot only if not already filled. Single increment -> resolved; multiple
   increment -> unresolved. Use checked increments and cap stored event examples.
4. If code decreases: increment reversal count, save the event if below cap, flag
   sweep non_monotonic. Continue reading to preserve diagnostics/counts.
5. If unchanged: update counts only; no new threshold information.

To preserve O(N+M) even for highly reversing data, once the first reversal is seen,
do not enumerate crossed thresholds on later increasing moves. Continue event counts
and plot points. Earlier slots remain diagnostic-only and finalization nulls all
estimates. Before the first reversal, each k is crossed at most once, so total threshold
work is O(M). This detail prevents adversarial O(N*M) behavior.

Avoid overflow in midpoint: x_l+(x_u-x_l)/2 after finite-difference validation.
For a later code increase on invalidated data, diagnostics retain the pair and delta
without per-code enumeration. Seen counts still update for every sample.

## Finalize observation data

Compute code coverage from seen_count. Derive interior missing candidates by a single
code-space pass. Sort nothing that changes observation order. Determine sweep status.
For non_monotonic clear all midpoint/width/residual estimates and reference/calibration
availability, retaining diagnostic statuses for slots. For monotonic data, resolved
threshold count and M-1 expected thresholds determine valid versus partial status.

Construct M-2 code rows. Point widths need two resolved transitions. Bounds need two
covered brackets. Keep status resolved, unresolved or unobserved consistently. Raw
nominal DNL bounds may exist for unresolved widths without a nominal point estimate.
Nominal residual bounds may likewise exist on a bracketed unresolved transition.

## Fit references

Build a slice/list of resolved transitions (bounded by M-1). Fit endpoint only if its
actual required endpoints are resolved; do not use the first/last available interior
point as a substitute. OLS uses only resolved transitions, in increasing k, no sample
weights. Reject non-finite/nonpositive line slopes; reference becomes unavailable with
reason numerical_failure. A computation overflow elsewhere becomes a command error.
Calibration has its own availability and is computed even with --reference best-fit.

For each selected available line, walk thresholds and widths once, emit residuals,
DNL and summary extrema. Conditional fitted uncertainty bands are intentionally absent.

## Cumulative INL runs

Track previous resolved index, cumulative value and run ID. When current k is exactly
previous k+1, add the corresponding DNL_ref(previous k). Otherwise increment run ID
and initialize from current direct INL. Store run_start=true on every new run. Keep a
scaled tolerance check of cumulative versus direct values in tests; Kahan summation
is useful on long runs. No artificial connector across a gap in SVG output.

## Event detection versus interpretation

Jump delta=1: ordinary transition, not an event warning.
Jump delta>1: unresolved threshold cluster and candidate codes in the skipped interval.
Reversal delta<0: violates monotonic deterministic sweep assumption. Counts are based
on adjacent input samples, not number of skipped code values. Repeated candidate codes
across invalidated data are listed once from the seen-code bitmap.
Do not treat DNL>+1 as proof of missing codes. A zero-width true bin has DNL=-1;
large positive DNL alone means a wide bin. Fitted/nominal normalization differs.

## Complexity and robustness

N rows, M codes, R<=3 references, D=1000 event cap, P=1024 plot buckets.
Parse/extract: O(N+M) time, O(M+D+P) memory. Analysis: O(R*M).
Synthesis: O(M+N) with a monotonic threshold cursor, or O(N log M) by binary search.
JSON may hold O(R*M) arrays; report sizes are intentionally bounded by B<=16.
Use buffered output. Account for count overflow and output path collisions.
Source CSV may be large; never read_to_string then parse all rows for convenience.

## Step-resolution reporting

Store count,min_v,mean_v,max_v and values divided by nominal q. Exactly N-1 steps.
Use online mean or compensated sum, with finite checks. A max step >=1 nominal LSB
adds coarse_sampling warning. No smaller step guarantees absence of missing codes:
actual bins may be narrower than nominal. Dense scans only reduce unresolved bounds.
