//! Advanced extension points for custom multilevel partitioning experiments.
//!
//! The top-level crate API is the stable METIS-like surface. This module keeps
//! lower-level algorithm components available intentionally, without exposing
//! the implementation module layout as the public API.

pub use crate::coarsen::hem::HeavyEdgeMatch;
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
