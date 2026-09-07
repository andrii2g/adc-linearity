# Design decisions

1. **Three references.** "Ideal endpoint" is ambiguous: nominal ideal range and a line
   through measured endpoint transitions are different. CLI exposes nominal, endpoint
   and best-fit separately; all is the default.
2. **Floor quantizer.** T_k=vmin+k*q is the explicit v1 convention. Other ADCs may place
   first transition at half an LSB. Supporting a transition phase flag is future work;
   do not silently compare a rounded quantizer against this floor reference.
3. **LSB normalization.** Reference DNL and INL use that line's b so discrete integration
   is valid. Export nominal DNL separately for comparison. Some tools keep nominal q
   after detrending; their values are not numerically interchangeable with ours.
4. **Two meanings of offset.** Generator shift/intercept shift are separate from first
   transition error. Public calibration reports the latter and input-span gain error.
5. **No inferred zero-width bins.** Bracketed jumps remain unresolved. Synthetic ground
   truth can have zero widths; CSV audit cannot prove equality at finite resolution.
6. **Reversals invalidate analysis.** No isotonic regression, sorting or averaging to
   make a reversing sweep look monotonic. Diagnostic report still succeeds unless strict.
7. **Bounds are limited.** Width and nominal bounds account for input sampling only.
   Fitted line uncertainty, noise, source accuracy and timing are outside this MVP.
8. **Streaming resolution statistics.** The earlier concept mentioned median input step.
   MVP uses exact streaming min/mean/max to keep O(M) memory; no hidden O(N) sample list.
9. **Bounded size.** Bits 2..16 and 5M synthetic samples cap accidental resource use.
   These are implementation limits, not claims about physical ADC resolution.
10. **No automatic spec pass/fail.** strict checks data completeness/monotonicity only.
    Absolute numerical limits require a later explicit comparison feature.
11. **Cross-platform repeatability.** Same platform/toolchain output is deterministic;
    libm sine and decimal rounding may differ in last bits across platforms.
12. **Safe local outputs.** New or empty directories only; no --force in MVP.
13. **License undecided.** Do not assign rights or a license for the user automatically.

Future candidates, kept out of MVP: descending ramps, repeated/noisy sweeps with
transition probability fitting, uncertainty budgets, code-density analysis with known
stimulus distribution, threshold phase support, comparison reports, calibration export.
