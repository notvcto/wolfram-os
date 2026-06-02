# Contributing to Wolfram

Welcome. You're here because you want to help build a kernel.
That's either very exciting or a sign of questionable judgment.
Probably both.

---

## Before You Write Code

Read these in order:

1. [docs/architecture.md](docs/architecture.md) — the full architecture, all six decisions
2. [docs/capability-model.md](docs/capability-model.md) — the capability model in depth
3. [docs/ipc.md](docs/ipc.md) — IPC primitives, when to use each
4. [docs/memory.md](docs/memory.md) — VMO model, no anonymous memory, why

If something in the docs is wrong or unclear, that's a contribution too.
Open a PR. Docs are code.

---

## What To Work On

Check the roadmap in README.md. If something isn't checked off, it needs doing.
If you don't know where to start, `Phase 1` items are the most approachable.
Getting the kernel to boot is a rite of passage. Help us not triple fault.

For larger contributions, open an issue first and describe what you're doing.
Not because we'll say no. Because coordination prevents two people building
the same thing in parallel with incompatible designs.

---

## Code Style

**Rust:**
- `rustfmt` always. No exceptions. `cargo fmt` before every commit.
- `clippy` always. `cargo clippy -- -D warnings`. Fix the warnings.
- `unsafe` requires a comment explaining exactly why it's necessary
  and exactly what invariants make it safe. One line is not enough.
- No `unwrap()` in kernel code. Ever. Panics in the kernel are panics.
  Use `Result`, use `Option`, handle the failure case.
- No `println!` in kernel code. Use the kernel's serial logger.

**Comments:**
- Write comments for why, not what. The code says what.
- If you're doing something non-obvious, explain it.
- If you're citing a spec or paper, link it.
- Easter eggs in comments are allowed and encouraged.
  Read [EASTEREGGS.md](EASTEREGGS.md) for the style guide.

**Commits:**
- Present tense, imperative mood: "add capability revocation" not "added"
- Be specific: "implement buddy allocator phase 2 migration" not "memory stuff"
- One logical change per commit. Not one file. One change.

---

## Testing

```bash
make test        # unit tests
make qemu        # boot in QEMU, watch serial output
make qemu-debug  # boot with GDB on :1234
```

If you're adding kernel functionality, add a test.
If you're fixing a bug, add a test that would have caught it.
If you're touching the capability model, add a lot of tests.
The capability model is the whole point. It needs to be correct.

---

## The Capability Model Is Sacred

This is the one area where we will push back hard on PRs.

The capability model is Wolfram's reason for existing. Every design decision
in the kernel exists to serve it. If a change weakens the capability model —
adds ambient authority, bypasses handle checks, introduces a shortcut for
"trusted" processes — it will not be merged.

There are no trusted processes. That's the point.
There is no root. That's the point.
There is no shortcut. That's the point.

If you think you need one of these things, open an issue and explain why.
We'll figure out the right capability-based solution together.

---

## Easter Eggs

Yes, really. Read [EASTEREGGS.md](EASTEREGGS.md).
Good easter eggs are welcome contributions.
They have a style guide. Follow it.

---

## Security Issues

If you find a vulnerability in the capability model — a way to escalate
authority beyond what was granted, forge a handle, bypass a rights check —
that's the most important kind of issue you can report.

Open an issue titled `[SECURITY] <brief description>`.
Be detailed. Include a reproduction case if possible.
We'll respond fast. This is the core of what Wolfram is.

---

## The Tone

Wolfram has a voice. It's dry, honest, occasionally funny, always precise.
Error messages, panic screens, CLI output, comments — they all sound like
the same person wrote them. A person who cares deeply about the project
and has a dark sense of humor about kernel development.

If you're writing user-facing output, read the existing messages first.
Match the tone. `W —` signs off on serious things. Keep that consistent.

---

## Credit

Everyone who contributes gets credited. In the repo, in release notes,
in the model card if we ever train something on Wolfram's codebase.

You built part of a kernel. That should be on your GitHub forever.

---

*W —*
