# Background and convention notes

Background reference consulted while designing the project:

- [Analog Devices: INL/DNL Measurements for High-Speed ADCs](https://www.analog.com/en/resources/technical-articles/inldnl-measurements-for-types-of-highspeed-adcs.html)
  explains code-width DNL and why straight-line reference selection affects INL.

This pack is not an implementation of a claimed IEEE measurement standard or a
vendor datasheet test procedure. Its floor-quantizer phase, unweighted OLS transition
fit, reference-slope LSB normalization, endpoint offset definition, partial-data policy
and sampling-bound rules are explicit project decisions. Use docs/MATH.md as the
computational contract. Always compare conventions before comparing reported values
with a device datasheet. In particular, wide positive DNL alone does not establish
missing codes; an idealized zero-width bin has DNL=-1.
