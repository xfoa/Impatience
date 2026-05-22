//! Fixed-bit-width counters with rollover-safe arithmetic and comparison.

use core::convert::TryFrom;
use paste::paste;

/// Storage requirements for a [`Counter`].
pub trait CounterParams: Copy + Clone + core::fmt::Debug + Default + 'static {
    type Storage: CounterStorage;
    const BITS: u32;
    const MASK: Self::Storage;
    const MSB: Self::Storage;
}

/// Trait implemented by the primitive unsigned types that can back a [`Counter`].
pub trait CounterStorage:
    Copy
    + Clone
    + Default
    + PartialEq
    + Eq
    + PartialOrd
    + Ord
    + core::hash::Hash
    + core::fmt::Debug
    + Into<u64>
    + TryFrom<u64>
    + core::ops::Not<Output = Self>
    + core::ops::BitAnd<Output = Self>
    + core::ops::BitOr<Output = Self>
{
    fn wrapping_add(self, other: Self) -> Self;
    fn wrapping_sub(self, other: Self) -> Self;
}

macro_rules! impl_counter_storage {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl CounterStorage for $ty {
                fn wrapping_add(self, other: Self) -> Self {
                    <$ty>::wrapping_add(self, other)
                }
                fn wrapping_sub(self, other: Self) -> Self {
                    <$ty>::wrapping_sub(self, other)
                }
            }
        )+
    };
}

impl_counter_storage!(u8, u16, u32, u64);

macro_rules! counter_size {
    ($bits:literal,  $storage:ty) => {
        paste! {
            #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct [< P $bits >];

            impl CounterParams for [< P $bits >] {
                type Storage = $storage;
                const BITS: u32 = $bits;
                const MASK: $storage = <$storage>::MAX >> (<$storage>::BITS - $bits);
                const MSB: $storage = 1 << ($bits - 1);
            }

            pub type [< Counter $bits >] = Counter<[< P $bits >]>;
        }
    };
}

counter_size!(8,  u8);
counter_size!(16, u16);
counter_size!(23, u32);
counter_size!(24, u32);
counter_size!(64, u64);

/// Fixed-bit-width counter with rollover-safe arithmetic and comparison.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Counter<P: CounterParams> {
    pub value: P::Storage,
}

impl<P: CounterParams> Counter<P> {
    pub const BITS: u32 = P::BITS;
    pub const MASK: P::Storage = P::MASK;
    pub const MSB: P::Storage = P::MSB;

    #[inline]
    pub fn new(value: P::Storage) -> Self {
        Self {
            value: value & P::MASK,
        }
    }

    #[inline]
    pub fn to_unsigned(self) -> P::Storage {
        self.value
    }

    #[inline]
    pub fn wrapping_add(self, other: Self) -> Self {
        Self::new(self.value.wrapping_add(other.value))
    }

    #[inline]
    pub fn wrapping_sub(self, other: Self) -> Self {
        Self::new(self.value.wrapping_sub(other.value))
    }

    /// Circular less-than: `self` is considered smaller than `other` if
    /// the forward distance from `self` to `other` is more than half the
    /// counter space.
    #[inline]
    pub fn circ_lt(self, other: Self) -> bool {
        let d = self.wrapping_sub(other).value;
        d >= Self::MSB
    }

    #[inline]
    pub fn circ_gt(self, other: Self) -> bool {
        other.circ_lt(self)
    }

    #[inline]
    pub fn circ_le(self, other: Self) -> bool {
        !self.circ_gt(other)
    }

    #[inline]
    pub fn circ_ge(self, other: Self) -> bool {
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
            Smaller::bits() < Self::BITS,
            "smaller type must have fewer bits"
        );

        let smaller_mask: u64 = Smaller::mask_u64();
        let smaller_msb: u64 = Smaller::msb_u64();

        let smaller_unsigned = smaller.to_u64();

        let smaller_mask_storage = match P::Storage::try_from(smaller_mask) {
            Ok(v) => v,
            Err(_) => unreachable!(),
        };
        let smaller_unsigned_storage = match P::Storage::try_from(smaller_unsigned) {
            Ok(v) => v,
            Err(_) => unreachable!(),
        };

        let high = recent.value & !smaller_mask_storage;
        let mut result = Self::new(high | smaller_unsigned_storage);

        let recent_low_u64: u64 = (recent.value & smaller_mask_storage).into();
        let recent_low = recent_low_u64 as i64;
        let smaller_val = smaller_unsigned as i64;

        if recent_low < smaller_val {
            let abs_diff = smaller_val - recent_low;
            let threshold = (smaller_msb as i64) - bias;
            if abs_diff >= threshold {
                let smaller_msb_x2 = match P::Storage::try_from(smaller_msb << 1) {
                    Ok(v) => v,
                    Err(_) => unreachable!(),
                };
                result = Self::new(result.value.wrapping_sub(smaller_msb_x2));
            }
        } else {
            let abs_diff = recent_low - smaller_val;
            let threshold = (smaller_msb as i64) + bias;
            if abs_diff > threshold {
                let smaller_msb_x2 = match P::Storage::try_from(smaller_msb << 1) {
                    Ok(v) => v,
                    Err(_) => unreachable!(),
                };
                result = Self::new(result.value.wrapping_add(smaller_msb_x2));
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

impl<P: CounterParams> core::ops::Add for Counter<P> {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        self.wrapping_add(rhs)
    }
}

impl<P: CounterParams> core::ops::Sub for Counter<P> {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        self.wrapping_sub(rhs)
    }
}

/// Trait exposing common counter metadata so generic code can work across
/// differently-sized types without casting associated types directly.
pub trait CounterTrait: Copy {
    fn bits() -> u32;
    fn mask_u64() -> u64;
    fn msb_u64() -> u64;
    fn to_u64(self) -> u64;
}

impl<P: CounterParams> CounterTrait for Counter<P> {
    #[inline]
    fn bits() -> u32 {
        P::BITS
    }
    #[inline]
    fn mask_u64() -> u64 {
        P::MASK.into()
    }
    #[inline]
    fn msb_u64() -> u64 {
        P::MSB.into()
    }
    #[inline]
    fn to_u64(self) -> u64 {
        self.value.into()
    }
}
