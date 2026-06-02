//! Async bidirectional channel. Carries bytes + handles.
//! Bounded queue. Backpressure on send when full.
//! Handle transfer is a move. Sender loses the handle.
//! Phase 2 implementation.
