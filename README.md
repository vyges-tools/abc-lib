# vyges-abc-lib

[ABC](https://github.com/berkeley-abc/abc), UC Berkeley's system for logic synthesis and
verification, built with **Bazel** at a pinned version into one static archive (`libvyges_abc.a`),
behind a small C shim and a safe Rust API.

> Part of Vyges Loom. Used where an engine restructures logic with ABC and must get the same
> network on every run and every machine, and by the formal engines for equivalence and property
> checking.

## What it gives you

```rust
use vyges_abc_lib::{Abc, ProbStatus};

let mut abc = Abc::start()?;                       // Abc_Start; holds the process-wide frame
abc.execute("miter golden.blif revised.blif")?;    // one ABC command line, as written
abc.execute("strash; &get -n; &cec -m")?;
assert_eq!(abc.prob_status(), ProbStatus::Unsat);  // equivalent
// drop: Abc_Stop
```

- `execute` runs one command line through ABC's own interpreter (`Cmd_CommandExecute`), so every
  ABC command is available: synthesis (`strash`, `dc2`, `&if`, `map`, ...), equivalence (`cec`,
  `&cec`, `dsec`), and property checking (`pdr`, `bmc3`, `int`, `dprove`).
- `prob_status`, `cex` and `cex_inputs` read the verdict, the failing output and frame, and the
  input values the last proof command left; `pi_name` / `po_name` name the current network's signals.
- `network_counts` reads the size of the current network.

ABC keeps one frame per process, so a session holds a process-wide lock; a second session waits.
Commands print to stdout, as ABC prints them; `execute_captured` returns that output instead.

⚠️ `&cec -m` assumes a SINGLE-output miter: on a multi-output one (`miter -m`) it checks output 0
only. Use `iprove` on a multi-output miter, which names the first output it disproves.

## Pin

- **ABC `0.64-yosyshq.bcr.2`** from the Bazel Central Registry (`MODULE.bazel`), with the module's
  own defines (`ABC_NAMESPACE=abc`, `ABC_USE_PTHREADS`, `ABC_USE_CUDD`, `ABC_USE_STDINT_H`).
- **No readline** (`--@abc//:use_readline=false`): nothing GPL is linked, and nothing here reads a
  terminal.
- the **hermetic LLVM** toolchain (Bazel module `llvm`), so the compiler is not the host's;
- `-c opt -O3 -ffp-contract=off` (`.bazelrc`): no fused multiply-add, so each floating-point
  operation rounds as IEEE 754 says, however the optimiser inlines.

`abc-pin.yaml` records the set. Bump on purpose, never to "latest". A build that registers a patch
for ABC's interactive prompt (flushing it on a pipe) compiles the same code on every path this
crate reaches: the prompt is read only by ABC's own interactive loop, which `execute` never enters.

## One object, one C API

`scripts/bundle.sh` prelinks ABC, zlib and (on Linux) the toolchain's C++ runtime into ONE object
with only the `vyges_abc_*` functions global. ABC's thousands of global C symbols stay private, so
a program that links another ABC (or another C++ runtime) cannot collide with this one.

## Getting the archive

- **Prebuilt** (no Bazel): `scripts/fetch-bundle.sh <dir>` downloads the bundle published for this
  checkout's build inputs (`scripts/bundle-id.sh`) and prints `VYGES_ABC_PREBUILT_DIR=<dir>`.
  Linux x86_64 and arm64, macOS arm64.
- **From source**: `cargo build` runs `scripts/bundle.sh` (needs `bazelisk` or `bazel`).

## Licence

Apache-2.0 for the files in this repository. The archive also carries ABC (BSD-style, The Regents
of the University of California) and zlib; see `NOTICE`.
