/// Latency sample aggregation.
pub mod aggregator;
pub(crate) mod event;
/// Main instrumentation profiler.
pub mod profiler;
/// Reporter trait and snapshot types.
pub mod reporter;
/// Measurement span type.
pub mod span;
/// SVG chart generation.
pub mod svg;
/// Event tracking with pending spans.
pub mod tracker;

pub use aggregator::LatencyAggregator;
pub use event::EventId;
pub use profiler::Profiler;
pub use reporter::{Reporter, Snapshot};
pub use span::Span;
pub use svg::{histogram_svg, scatter_plot_svg};
pub use tracker::EventTracker;

#[doc(hidden)]
pub use reporter::ConsoleReporter;
