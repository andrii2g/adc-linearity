# SVG specification

Generate self-contained SVG with viewBox, width/height, title and desc. No external
fonts, JavaScript, foreignObject or raster images. Escape &,<,>,quotes in user-derived
labels. Use background #ffffff, text #17202a, grid #d9e1e8 and minimum font size 16px.
Use differing line styles plus colors (nominal #0072b2, endpoint #d55e00, best-fit
#009e73). Do not rely on color alone. Put source basename, bits, status and coverage
in title/subtitle. Footer: "Static sweep estimate; sampling bounds exclude source error."

Default canvas 1280x800 with 100px left, 40px right, 100px top and 140px bottom margins.
If multiple panels or warning blocks need room, increase height rather than overlap.
Axes need unit labels and at least 4 readable ticks. Keep text outside plotting clip.
Use a common scale within a plot when comparing references; auto-expand an all-zero
range symmetrically (e.g. +/-0.1 LSB). Include zero baseline. Tick precision follows
range magnitude, never long floating-point tails. Empty plots display a reason.

## transfer.svg

x: analogue input in V; y: ADC code. Show observed samples/staircase with a legend
"sampled transfer"; sample endpoints are not exact measured threshold locations.
Show ideal reference line code=(x-vmin)/q, and available fitted inverse lines
code=(x-a)/b, clipped to code range. Label these as transition reference lines,
not the full ideal staircase. Include bracket markers for resolved transitions in
small cases; suppress overcrowded individual labels. Do not draw estimated unique
transition points for unresolved jumps.

Preserve code reversals in the plot. Use raw input sample buckets indexed by nominal
voltage position (clamped to first/last bucket outside nominal range). Retain first,
last, min-code and max-code sample per each of 1024 buckets, retaining their record
indices. Deduplicate and sort retained points by record; use actual retained x values
for axis extent. This preserves observed extrema and remains bounded. A code-reversal
warning includes total event count and the first event's values. Never render corrected
reference lines for invalidated analysis. Diagnostic event markers may be bounded.

## dnl.svg

x: interior code 1..M-2, y: DNL in labeled LSB basis. Plot selected reference series;
legend makes reference-slope versus nominal LSB explicit. Code 0 and M-1 excluded.
Mark -1 reference line as "zero-width truth boundary"; explain unresolved audit bins
have no measured DNL. Do not draw missing candidates as measured -1 bars.
Use bars for <=256 codes, line/vertical envelopes for larger arrays. Use point values
only on resolved bins. Nominal sampling-width bound whiskers are optional on sparse
plots; fitted error bands are excluded. Unresolved regions get pale hatching/markers
with a legend; unobserved regions stay blank with an untested note.

## inl.svg

x: transition index 1..M-1. Selected nominal line is labeled "total transition error";
endpoint and best-fit lines "INL". In all mode use two vertically stacked panels:
upper nominal error, lower endpoint/best-fit INL, with individual axis scales and
labels. This prevents a large gain error from hiding corrected linearity. A single
selected reference can use one panel. Gaps split polylines. Never bridge unresolved
or unobserved thresholds. Add counts and maxima only for evaluated values.
Cumulative values should coincide with direct residuals per run; default show direct
residual series and note local cumulative reconstruction in desc (no duplicate lines).

## Bounded rendering

At most about 2048 horizontal data buckets per panel. Downsample each contiguous
valid run using min/max envelopes in each pixel bucket, preserving original index
order and extrema. Null slots must split runs BEFORE reducing. Do not join runs merely
because they share a pixel. With many tiny runs, render unconnected extrema strokes
per pixel plus gap markers; never create fictitious continuity. Raw metrics never
use plot-downsampled data. XML element counts may scale with M for gap markers (M<=65536)
but avoid a giant polygon or a hidden O(N) raw sweep representation.

## Visual QA

Inspect perfect, bow, missing, affine and reversal SVGs. Check tick labels, legends,
zero lines, partial warnings, missing gaps, all-zero scaling, negative ranges, input
basename escaping and large-code decimation. XML parsing is necessary but cannot
prove readability; render/open images when the environment supports it. Keep three
representative generated SVGs under examples/ after implementation, with source config.
