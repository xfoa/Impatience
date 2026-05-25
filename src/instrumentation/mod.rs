pub mod aggregator;
pub(crate) mod event;
pub mod profiler;
pub mod reporter;
pub mod span;
pub mod svg;
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
