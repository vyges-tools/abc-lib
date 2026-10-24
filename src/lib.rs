// SPDX-License-Identifier: Apache-2.0
//! ABC (logic synthesis and verification), pinned and statically linked.
//!
//! 🔑 **One method per ABC call, in the caller's order.** [`Abc::execute`] runs one command line
//! through ABC's command interpreter, exactly as written; nothing here rewrites, batches or
//! reorders commands. A caller reproducing another program's ABC session issues that program's
//! commands in that program's order.
//!
//! ⛔ **ABC has ONE process-wide frame** (the current network, the current AIG, the last proof
//! status and counterexample). [`Abc::start`] therefore takes a process-wide lock and holds it until
//! the session is dropped: a second session waits for the first. Commands print to stdout, as ABC
//! prints them.

use std::ffi::{c_char, CString};
use std::sync::{Mutex, MutexGuard};

mod ffi {
    use std::ffi::c_char;

    extern "C" {
        pub fn vyges_abc_start() -> i32;
        pub fn vyges_abc_stop();
        pub fn vyges_abc_execute(command: *const c_char) -> i32;
        pub fn vyges_abc_execute_captured(command: *const c_char, path: *const c_char) -> i32;
        pub fn vyges_abc_prob_status() -> i32;
        pub fn vyges_abc_cex(po: *mut i32, frame: *mut i32) -> i32;
        pub fn vyges_abc_cex_inputs(frame: i32, bits: *mut i32, len: i32) -> i32;
        pub fn vyges_abc_network_counts(pis: *mut i32, pos: *mut i32, latches: *mut i32, nodes: *mut i32) -> i32;
        pub fn vyges_abc_network_name(kind: i32, index: i32, buf: *mut c_char, len: i32) -> i32;
    }
}

static FRAME: Mutex<()> = Mutex::new(());

/// Why a call failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// `Abc_Start` did not produce a frame.
    Start,
    /// The command contains a NUL byte and cannot be passed to ABC.
    Nul(String),
    /// ABC's interpreter returned non-zero for the command (or the call threw: -1). `output` is
    /// what it printed, from [`Abc::execute_captured`] (empty from [`Abc::execute`]).
    Command { command: String, code: i32, output: String },
    /// [`Abc::execute_captured`] could not redirect standard output to, or read back, its file.
    Capture(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Start => write!(f, "ABC did not start"),
            Error::Nul(c) => write!(f, "command contains a NUL byte: {c:?}"),
            Error::Command { command, code, .. } => write!(f, "ABC command `{command}` returned {code}"),
            Error::Capture(e) => write!(f, "capturing ABC's output: {e}"),
        }
    }
}

impl std::error::Error for Error {}

/// `Abc_FrameReadProbStatus`: the verdict the last proof command left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbStatus {
    /// -1: no verdict (not run, or the engine gave up).
    Undecided,
    /// 0: satisfiable — the property fails, or the two designs differ.
    Sat,
    /// 1: unsatisfiable — the property holds, or the two designs are equivalent.
    Unsat,
    /// Any other value.
    Other(i32),
}

impl ProbStatus {
    fn from_raw(v: i32) -> Self {
        match v {
            -1 => ProbStatus::Undecided,
            0 => ProbStatus::Sat,
            1 => ProbStatus::Unsat,
            v => ProbStatus::Other(v),
        }
    }
}

/// The counterexample the last proof command left: the output that failed and the time frame
/// (0-based) it failed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cex {
    pub po: i32,
    pub frame: i32,
}

/// The current logic network's size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetworkCounts {
    pub pis: i32,
    pub pos: i32,
    pub latches: i32,
    pub nodes: i32,
}

/// An ABC session: `Abc_Start` on creation, `Abc_Stop` on drop, the process-wide lock between.
pub struct Abc {
    _frame: MutexGuard<'static, ()>,
}

impl Abc {
    /// `Abc_Start`, after taking the process-wide lock (waits while another session is open).
    pub fn start() -> Result<Abc, Error> {
        // A panic in another session's caller poisons the lock but leaves ABC stopped (its drop
        // ran); the frame is safe to start again.
        let guard = FRAME.lock().unwrap_or_else(|e| e.into_inner());
        if unsafe { ffi::vyges_abc_start() } != 0 {
            return Err(Error::Start);
        }
        Ok(Abc { _frame: guard })
    }

