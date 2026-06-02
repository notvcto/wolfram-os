# The Wolfram Capability Model

This is the most important document in the repository.
Everything else is implementation. This is the idea.

---

## What A Capability Is

A capability is an unforgeable token that simultaneously names a resource
and authorizes operations on it.

In most systems, naming and authorization are separate:

```
Linux:
  name:          /etc/passwd        (anyone can name this)
  authorization: UID/GID/ACL check  (kernel decides separately)
```

In Wolfram:

```
Wolfram:
  name + authorization: the handle itself
  possession = permission. no separate check.
```

If a process holds a handle, it has the authorization the handle encodes.
If it doesn't hold a handle, it cannot name the resource at all.
The confused deputy problem doesn't exist here — authority is explicit, not ambient.

---

## Kernel Objects and Handles

Every resource the kernel manages is a kernel object:

```
Process     — address space + handle table
Thread      — execution context within a process
VMO         — Virtual Memory Object (memory region)
Channel     — bidirectional async message pipe
Job         — process container + policy enforcer
Resource    — hardware access (MMIO ranges, interrupt vectors)
```

Every kernel object has handles. A handle is an unforgeable integer
that lives in a process's handle table. The integer is meaningless
outside that process — you cannot take handle `7` from process A
and use it in process B. The kernel validates every syscall against
the calling process's handle table.

---

## Rights

Every handle carries a rights bitmask:

```rust
bitflags! {
    pub struct Rights: u32 {
        const DUPLICATE  = 1 << 0;  // copy handle to give to another process
        const TRANSFER   = 1 << 1;  // move handle into another process
        const READ       = 1 << 2;  // read from object
        const WRITE      = 1 << 3;  // write to object
        const EXECUTE    = 1 << 4;  // map VMO as executable
        const MAP        = 1 << 5;  // map VMO into address space
        const SIGNAL     = 1 << 6;  // raise signals on object
        const WAIT       = 1 << 7;  // wait for signals
        const INSPECT    = 1 << 8;  // read metadata without using object
        const MANAGE     = 1 << 9;  // lifecycle operations (kill, destroy)
    }
}
```

`INSPECT` separates "can use this object" from "can see metadata about it."
A process can hold a channel handle without knowing who is on the other end.

---

## Attenuation

When passing a handle to another process, you can only pass equal or fewer rights.
Never more. The kernel enforces this on every handle transfer.

```rust
// you hold READ | WRITE | DUPLICATE on a VMO
// you can give a child READ only:
channel.send(Message {
    handles: [vmo.attenuate(Rights::READ)],
})?;

// you cannot do this — kernel rejects it
channel.send(Message {
    handles: [vmo.attenuate(Rights::READ | Rights::EXECUTE)],
    // EXECUTE wasn't in your rights. rejected.
})?;
```

Authority flows in one direction: downward.
A child cannot have more authority than its parent granted.
A parent cannot grant more than it holds.
The root of all authority is the kernel at boot.

---

## Rust Type Enforcement

The capability model is enforced at runtime by the kernel.
It is also enforced at compile time by the Rust type system.

```rust
pub struct Handle<T: KernelObject, R: Rights> {
    raw: u32,
    _type: PhantomData<T>,
    _rights: PhantomData<R>,
}

// These are different types. The compiler will not let you mix them.
Handle<Vmo, Read>
Handle<Vmo, ReadWrite>
Handle<Channel, ReadWrite>

// This is a compile error. Not a runtime panic. A compile error.
fn write_vmo(h: Handle<Vmo, ReadWrite>, data: &[u8]) { ... }

let ro: Handle<Vmo, Read> = ...;
write_vmo(ro, data);  // ← does not compile
```

This costs nothing at runtime. `PhantomData` is zero-size.
The type information exists only during compilation.
Two layers of enforcement, one of them free.

---

## Revocation

The hard problem of capability systems: once you give someone a handle,
how do you take it back?

Wolfram uses indirect capability nodes. Processes don't hold handles
to objects directly — they hold handles to capability nodes that point
to objects. To revoke, the kernel invalidates the node. Every handle
pointing through it immediately loses access. The holder's next syscall
using that handle returns `Error::CapabilityRevoked`.

```
process A holds: Handle → CapNode(7) → VMO(0xdeadbeef)
process B holds: Handle → CapNode(7) → VMO(0xdeadbeef)

kernel revokes CapNode(7):
  process A's handle → CapNode(7) [INVALID] → error
  process B's handle → CapNode(7) [INVALID] → error
```

One kernel operation. Instant. Both handles dead simultaneously.

This adds one pointer indirection per capability lookup.
The overhead is negligible. The operational value is not.

---

## Capability Derivation Tree

Every handle has a lineage. Every capability derives from a parent capability.
The tree's root is the kernel at boot — it creates the first capabilities
and hands them to init. Everything else derives from that.

`fer cap audit <pid>` walks this tree for a specific process and shows
exactly where each of its handles came from, and what rights were granted
at each step.

A compromised process's entire authority lineage is auditable in one command.
This is not possible on any mainstream operating system today.

---

## What This Means In Practice

```
Linux:     open("/etc/passwd", O_RDONLY)
           kernel checks UID, GID, permission bits, ACLs, SELinux policy
           five separate layers asked "is this allowed"

Wolfram:   file_read(passwd_handle)
           kernel checks: does this process hold passwd_handle?
           does passwd_handle have READ rights?
           that's it. two checks. one data structure.
```

The simplicity isn't a compromise. It's a consequence of getting the model right.

*W —*
