use crate::timesync::counter::{Counter16, Counter23, Counter24, Counter64};
use crate::timesync::windowed_min::WindowedMinTS24;

/// Default One Way Delay (OWD) to return before time sync completes.
pub const DEFAULT_OWD_USEC: u32 = 200_000;

pub const TIME_23_LOST_BITS: u32 = 3;

/// Largest 23-bit counter difference value considered positive.
pub const TIME_23_BIAS: u32 = 0x200_000;

/// Error bound for 23-bit timestamps: `<= 8*2-1 = 15` microseconds.
pub const TIME_23_ERROR_BOUND: u32 = (1 << TIME_23_LOST_BITS) * 2 - 1;

pub const TIME_16_LOST_BITS: u32 = 9;

/// Largest 16-bit counter difference value considered positive.
pub const TIME_16_BIAS: u32 = 0x4000;

/// Error bound for 16-bit timestamps: `<= 512*2-1 = 1.023` milliseconds.
pub const TIME_16_ERROR_BOUND: u32 = (1 << TIME_16_LOST_BITS) * 2 - 1;

/// Window size for `WindowedMinTS24Deltas`.
/// Since clocks drift over time, old measurements must be ignored.
/// Assumes that clocks drift ~1 millisecond every 10 seconds.
pub const DRIFT_WINDOW_USEC: u64 = 10_000_000;

const SIGN_ROLLOVER_THRESHOLD: u32 = (1u32 << 22) << TIME_23_LOST_BITS;

/// Network time synchroniser.
///
/// Both peers keep an instance.  Outgoing packets carry a 24-bit truncated
/// local timestamp.  On receipt the delta `(local_recv - remote_send)` is fed
/// into a windowed-minimum tracker.  Peers periodically exchange their current
/// minimum delta, which allows each side to solve for clock skew and minimum
/// one-way delay.
#[derive(Clone, Debug)]
pub struct TimeSynchroniser {
    synchronised: bool,
    remote_time_delta_usec: u32,
    minimum_one_way_delay_usec: u32,
    windowed_min_ts24_deltas: WindowedMinTS24,
    last_fc_min_delta_ts24: Counter24,
    got_peer_update: bool,
}

impl TimeSynchroniser {
    /// Create a new, unsynchronised `TimeSynchroniser`.
    pub fn new() -> Self {
        Self {
            synchronised: false,
            remote_time_delta_usec: 0,
            minimum_one_way_delay_usec: DEFAULT_OWD_USEC,
            windowed_min_ts24_deltas: WindowedMinTS24::new(),
            last_fc_min_delta_ts24: Counter24::new(0),
            got_peer_update: false,
        }
    }

    /// Call this when the peer provides its latest 24-bit `MinDeltaTS24` value.
    ///
    /// The peer should do this periodically (e.g. every 2 seconds, or more
    /// frequently during the first minute).
    pub fn on_peer_min_delta_ts24(&mut self, min_delta_ts24: Counter24) {
        self.last_fc_min_delta_ts24 = min_delta_ts24;
        self.got_peer_update = true;
        self.recalculate();
    }

    /// Convert a local microsecond timestamp into the 24-bit truncated value
    /// that should be attached to every outgoing datagram.
    #[inline]
    pub fn local_time_to_datagram_ts24(local_usec: u64) -> u32 {
        ((local_usec >> TIME_23_LOST_BITS) & 0x00ff_ffff) as u32
    }

    /// Call this for every incoming datagram that carries a 24-bit timestamp.
    ///
    /// Returns an estimated one-way delay (OWD) for this datagram in
    /// microseconds, or `0` if the synchroniser is not yet synchronised.
    pub fn on_authenticated_datagram_timestamp(
        &mut self,
        remote_send_ts24: Counter24,
        local_recv_usec: u64,
    ) -> u32 {
        let local_ts24 = Counter24::new((local_recv_usec >> TIME_23_LOST_BITS) as u32);

        // OWD_i + ClockDelta(L-R)_i = Local Receive Time - Remote Send Time
        let delta_ts24 = local_ts24 - remote_send_ts24;

        self.windowed_min_ts24_deltas
            .update(delta_ts24, local_recv_usec, DRIFT_WINDOW_USEC);

        self.recalculate();

        let mut network_trip_usec = 0u32;

        if self.synchronised {
            network_trip_usec = self.minimum_one_way_delay_usec;

            let min_delta_ts24 = self.windowed_min_ts24_deltas.best();
            if delta_ts24.circ_gt(min_delta_ts24) {
                let relative_ts24 = delta_ts24 - min_delta_ts24;
                network_trip_usec += relative_ts24.to_unsigned() << TIME_23_LOST_BITS;
            }
        }

        network_trip_usec
    }