    /// `Cmd_CommandExecute(frame, command)`: one ABC command line, as written.
    pub fn execute(&mut self, command: &str) -> Result<(), Error> {
        let c = CString::new(command).map_err(|_| Error::Nul(command.to_string()))?;
        let code = unsafe { ffi::vyges_abc_execute(c.as_ptr() as *const c_char) };
        if code == 0 {
            Ok(())
        } else {
            Err(Error::Command { command: command.to_string(), code, output: String::new() })
        }
    }

    /// [`Abc::execute`], returning what the command printed instead of letting it reach standard
    /// output. ⚠️ Standard output is redirected PROCESS-WIDE for the call: anything another thread
    /// prints meanwhile lands in the capture. This thread's buffered output is flushed first.
    pub fn execute_captured(&mut self, command: &str) -> Result<String, Error> {
        use std::io::Write;
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let c = CString::new(command).map_err(|_| Error::Nul(command.to_string()))?;
        let path = std::env::temp_dir()
            .join(format!("vyges-abc-{}-{}.out", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        let p = CString::new(path.to_string_lossy().into_owned()).map_err(|_| Error::Capture("temp path".into()))?;
        let _ = std::io::stdout().flush();
        let code = unsafe { ffi::vyges_abc_execute_captured(c.as_ptr(), p.as_ptr()) };
        if code == -2 {
            return Err(Error::Capture(format!("cannot redirect standard output to {}", path.display())));
        }
        let output = std::fs::read(&path).map(|b| String::from_utf8_lossy(&b).into_owned());
        let _ = std::fs::remove_file(&path);
        let output = output.map_err(|e| Error::Capture(format!("{}: {e}", path.display())))?;
        if code == 0 {
            Ok(output)
        } else {
            Err(Error::Command { command: command.to_string(), code, output })
        }
    }

    /// `Abc_FrameReadProbStatus`.
    pub fn prob_status(&self) -> ProbStatus {
        ProbStatus::from_raw(unsafe { ffi::vyges_abc_prob_status() })
    }

    /// `Abc_FrameReadCex`: the counterexample the last proof command left, if any.
    pub fn cex(&self) -> Option<Cex> {
        let (mut po, mut frame) = (0, 0);
        (unsafe { ffi::vyges_abc_cex(&mut po, &mut frame) } == 1).then_some(Cex { po, frame })
    }

    /// The last counterexample's primary-input values in time `frame` (0-based), in the order of
    /// its primary inputs; `None` if there is no counterexample or the frame is outside it.
    pub fn cex_inputs(&self, frame: i32) -> Option<Vec<bool>> {
        let n = unsafe { ffi::vyges_abc_cex_inputs(frame, std::ptr::null_mut(), 0) };
        if n < 0 {
            return None;
        }
        let mut bits = vec![0i32; n as usize];
        unsafe { ffi::vyges_abc_cex_inputs(frame, bits.as_mut_ptr(), n) };
        Some(bits.into_iter().map(|b| b != 0).collect())
    }

    /// Counts of the current network (`Abc_FrameReadNtk`), if there is one.
    pub fn network_counts(&self) -> Option<NetworkCounts> {
        let (mut pis, mut pos, mut latches, mut nodes) = (0, 0, 0, 0);
        let ok = unsafe { ffi::vyges_abc_network_counts(&mut pis, &mut pos, &mut latches, &mut nodes) } == 1;
        ok.then_some(NetworkCounts { pis, pos, latches, nodes })
    }

    /// The name of the current network's primary input `index` (`Abc_ObjName(Abc_NtkPi(..))`).
    pub fn pi_name(&self, index: i32) -> Option<String> {
        name(0, index)
    }

    /// The name of the current network's primary output `index` (`Abc_ObjName(Abc_NtkPo(..))`).
    pub fn po_name(&self, index: i32) -> Option<String> {
        name(1, index)
    }
}

fn name(kind: i32, index: i32) -> Option<String> {
    let mut buf = vec![0u8; 256];
    let n = unsafe { ffi::vyges_abc_network_name(kind, index, buf.as_mut_ptr() as *mut c_char, buf.len() as i32) };
    if n < 0 {
        return None;
    }
    if n as usize >= buf.len() {
        buf = vec![0u8; n as usize + 1];
        unsafe { ffi::vyges_abc_network_name(kind, index, buf.as_mut_ptr() as *mut c_char, buf.len() as i32) };
    }
    buf.truncate(n as usize);
    Some(String::from_utf8_lossy(&buf).into_owned())
}

impl Drop for Abc {
    fn drop(&mut self) {
        unsafe { ffi::vyges_abc_stop() };
    }
}
