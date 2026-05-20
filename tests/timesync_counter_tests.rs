use impatience::timesync::{Counter8, Counter24, Counter64};

#[test]
fn counter_masking() {
    assert_eq!(Counter24::new(0x00ff_ffff).value, 0x00ff_ffff);
    assert_eq!(Counter24::new(0x01ff_ffff).value, 0x00ff_ffff);
}

#[test]
fn counter_wrapping() {
    let a = Counter24::new(0x00ff_ffff);
    let b = Counter24::new(1);
    let c = a + b;
    assert_eq!(c.value, 0);
}

#[test]
fn counter_circular_compare() {
    let a = Counter24::new(0);
    let b = Counter24::new(1);
    assert!(a.circ_lt(b));
    assert!(b.circ_gt(a));

    let max = Counter24::new(Counter24::MASK);
    assert!(max.circ_lt(a)); // rollover: max is just before 0
}

#[test]
fn expand_from_truncated_24_to_64() {
    let recent = Counter64::new(0x1_0000);
    let smaller = Counter24::new(0xff);
    let expanded = Counter64::expand_from_truncated(recent, smaller);
    assert_eq!(expanded.value, 0xff);

    let recent = Counter64::new(0x1_0000_0000);
    let smaller = Counter24::new(0xff);
    let expanded = Counter64::expand_from_truncated(recent, smaller);
    assert_eq!(expanded.value, 0x1_0000_00ff);
}

#[test]
fn expand_from_truncated_8_to_64() {
    let recent = Counter64::new(0x100);
    let smaller = Counter8::new(0xff);
    let expanded = Counter64::expand_from_truncated(recent, smaller);
    assert_eq!(expanded.value, 0xff);
}
