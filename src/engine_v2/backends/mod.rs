//! State backends (Constitution §9, ADR-0002). Each implements `SpinState`
//! behind the single interface; operators never see a concrete layout.
//!
//! - `reference`: correct-by-construction f64 backend. NOT the dense
//!   production layout and NOT performance-tuned — it is the golden oracle
//!   the Verification Track audits the fast backends against.
//! - `sparse_bitslice`: exact bit-sliced integer machine (Blueprint v0.1).
//! - `dense_byte`: dense float workhorse with production-shaped contiguous
//!   kernels (Blueprint v0.2), for instances where CSR indirection loses.

pub mod dense_byte;
pub mod reference;
pub mod sparse_bitslice;

pub use dense_byte::{DenseByteState, DENSE_N_LIMIT};
pub use reference::ReferenceState;
pub use sparse_bitslice::SparseBitSlice;