    /// Return the current minimum delta (best sample from the windowed tracker).
    #[inline]
    pub fn min_delta_ts24(&self) -> Counter24 {
        self.windowed_min_ts24_deltas.best()
    }

    /// Return `true` if the synchroniser has enough data to estimate peer time.
    #[inline]
    pub fn is_synchronised(&self) -> bool {
        self.synchronised
    }

    /// Get the minimum one-way delay seen so far (microseconds).
    ///
    /// This is equivalent to the shortest RTT/2 observed by any pair of
    /// packets, i.e. the average of upstream and downstream OWD.
    #[inline]
    pub fn minimum_one_way_delay_usec(&self) -> u32 {
        self.minimum_one_way_delay_usec
    }

    /// Get the current estimated remote time delta (microseconds).
    ///
    /// This is the estimated offset from local time to remote time:
    /// `remote_time = local_time + remote_time_delta_usec`.
    /// Returns `0` if not yet synchronised.
    #[inline]
    pub fn remote_time_delta_usec(&self) -> u32 {
        self.remote_time_delta_usec
    }

    /// Compress a local timestamp into a 16-bit remote-time field.
    ///
    /// Returns `None` if not yet synchronised.
    #[inline]
    pub fn to_remote_time_16(&self, local_usec: u64) -> Option<u16> {
        if !self.synchronised {
            return None;
        }

        let local_ts16 = (local_usec >> TIME_16_LOST_BITS) as u16;
        let delta_ts16 = (self.remote_time_delta_usec >> TIME_16_LOST_BITS) as u16;

        Some(local_ts16.wrapping_add(delta_ts16))
    }

    /// Expand a received 16-bit remote timestamp back into a local microsecond value.
    ///
    /// `local_usec` is the local wall-clock time at which the 16-bit field was received.
    #[inline]
    pub fn from_local_time_16(local_usec: u64, timestamp16: Counter16) -> u64 {
        Counter64::expand_from_truncated_with_bias(
            Counter64::new(local_usec >> TIME_16_LOST_BITS),
            timestamp16,
            TIME_16_BIAS as i64,
        )
        .to_unsigned()
            << TIME_16_LOST_BITS
    }

    /// Compress a local timestamp into a 23-bit remote-time field.
    ///
    /// Returns `None` if not yet synchronised.
    #[inline]
    pub fn to_remote_time_23(&self, local_usec: u64) -> Option<u32> {
        if !self.synchronised {
            return None;
        }

        let local_ts23 = Counter23::new((local_usec >> TIME_23_LOST_BITS) as u32);
        let delta_ts23 = Counter23::new(self.remote_time_delta_usec >> TIME_23_LOST_BITS);

        Some((local_ts23 + delta_ts23).to_unsigned())
    }

    /// Expand a received 23-bit remote timestamp back into a local microsecond value.
    ///
    /// `local_usec` is the local wall-clock time at which the 23-bit field was received.
    #[inline]
    pub fn from_local_time_23(local_usec: u64, timestamp23: Counter23) -> u64 {
        Counter64::expand_from_truncated_with_bias(
            Counter64::new(local_usec >> TIME_23_LOST_BITS),
            timestamp23,
            TIME_23_BIAS as i64,
        )
        .to_unsigned()
            << TIME_23_LOST_BITS
    }

    fn recalculate(&mut self) {
        if !self.windowed_min_ts24_deltas.is_valid() || !self.got_peer_update {
            return;
        }

        let min_recv_delta_ts24 = self.windowed_min_ts24_deltas.best();
        let min_send_delta_ts24 = self.last_fc_min_delta_ts24;

        // minOWD ~= (minSend + minRecv) / 2
        let min_owd_ts23 =
            Counter23::new((min_send_delta_ts24 + min_recv_delta_ts24).to_unsigned() >> 1);

        // ClockDelta ~= (minSend - minRecv) / 2
        let clock_delta_ts23 =
            Counter23::new((min_send_delta_ts24 - min_recv_delta_ts24).to_unsigned() >> 1);

        self.remote_time_delta_usec = clock_delta_ts23.to_unsigned() << TIME_23_LOST_BITS;

        let mut min_owd_usec = min_owd_ts23.to_unsigned() << TIME_23_LOST_BITS;
        if min_owd_usec >= SIGN_ROLLOVER_THRESHOLD {
            min_owd_usec = 0;
        }
        self.minimum_one_way_delay_usec = min_owd_usec;

        self.synchronised = true;
    }
}

impl Default for TimeSynchroniser {
    fn default() -> Self {
        Self::new()
    }
}
