//! TimeSync: Network time synchronisation in Rust.
//!
//! This module reimplements the [TimeSync](https://github.com/catid/TimeSync)
//! C++ library.  It synchronises clocks between two UDP peers by treating
//! every packet as a timing probe, and provides per-packet one-way-delay
//! estimates as well as compact (16- or 23-bit) remote timestamps.

mod counter;
mod synchroniser;
mod windowed_min;

pub use counter::{Counter, Counter8, Counter16, Counter23, Counter24, Counter64, CounterParams, CounterStorage, CounterTrait};
pub use synchroniser::{
    TimeSynchroniser, DEFAULT_OWD_USEC, DRIFT_WINDOW_USEC, TIME_16_BIAS,
    TIME_16_ERROR_BOUND, TIME_16_LOST_BITS, TIME_23_BIAS, TIME_23_ERROR_BOUND,
    TIME_23_LOST_BITS,
};
pub use windowed_min::{Sample, WindowedMinTS24};
