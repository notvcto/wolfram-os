//! Wolfram IPC
//! Channels (async, bytes + handles) + FastCall (sync, registers, ~50-150 cycles).
//! Handle transfer is move semantics. Sender loses handle on send.

pub mod channel;
pub mod fastcall;

pub fn init() {}
