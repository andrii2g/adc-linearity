use adc_linearity_audit::config::AuditConfig;

#[test]
fn nominal_lsb_uses_number_of_levels() {
    let config = AuditConfig { bits: 3, vmin_v: 0.0, vmax_v: 8.0 };
    let derived = config.validate().expect("valid fixture config");
    assert_eq!(derived.levels, 8);
    assert_eq!(derived.nominal_lsb_v, 1.0);
}

#[test]
fn rejects_unrepresentable_nominal_transitions() {
    let config = AuditConfig { bits: 16, vmin_v: 1e16, vmax_v: 1e16 + 8.0 };
    assert!(config.validate().is_err());
}

#[test]
fn rejects_nonfinite_range_and_bad_bits() {
    for config in [
        AuditConfig { bits: 1, vmin_v: 0.0, vmax_v: 8.0 },
        AuditConfig { bits: 17, vmin_v: 0.0, vmax_v: 8.0 },
        AuditConfig { bits: 3, vmin_v: 0.0, vmax_v: f64::NAN },
        AuditConfig { bits: 3, vmin_v: 8.0, vmax_v: 0.0 },
    ] {
        assert!(config.validate().is_err());
    }
}
