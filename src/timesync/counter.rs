//! Fixed-bit-width counters with rollover-safe arithmetic and comparison.

/// Generate a counter newtype with `BITS` data bits backed by `$base`.
macro_rules! counter {
    ($name:ident, $base:ty, $bits:expr) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name {
            pub value: $base,
        }

        impl $name {
            pub const BITS: u32 = $bits;
            pub const MASK: $base = <$base>::MAX >> (<$base>::BITS - $bits);
            pub const MSB: $base = (1 as $base) << ($bits - 1);

            #[inline]
            pub const fn new(value: $base) -> Self {
                Self {
                    value: value & Self::MASK,
                }
            }

            #[inline]
            pub const fn to_unsigned(self) -> $base {
                self.value
            }

            #[inline]
            pub const fn wrapping_add(self, other: Self) -> Self {
                Self::new(self.value.wrapping_add(other.value))
            }

            #[inline]
            pub const fn wrapping_sub(self, other: Self) -> Self {
                Self::new(self.value.wrapping_sub(other.value))
            }

            /// Circular less-than: `self` is considered smaller than `other` if
            /// the forward distance from `self` to `other` is more than half the
            /// counter space.
            #[inline]
            pub const fn circ_lt(self, other: Self) -> bool {
                let d = self.wrapping_sub(other).value;
                d >= Self::MSB
            }

            #[inline]
            pub const fn circ_gt(self, other: Self) -> bool {
                other.circ_lt(self)
            }

            #[inline]
            pub const fn circ_le(self, other: Self) -> bool {
                !self.circ_gt(other)
            }

            #[inline]
            pub const fn circ_ge(self, other: Self) -> bool {
                !self.circ_lt(other)
            }

            /// Expand a smaller counter into this larger counter type using a
            /// recent reference value and an optional bias.
            #[inline]
            pub fn expand_from_truncated_with_bias<Smaller>(
                recent: Self,
                smaller: Smaller,
                bias: i64,
            ) -> Self
            where
                Smaller: CounterTrait,
            {
                assert!(
                    (Smaller::bits() as u32) < Self::BITS,
                    "smaller type must have fewer bits"
                );

                let smaller_mask: u64 = Smaller::mask_u64();
                let smaller_msb: u64 = Smaller::msb_u64();

                let smaller_unsigned = smaller.to_u64();
                let high = recent.value & !(smaller_mask as $base);
                let mut result = Self::new(high | (smaller_unsigned as $base));

                let recent_low = (recent.value & (smaller_mask as $base)) as i64;
                let smaller_val = smaller_unsigned as i64;

                if recent_low < smaller_val {
                    let abs_diff = smaller_val - recent_low;
                    let threshold = (smaller_msb as i64) - bias;
                    if abs_diff >= threshold {
                        result = Self::new(
                            result.value.wrapping_sub((smaller_msb << 1) as $base),
                        );
                    }
                } else {
                    let abs_diff = recent_low - smaller_val;
                    let threshold = (smaller_msb as i64) + bias;
                    if abs_diff > threshold {
                        result = Self::new(
                            result.value.wrapping_add((smaller_msb << 1) as $base),
                        );
                    }
                }

                result
            }

            #[inline]
            pub fn expand_from_truncated<Smaller>(recent: Self, smaller: Smaller) -> Self
            where
                Smaller: CounterTrait,
            {
                Self::expand_from_truncated_with_bias(recent, smaller, 0)
            }
        }

        impl From<$base> for $name {
            #[inline]
            fn from(v: $base) -> Self {
                Self::new(v)
            }
        }

        impl core::ops::Add for $name {
            type Output = Self;
            #[inline]
            fn add(self, rhs: Self) -> Self {
                self.wrapping_add(rhs)
            }
        }

        impl core::ops::Sub for $name {
            type Output = Self;
            #[inline]
            fn sub(self, rhs: Self) -> Self {
                self.wrapping_sub(rhs)
            }
        }
    };
}

/// Trait exposing common counter metadata so generic code can work across
/// differently-sized types without casting associated types directly.
pub trait CounterTrait: Copy {
    fn bits() -> u32;
    fn mask_u64() -> u64;
    fn msb_u64() -> u64;
    fn to_u64(self) -> u64;
}

counter!(Counter8, u8, 8);
counter!(Counter16, u16, 16);
counter!(Counter23, u32, 23);
counter!(Counter24, u32, 24);
counter!(Counter64, u64, 64);

impl CounterTrait for Counter8 {
    #[inline]
    fn bits() -> u32 {
        Self::BITS
    }
    #[inline]
    fn mask_u64() -> u64 {
        Self::MASK as u64
    }
    #[inline]
    fn msb_u64() -> u64 {
        Self::MSB as u64
    }
    #[inline]
    fn to_u64(self) -> u64 {
        self.value as u64
    }
}

impl CounterTrait for Counter16 {
    #[inline]
    fn bits() -> u32 {
        Self::BITS
    }
    #[inline]
    fn mask_u64() -> u64 {
        Self::MASK as u64
    }
    #[inline]
    fn msb_u64() -> u64 {
        Self::MSB as u64
    }
    #[inline]
    fn to_u64(self) -> u64 {
        self.value as u64
    }
}

impl CounterTrait for Counter23 {
    #[inline]
    fn bits() -> u32 {
        Self::BITS
    }
    #[inline]
    fn mask_u64() -> u64 {
        Self::MASK as u64
    }
    #[inline]
    fn msb_u64() -> u64 {
        Self::MSB as u64
    }
    #[inline]
    fn to_u64(self) -> u64 {
        self.value as u64
    }
}

impl CounterTrait for Counter24 {
    #[inline]
    fn bits() -> u32 {
        Self::BITS
    }
    #[inline]
    fn mask_u64() -> u64 {
        Self::MASK as u64
    }
    #[inline]
    fn msb_u64() -> u64 {
        Self::MSB as u64
    }
    #[inline]
    fn to_u64(self) -> u64 {
        self.value as u64
    }
}

impl CounterTrait for Counter64 {
    #[inline]
    fn bits() -> u32 {
        Self::BITS
    }
    #[inline]
    fn mask_u64() -> u64 {
        Self::MASK
    }
    #[inline]
    fn msb_u64() -> u64 {
        Self::MSB
    }
    #[inline]
    fn to_u64(self) -> u64 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
