# Plot examples

These files are deterministic outputs from the fixed 8-bit demo configuration
(vmin=0, vmax=2.56 V, 64 samples per nominal LSB):

- perfect-transfer.svg — sampled perfect transfer and reference lines.
- bow-inl.svg — separated nominal total transition error and corrected INL panels.
- missing-code-dnl.svg — unresolved audit bins shown as gaps/markers, never as
  inferred DNL=-1 measurements.

Regenerate the full six-scenario set with:

~~~bash
cargo run --release -- demo --out results/demo
~~~
