# Mathematical contract

All definitions below are project conventions. Do not silently switch to a datasheet's
half-LSB transition convention or a minimax best straight line. Indexing is explicit.

## 1. Quantization and indexing

B bits, M=2^B levels, code c in 0..M-1. Nominal q=(vmax-vmin)/M volts/LSB.
Floor quantizer c(x)=clamp(floor((x-vmin)/q),0,M-1).
Internal threshold T_k is the voltage entering code k, k=1..M-1.
Ideal T_k=vmin+k*q. At a threshold, the output is the higher code.
Thresholds in a monotone model are nondecreasing; equal adjacent thresholds delete a
code. A threshold model cannot produce a true code reversal; reversal fixtures must
be constructed as observations separately.

In Vec storage, transition index i corresponds to k=i+1. Interior code k has width
W_k=T_(k+1)-T_k, k=1..M-2, using vector elements k and k-1. There are M-2 widths.
Code 0 and code M-1 are saturation bins and always excluded from width/DNL metrics.
No division by M-1 when constructing the nominal LSB.

## 2. Brackets and widths

For adjacent observations (x_l,c_l),(x_u,c_u), if c_u>c_l, every k in
c_l+1..c_u is bracketed by [x_l,x_u]. The underlying threshold is actually in
(x_l,x_u] for a deterministic floor quantizer; closed bounds are a conservative
representation. Single-code step: midpoint T_hat=x_l+(x_u-x_l)/2 and
half_bracket=(x_u-x_l)/2. Multi-code step: retain brackets, point estimate null.

Only two resolved adjacent transitions produce a point width:
W_hat_k=T_hat_(k+1)-T_hat_k.
The conservative input-voltage width bounds are:
W_low=max(0,L_(k+1)-U_k)
W_high=U_(k+1)-L_k.
Reject an internally inconsistent negative upper bound as an implementation error.
Width bounds can be reported when both transitions have brackets, even if either is
unresolved; their point width and point DNL stay null. If either bracket is absent,
all width bounds are null. On a non_monotonic sweep, all width fields are null.

These bounds neglect input-source error and are not statistical confidence intervals.
For resolved transitions separated by step h, each midpoint error is <=h/2, and a
width error is bounded by the sum of the two neighboring half-bracket widths.

## 3. References R_k=a+b*k

Nominal: a=vmin, b=q.
Endpoint: b=(T_hat_(M-1)-T_hat_1)/(M-2), a=T_hat_1-b.
Best fit: ordinary unweighted least squares, voltage dependent on integer transition
index, over resolved transitions only (at least three). It is not a fit to every
sample or to code centers and is not minimax.

With k_bar and t_bar as means:
Skk=sum((k-k_bar)^2)
Skt=sum((k-k_bar)*(T_hat_k-t_bar))
b=Skt/Skk; a=t_bar-b*k_bar.
Use centered accumulation (and preferably compensated summation), require Skk>0,
b>0 and finite outputs. For residual evaluation, prefer the centered expression
(T_hat_k-t_bar)-b*(k-k_bar) for best fit. Use an analogous anchored expression for
endpoint residuals to reduce cancellation. Store a,b along with anchor values.
Never use raw normal equations involving a subtraction of two large nearly equal sums.

## 4. DNL and INL normalization

For a selected reference with positive slope b:
DNL_ref(k)=W_hat_k/b-1.
INL_ref(k)=(T_hat_k-R_k)/b.
Nominal code-width DNL W_hat_k/q-1 is always exported separately, even if nominal
is not among requested references. Label the nominal transition residual as total
transition error: it includes offset and gain as well as nonlinearity.

DNL_ref and INL_ref use the SAME b. Mixing W/q with residual/b breaks the identity.
For a contiguous run of resolved transitions:
INL_ref(k+1)-INL_ref(k)=DNL_ref(k),
INL_ref(k)=INL_ref(k_start)+sum(j=k_start..k-1,DNL_ref(j)).
Endpoint INL at 1 and M-1 is zero up to floating-point arithmetic. Best-fit INL at
1 is generally nonzero. Do not unconditionally initialize cumulative INL to zero.

At any unresolved/unobserved transition, close the cumulative run. At the next
resolved transition, initialize a NEW run using its direct residual and mark
cumulative_run_start=true. Never sum across a null width. Report direct and cumulative
values for all resolved transitions, with run IDs starting at 0 in code order.
A one-transition run has one residual and no increment. Re-anchoring gives local
consistency; it does not recover the omitted error across the gap.

## 5. Bounds associated with normalized quantities

Nominal reference has fixed q,a, so threshold residual bounds
[(L-R_k)/q,(U-R_k)/q] are conservative sampling bounds. DNL bounds are
[W_low/q-1,W_high/q-1]. For fitted references, the line depends on uncertain
thresholds. Bounds computed holding a,b fixed are CONDITIONAL only; this MVP exports
nominal bounds and width-voltage bounds, not fitted INL/DNL bounds. Document this
restriction in summary assumptions. Never label a fitted midpoint line as exact truth.

## 6. Offset and input-span gain error

Endpoint-based calibration (separate from choice of displayed reference):
offset_v=T_hat_1-(vmin+q)
offset_lsb=offset_v/q
measured_span=T_hat_(M-1)-T_hat_1
ideal_span=(M-2)*q
gain_span_error_v=measured_span-ideal_span
gain_span_error_lsb=gain_span_error_v/q
gain_span_error_percent=100*(measured_span/ideal_span-1).

Positive offset means entering the first nonzero code requires higher input.
Positive gain_span_error_percent means more volts per code, NOT higher codes/volt.
Optional sensitivity error = 100*(q/b_endpoint-1), but omit it from schema v1.
The endpoint offset is not the extrapolated intercept a-vmin: gain shifts the first
transition too. Do not conflate those values. Report fitted intercept shift only as
a-vmin in the reference object, named intercept_shift_v, not offset_v.
If calibration is unavailable, serialize status unavailable, reason and null fields.

## 7. Summaries

For each selected reference, report DNL min/max and max_abs, and INL min/max/max_abs.
Track code/index of extrema; tie goes to smallest code/index. DNL codes refer to
interior bins; INL indices refer to transitions. Count values used. RMS INL is
sqrt(sum(INL^2)/n) over evaluated transitions (not weighted by input sample density).
Use a scaled sum-of-squares algorithm if needed to avoid overflow; reject non-finite
computed values rather than write Infinity. Do not include unseen/saturation bins as
zero entries. With no values, summary is null. A one-value summary is defined.

## 8. Hand-computed example

B=3, vmin=0, vmax=8 -> M=8 and q=1. Transitions:
[1, 2, 3.25, 4.5, 5.25, 6, 7].
Widths for interior codes 1..6:
[1, 1.25, 1.25, 0.75, 0.75, 1].
Endpoint a=0,b=1, so DNL=[0,.25,.25,-.25,-.25,0] and
INL=[0,0,.25,.5,.25,0,0]. Running DNL sums reproduce those INLs.
For best fit, symmetry gives b=1,a=1/7. Best-fit INL is endpoint INL minus 1/7;
its first residual is -1/7, not zero. Peak absolute endpoint INL=.5;
peak absolute best-fit INL=5/14. This fixture catches wrong cumulative anchoring.

Affine example T_k=.25+1.125*k, q=1:
a_endpoint=.25, b_endpoint=1.125, offset_v=.375,
gain_span_error_v=.75, gain_span_error_percent=12.5.
Endpoint and best-fit DNL/INL are zero; nominal DNL=.125.
This catches confusing intercept shift (.25) with first-transition offset (.375).
