# Report serialization contract v1

JSON uses snake_case, finite JSON numbers and explicit nulls for unavailable values.
No NaN/Infinity strings, automatic zero defaults or omitted unavailable numeric keys.
Integers are JSON integers. Vectors are in ascending code order. Ties for extrema use
the lowest index. Keep output independent of full source path and timestamp.

`fixtures/example-summary.json` is a complete illustrative report for perfect-3bit,
calculated from its analytic expectations. It is a schema example, not Rust execution
output. docs/REPORT_FORMAT.md is authoritative for additional cases.

## summary.json top-level fields

| Key | Type and meaning |
| --- | --- |
| schema_version | integer 1 |
| source | basename string |
| config | bits,vmin_v,vmax_v,levels,nominal_lsb_v,quantizer="floor" |
| status | valid,partial,non_monotonic |
| sample_count | integer |
| coverage | observed_code_count,min_observed_code,max_observed_code,resolved_transition_count,covered_transition_count,expected_transition_count,resolved_width_count,expected_width_count |
| quality | step_count,step_min_v,step_mean_v,step_max_v,step_min_lsb,step_mean_lsb,step_max_lsb,max_bracket_v,max_bracket_lsb |
| calibration | See below |
| references | Requested reference objects, including unavailable ones |
| transitions | M-1 transition objects |
| codes | M code objects; includes sample counts and saturation rows |
| diagnostics | Counts, bounded events, missing candidates, untested codes, warnings |
| assumptions | Array of concise fixed explanatory strings |

max_bracket is null when no first-crossing bracket exists. After a reversal it only
summarizes diagnostic brackets; its interpretation follows the status. Step metrics
always exist because valid input has at least two rows. covered count may reflect
first crossings before reversal only; add warning diagnostic_brackets_incomplete.

## Calibration object

availability: available|unavailable; reason: null or machine-readable string.
Numeric fields (all null if unavailable): offset_v,offset_lsb,gain_span_error_v,
gain_span_error_lsb,gain_span_error_percent. basis="first_and_last_internal_transition";
gain_convention="input_span". Reasons: non_monotonic_sweep,endpoint_not_resolved,
numerical_failure. Calibration independence from reference selection is intentional.

## Reference object

name: nominal|endpoint|best_fit; availability: available|unavailable; reason:null/string;
line:null or {a_v,b_v_per_code,anchor_k,anchor_v,intercept_shift_v};
lsb_basis: nominal|reference_slope; display_quantity: total_transition_error|inl;
coverage:{scope:full_range|partial,evaluated_transition_count,evaluated_width_count,
expected_transition_count,expected_width_count}; dnl_summary:null/object;
inl_summary:null/object; transition_metrics: M-1 entries; code_metrics: M-2 entries.

Unavailable lines still have arrays of all expected indices with null numeric values
and false run_start; coverage evaluated counts are zero, scope partial. Reason values:
non_monotonic_sweep,no_resolved_transitions,endpoint_not_resolved,
insufficient_resolved_transitions,numerical_failure. Do not silently drop the reference.

Transition metric fields: k,inl_lsb,cumulative_inl_lsb,cumulative_run_id,
cumulative_run_start. Run ID null when residual null. Nominal inl_lsb key is kept for
uniform machine processing; display_quantity determines its user-facing label.
Code metric fields: code,dnl_lsb. Exact fixed normalization is on its parent reference.

DNL summary fields: count,min_lsb,min_code,max_lsb,max_code,max_abs_lsb,max_abs_code.
INL summary fields: count,min_lsb,min_k,max_lsb,max_k,max_abs_lsb,max_abs_k,rms_lsb.
Reference scope is full_range only if all expected transitions AND widths evaluated.

## Transition object

k,status:resolved|unresolved|unobserved|invalidated,
bracket:null or {lower_v,upper_v,lower_record,upper_record},
estimate_v,half_bracket_v,nominal_error_lower_lsb,nominal_error_upper_lsb.
Bounds may exist for bracketed unresolved thresholds, but not invalidated thresholds.
For a reversed sweep all transition statuses become invalidated; retained brackets
are diagnostic-only. An unobserved slot's bracket remains null. No estimate is zero
unless a resolved midpoint actually equals zero.

## Code object

code,samples,observation_status:observed|missing_candidate|untested,
width_status:saturation|resolved|unresolved|unobserved|invalidated,
width_v,width_lower_v,width_upper_v,nominal_dnl_lsb,
nominal_dnl_lower_lsb,nominal_dnl_upper_lsb.
End codes always have width_status saturation and null width/DNL values, including on
reversed sweeps. Interior invalidated widths take precedence over all other statuses.
Observation statuses describe appearance only; on reversal candidates are explicitly
marked non-diagnostic through diagnostics.candidates_interpretable=false.

## Diagnostics

jump_count,reversal_count,jumps_omitted,reversals_omitted are integer event counts.
jumps,reversals are arrays of {previous_record,current_record,previous_v,current_v,
previous_code,current_code}. Maximum 1000 entries per array.
missing_code_candidates,untested_codes are ascending integer arrays.
candidates_interpretable:bool (false on non_monotonic).
warnings: unique machine-readable strings in a fixed order:
coarse_sampling,partial_coverage,code_reversal,diagnostic_brackets_incomplete.
Only include warnings whose condition holds. coarse_sampling means max_step_lsb>=1.

Fixed assumptions should state: increasing deterministic sweep; floor transition
convention; sampling bounds exclude input-source error/noise; fitted-line bounds not
estimated; unobserved codes are not proven missing; saturation widths excluded.

## CSV reports

Use csv Writer; output header even when rows contain only nulls. Use round-trip f64
formatting, LF newlines and invariant decimal point. Null numeric values are empty
fields; never "null", NaN or -999 sentinels. Booleans true/false.

transitions.csv base header:
k,status,lower_v,upper_v,lower_record,upper_record,estimate_v,half_bracket_v,nominal_error_lower_lsb,nominal_error_upper_lsb
Append four columns per requested reference r in stable order:
{r}_inl_lsb,{r}_cumulative_inl_lsb,{r}_run_id,{r}_run_start

codes.csv base header:
code,samples,observation_status,width_status,width_v,width_lower_v,width_upper_v,nominal_dnl_lsb,nominal_dnl_lower_lsb,nominal_dnl_upper_lsb
Append {r}_dnl_lsb for each requested reference r. End-code r_dnl fields are empty.

## Demo comparison CSV

scenario,status,resolved_transitions,expected_transitions,missing_candidate_count,
truth_missing_code_count,endpoint_max_abs_inl_lsb,best_fit_max_abs_inl_lsb,
nominal_max_abs_dnl_lsb,offset_lsb,gain_span_error_percent

The header is a single CSV record (line breaks above are only for readability).
Truth counts come only from synth truth, not the input analyzer. Missing candidate
and truth missing code count must stay separate even when they happen to agree.

## Numeric failure policy

Before serialization, validate every produced numeric field is finite. A computation
that should be available but overflows is an error, not a null value pretending to
mean missing data. Unavailable reference fits can carry numerical_failure explicitly.
Check summary sums and line/residual evaluation too. serde_json is not the validator.
