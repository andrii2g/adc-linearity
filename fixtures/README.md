# Fixture guide

All audit fixtures use bits=3, vmin=0, vmax=8. Read docs/TEST_PLAN.md and expected.json.
Dense fixture samples are dyadic and straddle exact thresholds symmetrically; their
midpoints can be checked exactly without floating-point tolerance. The affine sweep
intentionally extends beyond nominal vmax. bow-hand is a hand-designed symmetric
profile, not a sample of the quadratic synth model.

expected.json includes synthetic truth only to validate the fixtures and acceptance
suite. The production analyzer must never load it. true_missing_codes exists only
for the cases demonstrating known synthetic truth. Unknown fields in the expectations
are not implied values; tests assert the explicitly present oracle properties.

example-summary.json is an illustrative full schema for the perfect fixture, assembled
from its hand calculations. It is not output from a built Rust binary. CLI exit behavior
is documented in docs/CLI.md; invalid/ files should all exit 2 with no report output.
