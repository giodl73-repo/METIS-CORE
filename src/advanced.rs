//! Advanced extension points for custom multilevel partitioning experiments.
//!
//! The top-level crate API is the stable METIS-like surface. This module keeps
//! lower-level algorithm components available intentionally, without exposing
//! the implementation module layout as the public API.
//!
//! Coarseners, initial partitioners, and refiners return `Result` so custom
//! extension points can reject invalid graphs, impossible part counts, malformed
//! partitions, and implementation-specific contract failures explicitly.
//!
//! ```
//! use metis_core::advanced::{
//!     Coarsener, FiducciaMattheyses, GrowBisect, InitialPartitioner, Refiner,
//!     SortedHeavyEdgeMatchWithParams,
//! };
//! use metis_core::{CsrGraph, ObjectiveType};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let graph = CsrGraph::from_csr(
//!     &[0, 2, 4, 6, 8],
//!     &[1, 3, 0, 2, 1, 3, 0, 2],
//!     &[],
//!     &[],
//! )?;
//! let (coarse, _map) = SortedHeavyEdgeMatchWithParams::new(20, 2).coarsen(&graph)?;
//! let initial = GrowBisect.partition(&coarse, 2, 7)?;
//! let refined =
//!     FiducciaMattheyses::new(10, false, ObjectiveType::Cut, 10, 5).refine(&coarse, initial)?;
//! refined.validate_for_graph(&coarse)?;
//! # Ok(())
//! # }
//! ```

pub use crate::coarsen::hem::{HeavyEdgeMatch, HeavyEdgeMatchWithParams};
pub use crate::coarsen::mindegree::MinDegreeMatch;
pub use crate::coarsen::shem::{SortedHeavyEdgeMatch, SortedHeavyEdgeMatchWithParams};
pub use crate::coarsen::twohop::{TwoHopMatch, TwoHopMatchWithParams};
pub use crate::coarsen::Coarsener;
pub use crate::init::grow::{GrowBisect, GrowKway};
pub use crate::init::multiconstraint::MultiConstraintInit;
pub use crate::init::random::RandomBisect;
pub use crate::init::InitialPartitioner;
pub use crate::multilevel::hierarchy::CoarseningHierarchy;
pub use crate::refine::fm::FiducciaMattheyses;
pub use crate::refine::kway::GreedyKWay;
pub use crate::refine::lp::rebalance_to_ufactor;
pub use crate::refine::Refiner;
