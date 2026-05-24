use std::collections::HashMap;
use std::hash::Hash;

/// Tracks in-flight events keyed by an arbitrary type `K`.
///
/// Each entry stores the original [`Span`](crate::instrumentation::Span)
/// plus an arbitrary user-defined metadata tuple `M`.  This is useful when
/// you need to correlate a later remote-finish response with the local
/// start event, e.g. to compute end-to-end latency or to store auxiliary
/// data (character, delay, etc.) that should be reported alongside the
/// result.
///
/// # Example
/// ```
/// use impatience::instrumentation::{EventId, Span, EventTracker};
///
/// let mut tracker: EventTracker<u32, (char, u32)> = EventTracker::new();
/// let span = Span::new(EventId(1), "demo", 0);
/// tracker.insert(1, span, ('a', 42));
///
/// if let Some((span, meta)) = tracker.remove(&1) {
///     assert_eq!(meta.0, 'a');
/// }
/// ```
#[derive(Clone, Debug, Default)]
pub struct EventTracker<K, M> {
    inner: HashMap<K, (crate::instrumentation::Span, M)>,
}

impl<K, M> EventTracker<K, M>
where
    K: Eq + Hash,
{
    /// Create a new, empty tracker.
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Create a new tracker with the given capacity hint.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: HashMap::with_capacity(capacity),
        }
    }

    /// Store a span and its associated metadata.
    pub fn insert(&mut self, key: K, span: crate::instrumentation::Span, meta: M) {
        self.inner.insert(key, (span, meta));
    }

    /// Retrieve and remove a span and its metadata.
    ///
    /// Returns `None` if the key is not present.
    pub fn remove(&mut self, key: &K) -> Option<(crate::instrumentation::Span, M)> {
        self.inner.remove(key)
    }

    /// Number of in-flight events.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// `true` if no events are in flight.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}
