# Deterministic synthetic ADC contract

The generator constructs exact model thresholds first and produces codes by threshold
comparison. It does not ask the analyzer to manufacture expected values.

## Base thresholds

M=2^B, q=(vmax-vmin)/M, k=1..M-1, u=(k-1)/(M-2).
perfect: T_base(k)=vmin+k*q.
bow: T_base(k)=vmin+k*q + A*q*4*u*(1-u).
periodic: T_base(k)=vmin+k*q + A*q*sin(2*pi*p*u).
For periodic, assign displacement at k=1 and k=M-1 exactly zero (avoid a sin(2*pi*p)
roundoff perturbing endpoints). Validate every adjacent threshold difference >0.
A is a peak threshold-displacement parameter, not a guaranteed fitted-INL maximum.
Changing p changes local DNL severity at fixed displacement amplitude.

missing-code: start with perfect base thresholds, pick interior code c, then set
T_base(c+1)=T_base(c). Only this intentional equal pair is allowed. Do not remove
a threshold from the vector: all M-1 thresholds remain indexed.
For interior c away from the high edge this also widens the next code to 2q. If
c=M-2, the next bin is saturation and has no interior DNL metric.

## Optional affine transform

Let offset_v=offset_lsb*q and s=1+span_error_percent/100 >0.
T(k)=vmin+offset_v+s*(T_base(k)-vmin).
The generator parameter offset_lsb is a shift at vmin, not endpoint-reported offset.
For a perfect base, reported first-transition offset_lsb equals
input_offset_lsb+(s-1). Document this in help and truth.

Validate all thresholds finite and nondecreasing; strictly increasing unless intentional
missing-code equality. Check actual representable widths, not merely a symbolic
amplitude limit. Reject degenerate/overflowed nominal range and excessive sample counts.
Do not sort thresholds to repair a physically different model.

## Sampling

Define x_start=min(vmin,T_1-q), x_end=max(vmax,T_(M-1)+q).
Desired step h=q/samples_per_lsb. Define intervals=ceil((x_end-x_start)/h).
Require intervals>=1 and intervals+1<=5,000,000. Samples i=0..intervals:
x_i=x_start+(x_end-x_start)*(i/intervals), with first/last explicitly assigned.
This includes both outer codes and ensures maximum mathematical step <=h. Validate
floating-point samples remain strictly increasing; no duplicate rounding artifacts.
Avoid repeated x+=h accumulation. Round-trip CSV f64 output retains distinctions.

Code(x) is the number of thresholds <=x. Implement via slice.partition_point or a
monotonic cursor. Thus threshold equality skips a zero-width code naturally, and an
input exactly on one or more thresholds belongs to the code after all those thresholds.
Codes always remain in 0..M-1 with exactly M-1 threshold elements.

## truth.json

schema_version=1,kind="synthetic_truth", config, model name, effective model parameters,
sample_count,sample_start_v,sample_end_v,max_nominal_step_v,thresholds_v (M-1 entries),
interior_widths_v (M-2 entries),true_missing_codes (exact equal pairs),
nominal_dnl_lsb (M-2),endpoint_inl_lsb (M-1),endpoint_line,
expected_calibration. Add generation="threshold_model"; decimated coarse scenario
adds sampling="every_256th_plus_final" and retained sample_count.

Truth computes widths directly from thresholds, including width=0 and DNL=-1 when
applicable. Analyze never imports this file. For general nonlinear truth, endpoint
line uses actual extreme thresholds even when one is a repeated threshold. Model
truth can establish transitions unavailable in a finite observational audit.
Truth arrays are finite. Do not create a misleading "confirmed_missing" field in
ordinary audit summary by copying this truth.

## Expected demo interpretation

- Perfect: corrected DNL/INL within sweep resolution, no candidates.
- Bow: smooth endpoint INL peak near 3 LSB, best-fit removes average/linear component.
- Periodic: oscillating INL and alternating width changes.
- Missing: true code 128 absent; audit marks transitions 128 and 129 unresolved and
  candidate code 128. Audit DNL for that bin is null, while truth DNL is -1.
- Affine: nominal error accumulates across codes; corrected INL near zero; calibration
  recovers offset and span within step-derived tolerance.
- Coarse-perfect: many candidates despite no true missing codes. This is the educational
  counterexample that prevents the tool from equating absence with physical failure.
