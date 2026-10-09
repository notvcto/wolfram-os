# Wolfram IPC

Wolfram provides two inter-process communication primitives. Both are strictly capability-controlled.

## Channels
Channels are asynchronous, bidirectional message pipes.
- **Payload:** Bytes + capability handles.
- **Queueing:** Bounded queues. Sends exert backpressure when full.
- **Semantics:** Handle transfer is a *move*. When a process sends a handle over a channel, it loses that handle in its own table.

## FastCall
FastCall is a synchronous, register-only IPC mechanism for latency-critical paths.
- **Performance:** Designed for ~50-150 cycles.
- **Model:** Thread donation. The caller's thread context is temporarily donated to the callee.
- **Use Case:** Interrupt handlers and capability hot paths only. Not for general application code.

*See [docs/architecture.md](architecture.md) for architectural context.*
