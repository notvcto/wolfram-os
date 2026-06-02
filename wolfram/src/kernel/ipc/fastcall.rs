//! FastCall — synchronous IPC for latency-critical paths.
//! Register-only. ~50-150 cycles. Thread donation model.
//! Used by interrupt handlers and capability hot paths only.
//! NOT for application code.
//!
//! ~liedtke would say this should be 30 cycles.
//! liedtke would be right.
