# Little Snitch for Linux

This repository contains the Open Source part of Little Snitch for Linux.
It consists of:

* The Rust crate `ebpf`, which contains all eBPF programs attached to the
  Linux kernel.
* The Rust crate `common`, which contains all types and functions shared
  between kernel and user space.
* The Rust crate `demo-runner`. This is a user-space program which loads
  the eBPF programs into the kernel and demonstrates how to share data
  with these programs via eBPF maps. It loads two blocklists for
  demonstration: `blocked_hosts.txt` and `blocked_domains.txt`.
* `webroot`: This is the JavaScript web UI of Little Snitch for Linux.

All code in this public repository is Open Source and distributed under
the [GNU General Public License, Version 2]. It is part of Little Snitch
for Linux, a free product by [Objective Development](https://obdev.at).
The full product also includes proprietary code that is not part of this
repository. While Little Snitch for Linux is free to use, that proprietary
portion is not Open Source.


## Prerequisites

1. stable rust toolchains: `rustup toolchain install stable`
2. the pinned nightly toolchain for the eBPF build, e.g.
   `rustup toolchain install nightly-2026-07-07 --component rust-src`
   (see `ebpf/rust-toolchain.toml` for the currently pinned version)
3. bpf-linker: `cargo install bpf-linker --locked` (version 0.10.3 is known to work)
4. the `clang` C/C++ compiler

Note: eBPF is a bit fragile. The kernel verifier may reject the program
after changes in the verifier itself, in the Rust compiler or in bpf-linker.
The nightly toolchain used for the eBPF build is therefore pinned in
`ebpf/rust-toolchain.toml`; the build scripts of the crates embedding the
eBPF program (demo-runner and the daemon) read the pin from there and pass
it to `rustup run` explicitly. This code is known to pass the verifier on
Linux kernels 6.12 to 7.2.0 when compiled with the pinned nightly-2026-07-07
and bpf-linker 0.10.3. Other versions may or may not work.

## Build & Run

Use `cargo build`, `cargo check`, etc. as normal. Run your program with:

```shell
cargo run --release
```

Cargo build scripts automatically build the eBPF programs and include them
in the binary.


### eBPF

All eBPF code is distributed under the terms of the [GNU General Public License, Version 2].

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the GPL-2 license, shall be
licensed as above, without any additional terms or conditions.

[GNU General Public License, Version 2]: LICENSE-GPL2.txt
