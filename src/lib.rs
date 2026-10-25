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

use std::ffi::{c_char, c_void, CString};
use std::sync::{Mutex, MutexGuard};

mod ffi {
    use std::ffi::{c_char, c_void};

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

        pub fn vyges_abc_scl_lib_alloc(name: *const c_char, file_name: *const c_char, unit_cap_fst: f32, unit_cap_snd: i32, unit_time: i32, default_max_out_slew: f32) -> *mut c_void;
        pub fn vyges_abc_scl_cell_add(lib: *mut c_void, name: *const c_char, area: f32, has_leakage: i32, leakage: f32) -> i32;
        pub fn vyges_abc_scl_input_pin_add(lib: *mut c_void, name: *const c_char, rise_cap: f32, fall_cap: f32) -> i32;
        pub fn vyges_abc_scl_output_pin_add(lib: *mut c_void, name: *const c_char, has_max_cap: i32, max_out_cap: f32, has_max_slew: i32, max_out_slew: f32, func_text: *const c_char) -> i32;
        pub fn vyges_abc_scl_timing_add(lib: *mut c_void, related_pin: *const c_char, sense: i32) -> i32;
        pub fn vyges_abc_scl_surface(lib: *mut c_void, which: i32, n0: i32, index0: *const f32, n1: i32, index1: *const f32, values: *const f32) -> i32;
        pub fn vyges_abc_scl_lib_finish(lib: *mut c_void) -> i32;
        pub fn vyges_abc_scl_lib_free(lib: *mut c_void);
        pub fn vyges_abc_scl_average_slew(lib: *mut c_void) -> f32;
        pub fn vyges_abc_scl_cell_count(lib: *mut c_void) -> i32;
        pub fn vyges_abc_scl_cell_info(lib: *mut c_void, cell: i32, n_inputs: *mut i32, n_outputs: *mut i32, area: *mut f32) -> i32;
        pub fn vyges_abc_scl_name(lib: *mut c_void, cell: i32, pin: i32, buf: *mut c_char, len: i32) -> i32;
        pub fn vyges_abc_scl_pin_truth(lib: *mut c_void, cell: i32, pin: i32, word: *mut u64) -> i32;
        pub fn vyges_abc_scl_derive_genlib(lib: *mut c_void) -> *mut c_void;
        pub fn vyges_abc_mio_free(mio: *mut c_void);
        pub fn vyges_abc_mio_gate_count(mio: *mut c_void) -> i32;
        pub fn vyges_abc_mio_gate_pin_count(mio: *mut c_void, gate: i32) -> i32;
        pub fn vyges_abc_mio_name(mio: *mut c_void, gate: i32, pin: i32, buf: *mut c_char, len: i32) -> i32;
        pub fn vyges_abc_mio_special_gate(mio: *mut c_void, which: i32) -> i32;
        pub fn vyges_abc_mio_rename(mio: *mut c_void, gate: i32, name: *const c_char, out_name: *const c_char) -> i32;
        pub fn vyges_abc_ntk_alloc_mapped_netlist(mio: *mut c_void) -> *mut c_void;
        pub fn vyges_abc_ntk_delete(ntk: *mut c_void);
        pub fn vyges_abc_ntk_create(ntk: *mut c_void, kind: i32, gate: i32) -> i32;
        pub fn vyges_abc_obj_assign_name(ntk: *mut c_void, obj: i32, name: *const c_char) -> i32;
        pub fn vyges_abc_obj_add_fanin(ntk: *mut c_void, obj: i32, fanin: i32) -> i32;
        pub fn vyges_abc_ntk_to_logic(ntk: *mut c_void) -> *mut c_void;
        pub fn vyges_abc_ntk_strash(ntk: *mut c_void, all_nodes: i32, cleanup: i32, record: i32) -> *mut c_void;
        pub fn vyges_abc_ntk_balance(ntk: *mut c_void, duplicate: i32, selective: i32, update_level: i32) -> *mut c_void;
        pub fn vyges_abc_ntk_to_netlist(ntk: *mut c_void) -> *mut c_void;
        pub fn vyges_abc_frame_set_lib_and_driver(ntk: *mut c_void) -> i32;
        pub fn vyges_abc_ntk_map(ntk: *mut c_void, delay_target: f64, area_multi: f64, delay_multi: f64, log_fan: f32, slew: f32, gain: f32, n_gates_min: i32, recovery: i32, switching: i32, skip_fanout: i32, use_profile: i32, use_buffs: i32, verbose: i32) -> *mut c_void;
        pub fn vyges_abc_ntk_cleanup(ntk: *mut c_void, verbose: i32) -> i32;
        pub fn vyges_abc_scl_buffering(ntk: *mut c_void, lib: *mut c_void, gain_ratio: i32, degree: i32, size_only: i32, add_bufs: i32, buf_pis: i32, use_wire_loads: i32, verbose: i32, very_verbose: i32) -> *mut c_void;
        pub fn vyges_abc_ntk_is_mapped_netlist(ntk: *mut c_void) -> i32;
        pub fn vyges_abc_ntk_mio(ntk: *mut c_void) -> *mut c_void;
        pub fn vyges_abc_ntk_obj_num_max(ntk: *mut c_void) -> i32;
        pub fn vyges_abc_obj_type(ntk: *mut c_void, obj: i32) -> i32;
        pub fn vyges_abc_obj_name(ntk: *mut c_void, obj: i32, buf: *mut c_char, len: i32) -> i32;
        pub fn vyges_abc_obj_fanin_count(ntk: *mut c_void, obj: i32) -> i32;
        pub fn vyges_abc_obj_fanin(ntk: *mut c_void, obj: i32, k: i32) -> i32;
        pub fn vyges_abc_obj_fanout_count(ntk: *mut c_void, obj: i32) -> i32;
        pub fn vyges_abc_obj_fanout(ntk: *mut c_void, obj: i32, k: i32) -> i32;
        pub fn vyges_abc_obj_gate(ntk: *mut c_void, obj: i32) -> i32;
        pub fn vyges_abc_gia_from_network(ntk: *mut c_void) -> *mut c_void;
        pub fn vyges_abc_gia_free(gia: *mut c_void);
        pub fn vyges_abc_gia_replace(gia: *mut c_void, next: *mut c_void) -> *mut c_void;
        pub fn vyges_abc_gia_query(gia: *mut c_void, which: i32) -> i32;
        pub fn vyges_abc_gia_rehash(gia: *mut c_void, add_strash: i32) -> *mut c_void;
        pub fn vyges_abc_gia_dch(gia: *mut c_void, bt_limit: i32) -> *mut c_void;
        pub fn vyges_abc_gia_equiv_reduce(gia: *mut c_void, use_all: i32, dual_out: i32, skip_phase: i32, verbose: i32) -> *mut c_void;
        pub fn vyges_abc_gia_syn2(gia: *mut c_void, old_algo: i32, coarsen: i32, cut_min: i32, relax_ratio: i32, delay_min: i32, verbose: i32, very_verbose: i32) -> *mut c_void;
        pub fn vyges_abc_gia_syn3(gia: *mut c_void, verbose: i32, very_verbose: i32) -> *mut c_void;
        pub fn vyges_abc_gia_syn4(gia: *mut c_void, verbose: i32, very_verbose: i32) -> *mut c_void;
        pub fn vyges_abc_gia_retime_forward(gia: *mut c_void, max_iters: i32, verbose: i32) -> *mut c_void;
        pub fn vyges_abc_gia_compress2(gia: *mut c_void, update_level: i32, verbose: i32) -> *mut c_void;
        pub fn vyges_abc_gia_area_balance(gia: *mut c_void, simple_and: i32, new_nodes_max: i32, verbose: i32, very_verbose: i32) -> *mut c_void;
        pub fn vyges_abc_gia_balance(gia: *mut c_void, simple_and: i32, strict: i32, verbose: i32) -> *mut c_void;
        pub fn vyges_abc_gia_equiv_reduce_and_remap(gia: *mut c_void, seq: i32, miter_pairs: i32) -> *mut c_void;
        pub fn vyges_abc_gia_if_map(gia: *mut c_void, delay_opt: i32, lut_size: i32, truth: i32, cut_min: i32, exp_red: i32) -> *mut c_void;
        pub fn vyges_abc_gia_synch2(gia: *mut c_void, bt_limit: i32, lut_size: i32, relax_ratio: i32) -> *mut c_void;
        pub fn vyges_abc_ntk_from_cell_mapped_gia(gia: *mut c_void, use_buffs: i32) -> *mut c_void;
        pub fn vyges_abc_ntk_from_mapped_gia(gia: *mut c_void, find_enables: i32, use_buffs: i32) -> *mut c_void;
        pub fn vyges_abc_ntk_from_gia_aig(gia: *mut c_void) -> *mut c_void;
        pub fn vyges_abc_ntk_name_from_gia(ntk: *mut c_void, gia: *mut c_void) -> i32;
        pub fn vyges_abc_ntk_redirect_ci_co(ntk: *mut c_void) -> i32;
        pub fn vyges_abc_ntk_is_strash(ntk: *mut c_void) -> i32;
        pub fn vyges_abc_getopt_reset();
        pub fn vyges_abc_ntk_dup_dfs(ntk: *mut c_void) -> *mut c_void;
        pub fn vyges_abc_scl_resize(lib: *mut c_void, ntk: *mut c_void, upsize: i32, iters: i32, iter_no_change: i32, window: i32, ratio: i32, notches: i32, use_dept: i32) -> i32;
        pub fn vyges_abc_frame_set_lib_gen(mio: *mut c_void) -> i32;
        pub fn vyges_abc_ntk_set_man_func(ntk: *mut c_void, mio: *mut c_void);
        pub fn vyges_abc_stdout_suppress() -> i32;
        pub fn vyges_abc_stdout_restore(saved: i32);
        pub fn vyges_abc_gia_start(n_objs: i32, name: *const c_char) -> *mut c_void;
        pub fn vyges_abc_gia_append_ci(gia: *mut c_void) -> i32;
        pub fn vyges_abc_gia_hash_and(gia: *mut c_void, lit0: i32, lit1: i32) -> i32;
        pub fn vyges_abc_gia_append_co(gia: *mut c_void, lit: i32) -> i32;
        pub fn vyges_abc_gia_cleanup(gia: *mut c_void) -> *mut c_void;
        pub fn vyges_abc_gia_count(gia: *mut c_void, which: i32) -> i32;
        pub fn vyges_abc_gia_ci_id(gia: *mut c_void, i: i32) -> i32;
        pub fn vyges_abc_gia_co_id(gia: *mut c_void, i: i32) -> i32;
        pub fn vyges_abc_gia_obj(gia: *mut c_void, id: i32, f0: *mut i32, c0: *mut i32, f1: *mut i32, c1: *mut i32) -> i32;
        pub fn vyges_abc_frame_update_gia(gia: *mut c_void);
        pub fn vyges_abc_frame_get_gia() -> *mut c_void;
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
    read_name(|buf, len| unsafe { ffi::vyges_abc_network_name(kind, index, buf, len) })
}

/// A name a shim call copies into a buffer, asked again with a larger buffer when it did not fit.
fn read_name(mut call: impl FnMut(*mut c_char, i32) -> i32) -> Option<String> {
    let mut buf = vec![0u8; 256];
    let n = call(buf.as_mut_ptr() as *mut c_char, buf.len() as i32);
    if n < 0 {
        return None;
    }
    if n as usize >= buf.len() {
        buf = vec![0u8; n as usize + 1];
        call(buf.as_mut_ptr() as *mut c_char, buf.len() as i32);
    }
    buf.truncate(n as usize);
    Some(String::from_utf8_lossy(&buf).into_owned())
}

impl Drop for Abc {
    fn drop(&mut self) {
        unsafe { ffi::vyges_abc_stop() };
    }
}

/// A string for a shim call; a NUL byte is [`Error::Nul`].
fn c_str(s: &str) -> Result<CString, Error> {
    CString::new(s).map_err(|_| Error::Nul(s.to_string()))
}

/// Why a library or network call failed: the call's name.
fn failed(call: &str) -> Error {
    Error::Command { command: call.to_string(), code: -1, output: String::new() }
}

/// An arc's sense, as a standard-cell library timing records it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sense {
    PositiveUnate,
    NegativeUnate,
    NonUnate,
}

/// The four tables of a timing, in the order a cell library keeps them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    CellRise,
    CellFall,
    RiseTransition,
    FallTransition,
}

/// A standard-cell library (`SC_Lib`), built cell by cell: [`CellLib::add_cell`], then that
/// cell's input pins, its output pins, and each output pin's timings and their tables, then
/// [`CellLib::finish`]. Freed on drop.
pub struct CellLib {
    ptr: *mut c_void,
}

/// A cell of a finished [`CellLib`].
#[derive(Clone, Debug, PartialEq)]
pub struct CellInfo {
    pub name: String,
    pub inputs: usize,
    pub outputs: usize,
    pub area: f32,
}

impl CellLib {
    /// `Abc_SclLibAlloc`, with the library's name, file name, units (capacitance `fst * 10^-snd`
    /// farads, time `10^-unit_time` seconds) and default maximum output slew (-1: none).
    pub fn new(_abc: &Abc, name: &str, file_name: &str, unit_cap_fst: f32, unit_cap_snd: i32, unit_time: i32, default_max_out_slew: f32) -> Result<CellLib, Error> {
        let (n, f) = (c_str(name)?, c_str(file_name)?);
        let ptr = unsafe { ffi::vyges_abc_scl_lib_alloc(n.as_ptr(), f.as_ptr(), unit_cap_fst, unit_cap_snd, unit_time, default_max_out_slew) };
        if ptr.is_null() {
            return Err(failed("Abc_SclLibAlloc"));
        }
        Ok(CellLib { ptr })
    }

    /// A cell, its id the number of cells before it. `leakage` `None` leaves it 0.
    pub fn add_cell(&mut self, name: &str, area: f32, leakage: Option<f32>) -> Result<usize, Error> {
        let n = c_str(name)?;
        let id = unsafe { ffi::vyges_abc_scl_cell_add(self.ptr, n.as_ptr(), area, leakage.is_some() as i32, leakage.unwrap_or(0.0)) };
        usize::try_from(id).map_err(|_| failed("Abc_SclCellAlloc"))
    }

    /// An input pin of the last cell (before any of its output pins).
    pub fn add_input_pin(&mut self, name: &str, rise_cap: f32, fall_cap: f32) -> Result<(), Error> {
        let n = c_str(name)?;
        match unsafe { ffi::vyges_abc_scl_input_pin_add(self.ptr, n.as_ptr(), rise_cap, fall_cap) } {
            0 => Ok(()),
            _ => Err(failed("input pin")),
        }
    }

    /// An output pin of the last cell, its function `func_text` over the cell's input pins.
    pub fn add_output_pin(&mut self, name: &str, max_out_cap: Option<f32>, max_out_slew: Option<f32>, func_text: &str) -> Result<(), Error> {
        let (n, f) = (c_str(name)?, c_str(func_text)?);
        let code = unsafe {
            ffi::vyges_abc_scl_output_pin_add(self.ptr, n.as_ptr(), max_out_cap.is_some() as i32, max_out_cap.unwrap_or(0.0), max_out_slew.is_some() as i32, max_out_slew.unwrap_or(0.0), f.as_ptr())
        };
        match code {
            0 => Ok(()),
            -2 => Err(failed(&format!("Mio_ParseFormulaTruth({func_text})"))),
            _ => Err(failed("output pin")),
        }
    }

    /// A timing on the last output pin, from `related_pin`.
    pub fn add_timing(&mut self, related_pin: &str, sense: Sense) -> Result<(), Error> {
        let n = c_str(related_pin)?;
        let s = match sense {
            Sense::PositiveUnate => 0,
            Sense::NegativeUnate => 1,
            Sense::NonUnate => 2,
        };
        match unsafe { ffi::vyges_abc_scl_timing_add(self.ptr, n.as_ptr(), s) } {
            0 => Ok(()),
            _ => Err(failed("timing")),
        }
    }

    /// One table of the last timing: its axes and its values, row by row (`index0.len()` rows of
    /// `index1.len()`).
    pub fn add_surface(&mut self, which: Surface, index0: &[f32], index1: &[f32], values: &[f32]) -> Result<(), Error> {
        if values.len() != index0.len() * index1.len() {
            return Err(failed("surface: values do not fill the axes"));
        }
        let w = match which {
            Surface::CellRise => 0,
            Surface::CellFall => 1,
            Surface::RiseTransition => 2,
            Surface::FallTransition => 3,
        };
        let code = unsafe { ffi::vyges_abc_scl_surface(self.ptr, w, index0.len() as i32, index0.as_ptr(), index1.len() as i32, index1.as_ptr(), values.as_ptr()) };
        match code {
            0 => Ok(()),
            _ => Err(failed("surface")),
        }
    }

    /// `Abc_SclLibNormalize`, `Abc_SclHashCells`, `Abc_SclLinkCells`.
    pub fn finish(&mut self) -> Result<(), Error> {
        match unsafe { ffi::vyges_abc_scl_lib_finish(self.ptr) } {
            0 => Ok(()),
            _ => Err(failed("Abc_SclLibNormalize")),
        }
    }

    /// `Abc_SclComputeAverageSlew`.
    pub fn average_slew(&self) -> f32 {
        unsafe { ffi::vyges_abc_scl_average_slew(self.ptr) }
    }

    /// The cells, in library order.
    pub fn cells(&self) -> Vec<CellInfo> {
        let n = unsafe { ffi::vyges_abc_scl_cell_count(self.ptr) }.max(0);
        (0..n)
            .map(|i| {
                let (mut inputs, mut outputs, mut area) = (0, 0, 0.0f32);
                unsafe { ffi::vyges_abc_scl_cell_info(self.ptr, i, &mut inputs, &mut outputs, &mut area) };
                let name = read_name(|b, l| unsafe { ffi::vyges_abc_scl_name(self.ptr, i, -1, b, l) }).unwrap_or_default();
                CellInfo { name, inputs: inputs as usize, outputs: outputs as usize, area }
            })
            .collect()
    }

    /// The name of pin `pin` of cell `cell` (inputs first, then outputs).
    pub fn pin_name(&self, cell: usize, pin: usize) -> Option<String> {
        read_name(|b, l| unsafe { ffi::vyges_abc_scl_name(self.ptr, cell as i32, pin as i32, b, l) })
    }

    /// The first word of pin `pin`'s truth table.
    pub fn pin_truth(&self, cell: usize, pin: usize) -> Option<u64> {
        let mut w = 0u64;
        (unsafe { ffi::vyges_abc_scl_pin_truth(self.ptr, cell as i32, pin as i32, &mut w) } == 1).then_some(w)
    }

    /// `Abc_SclDeriveGenlibSimple`: the gate library of this (finished) cell library. It prints
    /// its own line, as ABC does.
    pub fn derive_genlib(&self) -> Result<GateLib, Error> {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let ptr = unsafe { ffi::vyges_abc_scl_derive_genlib(self.ptr) };
        if ptr.is_null() {
            return Err(failed("Abc_SclDeriveGenlibSimple"));
        }
        Ok(GateLib { ptr })
    }
}

impl Drop for CellLib {
    fn drop(&mut self) {
        unsafe { ffi::vyges_abc_scl_lib_free(self.ptr) };
    }
}

/// A gate library (`Mio_Library_t`). ⚠️ NOT freed on drop: a network built over it, and the frame
/// once [`Network::install_library`] ran, refer to it; [`GateLib::free`] frees one nothing adopted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GateLib {
    ptr: *mut c_void,
}

/// A gate library's special gates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Special {
    Const0,
    Const1,
    Buffer,
}

impl GateLib {
    /// `Mio_LibraryDelete`.
    pub fn free(self) {
        unsafe { ffi::vyges_abc_mio_free(self.ptr) };
    }

    /// The number of gates (in the library's gate-list order).
    pub fn gate_count(&self) -> usize {
        unsafe { ffi::vyges_abc_mio_gate_count(self.ptr) }.max(0) as usize
    }

    /// A gate's name.
    pub fn gate_name(&self, gate: usize) -> Option<String> {
        read_name(|b, l| unsafe { ffi::vyges_abc_mio_name(self.ptr, gate as i32, -1, b, l) })
    }

    /// A gate's output pin name.
    pub fn gate_out_name(&self, gate: usize) -> Option<String> {
        read_name(|b, l| unsafe { ffi::vyges_abc_mio_name(self.ptr, gate as i32, -2, b, l) })
    }

    /// A gate's input pin names, in the gate's pin order.
    pub fn gate_pins(&self, gate: usize) -> Vec<String> {
        let n = unsafe { ffi::vyges_abc_mio_gate_pin_count(self.ptr, gate as i32) }.max(0);
        (0..n).filter_map(|k| read_name(|b, l| unsafe { ffi::vyges_abc_mio_name(self.ptr, gate as i32, k, b, l) })).collect()
    }

    /// `Mio_LibraryReadConst0` / `Const1` / `Buf`'s gate, if any.
    pub fn special(&self, which: Special) -> Option<usize> {
        let w = match which {
            Special::Const0 => 0,
            Special::Const1 => 1,
            Special::Buffer => 2,
        };
        usize::try_from(unsafe { ffi::vyges_abc_mio_special_gate(self.ptr, w) }).ok()
    }

    /// Renames a gate and its output pin in place.
    pub fn rename(&mut self, gate: usize, name: &str, out_name: &str) -> Result<(), Error> {
        let (n, o) = (c_str(name)?, c_str(out_name)?);
        match unsafe { ffi::vyges_abc_mio_rename(self.ptr, gate as i32, n.as_ptr(), o.as_ptr()) } {
            0 => Ok(()),
            _ => Err(failed("rename gate")),
        }
    }
}

/// `Abc_ObjType_t`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjType {
    Const1,
    Pi,
    Po,
    Net,
    Node,
    Other(i32),
}

/// A network (`Abc_Ntk_t`); objects are named by their ids. Deleted on drop.
pub struct Network {
    ptr: *mut c_void,
}

/// `Abc_NtkMap`'s parameters, in its order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapParams {
    pub delay_target: f64,
    pub area_multi: f64,
    pub delay_multi: f64,
    pub log_fan: f32,
    pub slew: f32,
    pub gain: f32,
    pub gates_min: i32,
    pub recovery: bool,
    pub switching: bool,
    pub skip_fanout: bool,
    pub use_profile: bool,
    pub use_buffs: bool,
    pub verbose: bool,
}

/// `Abc_SclBufferingPerform`'s parameters but the slew (the library's average slew).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferParams {
    pub gain_ratio: i32,
    pub degree: i32,
    pub size_only: bool,
    pub add_bufs: bool,
    pub buf_pis: bool,
    pub use_wire_loads: bool,
    pub verbose: bool,
    pub very_verbose: bool,
}

impl Network {
    fn wrap(ptr: *mut c_void, call: &str) -> Result<Network, Error> {
        if ptr.is_null() {
            return Err(failed(call));
        }
        Ok(Network { ptr })
    }

    /// `Abc_NtkAlloc(netlist, mapped, with a memory manager)` over gate library `lib`.
    pub fn mapped_netlist(_abc: &Abc, lib: GateLib) -> Result<Network, Error> {
        Self::wrap(unsafe { ffi::vyges_abc_ntk_alloc_mapped_netlist(lib.ptr) }, "Abc_NtkAlloc")
    }

    fn create(&mut self, kind: i32, gate: i32, call: &str) -> Result<usize, Error> {
        usize::try_from(unsafe { ffi::vyges_abc_ntk_create(self.ptr, kind, gate) }).map_err(|_| failed(call))
    }

    /// `Abc_NtkCreatePi`.
    pub fn create_pi(&mut self) -> Result<usize, Error> {
        self.create(0, 0, "Abc_NtkCreatePi")
    }

    /// `Abc_NtkCreatePo`.
    pub fn create_po(&mut self) -> Result<usize, Error> {
        self.create(1, 0, "Abc_NtkCreatePo")
    }

    /// `Abc_NtkCreateNet`.
    pub fn create_net(&mut self) -> Result<usize, Error> {
        self.create(2, 0, "Abc_NtkCreateNet")
    }

    /// `Abc_NtkCreateNode`, gate `gate` of the network's library its data.
    pub fn create_node(&mut self, gate: usize) -> Result<usize, Error> {
        self.create(3, gate as i32, "Abc_NtkCreateNode")
    }

    /// `Abc_ObjAssignName(obj, name, no suffix)`.
    pub fn assign_name(&mut self, obj: usize, name: &str) -> Result<(), Error> {
        let n = c_str(name)?;
        match unsafe { ffi::vyges_abc_obj_assign_name(self.ptr, obj as i32, n.as_ptr()) } {
            0 => Ok(()),
            _ => Err(failed("Abc_ObjAssignName")),
        }
    }

    /// `Abc_ObjAddFanin(obj, fanin)`.
    pub fn add_fanin(&mut self, obj: usize, fanin: usize) -> Result<(), Error> {
        match unsafe { ffi::vyges_abc_obj_add_fanin(self.ptr, obj as i32, fanin as i32) } {
            0 => Ok(()),
            _ => Err(failed("Abc_ObjAddFanin")),
        }
    }

    /// `Abc_NtkToLogic`.
    pub fn to_logic(&self) -> Result<Network, Error> {
        Self::wrap(unsafe { ffi::vyges_abc_ntk_to_logic(self.ptr) }, "Abc_NtkToLogic")
    }

    /// `Abc_NtkStrash`.
    pub fn strash(&self, all_nodes: bool, cleanup: bool, record: bool) -> Result<Network, Error> {
        Self::wrap(unsafe { ffi::vyges_abc_ntk_strash(self.ptr, all_nodes as i32, cleanup as i32, record as i32) }, "Abc_NtkStrash")
    }

    /// `Abc_NtkBalance`.
    pub fn balance(&self, duplicate: bool, selective: bool, update_level: bool) -> Result<Network, Error> {
        Self::wrap(unsafe { ffi::vyges_abc_ntk_balance(self.ptr, duplicate as i32, selective as i32, update_level as i32) }, "Abc_NtkBalance")
    }

    /// `Abc_NtkToNetlist`.
    pub fn to_netlist(&self) -> Result<Network, Error> {
        Self::wrap(unsafe { ffi::vyges_abc_ntk_to_netlist(self.ptr) }, "Abc_NtkToNetlist")
    }

    /// `Abc_FrameSetLibGen` with this network's gate library, and `Abc_FrameSetDrivingCell` with
    /// that library's buffer gate. Fails if the library has no buffer.
    pub fn install_library(&self, _abc: &mut Abc) -> Result<(), Error> {
        match unsafe { ffi::vyges_abc_frame_set_lib_and_driver(self.ptr) } {
            0 => Ok(()),
            _ => Err(failed("Mio_LibraryReadBuf")),
        }
    }

    /// `Abc_NtkMap` with the frame's gate library. Prints what ABC prints.
    pub fn map(&self, p: &MapParams) -> Result<Network, Error> {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let ptr = unsafe {
            ffi::vyges_abc_ntk_map(
                self.ptr,
                p.delay_target,
                p.area_multi,
                p.delay_multi,
                p.log_fan,
                p.slew,
                p.gain,
                p.gates_min,
                p.recovery as i32,
                p.switching as i32,
                p.skip_fanout as i32,
                p.use_profile as i32,
                p.use_buffs as i32,
                p.verbose as i32,
            )
        };
        Self::wrap(ptr, "Abc_NtkMap")
    }

    /// `Abc_NtkCleanup`: the number of nodes removed.
    pub fn cleanup(&mut self, verbose: bool) -> Result<usize, Error> {
        usize::try_from(unsafe { ffi::vyges_abc_ntk_cleanup(self.ptr, verbose as i32) }).map_err(|_| failed("Abc_NtkCleanup"))
    }

    /// `Abc_SclBufferingPerform` over cell library `lib`, at its average slew. Prints what ABC
    /// prints.
    pub fn buffering(&self, lib: &CellLib, p: &BufferParams) -> Result<Network, Error> {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let ptr = unsafe {
            ffi::vyges_abc_scl_buffering(
                self.ptr,
                lib.ptr,
                p.gain_ratio,
                p.degree,
                p.size_only as i32,
                p.add_bufs as i32,
                p.buf_pis as i32,
                p.use_wire_loads as i32,
                p.verbose as i32,
                p.very_verbose as i32,
            )
        };
        Self::wrap(ptr, "Abc_SclBufferingPerform")
    }

    /// Whether this is a netlist with a mapping.
    pub fn is_mapped_netlist(&self) -> bool {
        unsafe { ffi::vyges_abc_ntk_is_mapped_netlist(self.ptr) == 1 }
    }

    /// The network's gate library.
    pub fn gate_lib(&self) -> GateLib {
        GateLib { ptr: unsafe { ffi::vyges_abc_ntk_mio(self.ptr) } }
    }

    /// `Abc_NtkObjNumMax`.
    pub fn obj_num_max(&self) -> usize {
        unsafe { ffi::vyges_abc_ntk_obj_num_max(self.ptr) }.max(0) as usize
    }

    /// An object's type; `None` for an empty slot.
    pub fn obj_type(&self, obj: usize) -> Option<ObjType> {
        match unsafe { ffi::vyges_abc_obj_type(self.ptr, obj as i32) } {
            -1 => None,
            1 => Some(ObjType::Const1),
            2 => Some(ObjType::Pi),
            3 => Some(ObjType::Po),
            6 => Some(ObjType::Net),
            7 => Some(ObjType::Node),
            t => Some(ObjType::Other(t)),
        }
    }

    /// `Abc_ObjName`.
    pub fn obj_name(&self, obj: usize) -> Option<String> {
        read_name(|b, l| unsafe { ffi::vyges_abc_obj_name(self.ptr, obj as i32, b, l) })
    }

    /// An object's fanin ids, in order.
    pub fn fanins(&self, obj: usize) -> Vec<usize> {
        let n = unsafe { ffi::vyges_abc_obj_fanin_count(self.ptr, obj as i32) }.max(0);
        (0..n).map(|k| unsafe { ffi::vyges_abc_obj_fanin(self.ptr, obj as i32, k) } as usize).collect()
    }

    /// An object's fanout ids, in order.
    pub fn fanouts(&self, obj: usize) -> Vec<usize> {
        let n = unsafe { ffi::vyges_abc_obj_fanout_count(self.ptr, obj as i32) }.max(0);
        (0..n).map(|k| unsafe { ffi::vyges_abc_obj_fanout(self.ptr, obj as i32, k) } as usize).collect()
    }

    /// A node's gate, as its index in the network's gate library.
    pub fn obj_gate(&self, obj: usize) -> Option<usize> {
        usize::try_from(unsafe { ffi::vyges_abc_obj_gate(self.ptr, obj as i32) }).ok()
    }
}

impl Drop for Network {
    fn drop(&mut self) {
        unsafe { ffi::vyges_abc_ntk_delete(self.ptr) };
    }
}

/// `Abc_SclUpsizePerform` / `Abc_SclDnsizePerform`'s parameters set (the rest zero).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SizeParams {
    pub iters: i32,
    pub iter_no_change: i32,
    pub window: i32,
    pub ratio: i32,
    pub notches: i32,
    pub use_dept: bool,
}

/// Up- or down-sizing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resize {
    Up,
    Down,
}

impl Network {
    /// `Abc_NtkIsStrash`.
    pub fn is_strash(&self) -> bool {
        unsafe { ffi::vyges_abc_ntk_is_strash(self.ptr) != 0 }
    }

    /// Sets the network's `pManFunc` to gate library `lib`.
    pub fn set_gate_lib(&mut self, lib: GateLib) {
        unsafe { ffi::vyges_abc_ntk_set_man_func(self.ptr, lib.ptr) };
    }

    /// `Abc_NtkDupDfs`.
    pub fn dup_dfs(&self) -> Result<Network, Error> {
        Self::wrap(unsafe { ffi::vyges_abc_ntk_dup_dfs(self.ptr) }, "Abc_NtkDupDfs")
    }

    /// Gate sizing in place over cell library `lib`.
    pub fn resize(&mut self, lib: &CellLib, which: Resize, p: &SizeParams) -> Result<(), Error> {
        let up = matches!(which, Resize::Up) as i32;
        match unsafe { ffi::vyges_abc_scl_resize(lib.ptr, self.ptr, up, p.iters, p.iter_no_change, p.window, p.ratio, p.notches, p.use_dept as i32) } {
            0 => Ok(()),
            _ => Err(failed("Abc_SclUpsizePerform / Abc_SclDnsizePerform")),
        }
    }

    /// `Abc_NtkFromCellMappedGia`.
    pub fn from_cell_mapped_gia(gia: &Gia, use_buffs: bool) -> Result<Network, Error> {
        Self::wrap(unsafe { ffi::vyges_abc_ntk_from_cell_mapped_gia(gia.ptr, use_buffs as i32) }, "Abc_NtkFromCellMappedGia")
    }

    /// `Abc_NtkFromMappedGia`.
    pub fn from_mapped_gia(gia: &Gia, find_enables: bool, use_buffs: bool) -> Result<Network, Error> {
        Self::wrap(unsafe { ffi::vyges_abc_ntk_from_mapped_gia(gia.ptr, find_enables as i32, use_buffs as i32) }, "Abc_NtkFromMappedGia")
    }

    /// `Gia_ManToAig(gia, 0)` then `Abc_NtkFromAigPhase`, the AIG's name kept.
    pub fn from_gia_aig(gia: &Gia) -> Result<Network, Error> {
        Self::wrap(unsafe { ffi::vyges_abc_ntk_from_gia_aig(gia.ptr) }, "Abc_NtkFromAigPhase")
    }

    /// Every CI and CO renamed after `gia`'s name vectors, in order.
    pub fn name_from_gia(&mut self, gia: &Gia) -> Result<(), Error> {
        match unsafe { ffi::vyges_abc_ntk_name_from_gia(self.ptr, gia.ptr) } {
            0 => Ok(()),
            _ => Err(failed("Abc_ObjAssignName from the GIA names")),
        }
    }

    /// `Abc_NtkRedirectCiCo`.
    pub fn redirect_ci_co(&mut self) -> Result<(), Error> {
        match unsafe { ffi::vyges_abc_ntk_redirect_ci_co(self.ptr) } {
            0 => Ok(()),
            _ => Err(failed("Abc_NtkRedirectCiCo")),
        }
    }
}

/// An AIG manager (`Gia_Man_t`); every rewrite replaces it, the new one taking over the names it
/// lacks. Freed on drop.
pub struct Gia {
    ptr: *mut c_void,
}

/// What a manager carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GiaHas {
    Choices,
    Mapping,
    CellMapping,
    Muxes,
    Dangling,
}

impl Gia {
    /// From a logic network: strashed, through an AIG, latches zeroed, CI and CO names kept.
    pub fn from_network(_abc: &Abc, ntk: &Network) -> Result<Gia, Error> {
        let ptr = unsafe { ffi::vyges_abc_gia_from_network(ntk.ptr) };
        if ptr.is_null() {
            return Err(failed("Gia_ManFromAig"));
        }
        Ok(Gia { ptr })
    }

    pub fn has(&self, what: GiaHas) -> bool {
        let w = match what {
            GiaHas::Choices => 0,
            GiaHas::Mapping => 1,
            GiaHas::CellMapping => 2,
            GiaHas::Muxes => 3,
            GiaHas::Dangling => 4,
        };
        unsafe { ffi::vyges_abc_gia_query(self.ptr, w) > 0 }
    }

    fn replace(&mut self, next: *mut c_void, call: &str) -> Result<(), Error> {
        if next.is_null() {
            return Err(failed(call));
        }
        self.ptr = unsafe { ffi::vyges_abc_gia_replace(self.ptr, next) };
        Ok(())
    }

    /// `Gia_ManRehash`.
    pub fn rehash(&mut self, add_strash: bool) -> Result<(), Error> {
        self.replace(unsafe { ffi::vyges_abc_gia_rehash(self.ptr, add_strash as i32) }, "Gia_ManRehash")
    }

    /// `Gia_ManPerformDch` over `Dch_ManSetDefaultParams`, `nBTLimit` overridden when given.
    pub fn dch(&mut self, bt_limit: Option<i32>) -> Result<(), Error> {
        self.replace(unsafe { ffi::vyges_abc_gia_dch(self.ptr, bt_limit.unwrap_or(-1)) }, "Gia_ManPerformDch")
    }

    /// `Gia_ManEquivReduce`.
    pub fn equiv_reduce(&mut self, use_all: bool, dual_out: bool, skip_phase: bool, verbose: bool) -> Result<(), Error> {
        self.replace(unsafe { ffi::vyges_abc_gia_equiv_reduce(self.ptr, use_all as i32, dual_out as i32, skip_phase as i32, verbose as i32) }, "Gia_ManEquivReduce")
    }

    /// `Gia_ManAigSyn2`.
    #[allow(clippy::too_many_arguments)]
    pub fn syn2(&mut self, old_algo: bool, coarsen: bool, cut_min: bool, relax_ratio: i32, delay_min: bool, verbose: bool, very_verbose: bool) -> Result<(), Error> {
        let next = unsafe { ffi::vyges_abc_gia_syn2(self.ptr, old_algo as i32, coarsen as i32, cut_min as i32, relax_ratio, delay_min as i32, verbose as i32, very_verbose as i32) };
        self.replace(next, "Gia_ManAigSyn2")
    }

    /// `Gia_ManAigSyn3`.
    pub fn syn3(&mut self, verbose: bool, very_verbose: bool) -> Result<(), Error> {
        self.replace(unsafe { ffi::vyges_abc_gia_syn3(self.ptr, verbose as i32, very_verbose as i32) }, "Gia_ManAigSyn3")
    }

    /// `Gia_ManAigSyn4`.
    pub fn syn4(&mut self, verbose: bool, very_verbose: bool) -> Result<(), Error> {
        self.replace(unsafe { ffi::vyges_abc_gia_syn4(self.ptr, verbose as i32, very_verbose as i32) }, "Gia_ManAigSyn4")
    }

    /// `Gia_ManRetimeForward`.
    pub fn retime_forward(&mut self, max_iters: i32, verbose: bool) -> Result<(), Error> {
        self.replace(unsafe { ffi::vyges_abc_gia_retime_forward(self.ptr, max_iters, verbose as i32) }, "Gia_ManRetimeForward")
    }

    /// `Gia_ManCompress2`.
    pub fn compress2(&mut self, update_level: bool, verbose: bool) -> Result<(), Error> {
        self.replace(unsafe { ffi::vyges_abc_gia_compress2(self.ptr, update_level as i32, verbose as i32) }, "Gia_ManCompress2")
    }

    /// `Gia_ManAreaBalance`.
    pub fn area_balance(&mut self, simple_and: bool, new_nodes_max: i32, verbose: bool, very_verbose: bool) -> Result<(), Error> {
        self.replace(unsafe { ffi::vyges_abc_gia_area_balance(self.ptr, simple_and as i32, new_nodes_max, verbose as i32, very_verbose as i32) }, "Gia_ManAreaBalance")
    }

    /// `Gia_ManBalance`.
    pub fn balance(&mut self, simple_and: bool, strict: bool, verbose: bool) -> Result<(), Error> {
        self.replace(unsafe { ffi::vyges_abc_gia_balance(self.ptr, simple_and as i32, strict as i32, verbose as i32) }, "Gia_ManBalance")
    }

    /// `Gia_ManEquivReduceAndRemap`.
    pub fn equiv_reduce_and_remap(&mut self, seq: bool, miter_pairs: bool) -> Result<(), Error> {
        self.replace(unsafe { ffi::vyges_abc_gia_equiv_reduce_and_remap(self.ptr, seq as i32, miter_pairs as i32) }, "Gia_ManEquivReduceAndRemap")
    }

    /// `Gia_ManPerformMapping` over `Gia_ManSetIfParsDefault` with these fields.
    pub fn if_map(&mut self, delay_opt: bool, lut_size: i32, truth: bool, cut_min: bool, exp_red: bool) -> Result<(), Error> {
        let next = unsafe { ffi::vyges_abc_gia_if_map(self.ptr, delay_opt as i32, lut_size, truth as i32, cut_min as i32, exp_red as i32) };
        self.replace(next, "Gia_ManPerformMapping")
    }

    /// `Gia_ManAigSynch2` over `Dch_ManSetDefaultParams` with `nBTLimit`.
    pub fn synch2(&mut self, bt_limit: i32, lut_size: i32, relax_ratio: i32) -> Result<(), Error> {
        self.replace(unsafe { ffi::vyges_abc_gia_synch2(self.ptr, bt_limit, lut_size, relax_ratio) }, "Gia_ManAigSynch2")
    }
}

/// One object of a manager, as `Gia_ManObj` reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GiaObj {
    Const0,
    Ci,
    /// Its fanin: object id, complemented.
    Co(i32, bool),
    /// Its two fanins (`Gia_ObjIsAnd`): object id, complemented.
    And(i32, bool, i32, bool),
}

impl Gia {
    /// `Gia_ManStart(n_objs)` named `name`, its structural hash table allocated: an empty manager
    /// to build by hand.
    pub fn start(_abc: &Abc, n_objs: i32, name: &str) -> Result<Gia, Error> {
        let c = CString::new(name).map_err(|_| Error::Nul(name.to_string()))?;
        let ptr = unsafe { ffi::vyges_abc_gia_start(n_objs, c.as_ptr() as *const c_char) };
        if ptr.is_null() {
            return Err(failed("Gia_ManStart"));
        }
        Ok(Gia { ptr })
    }

    /// `Gia_ManAppendCi`: the new CI's literal.
    pub fn append_ci(&mut self) -> i32 {
        unsafe { ffi::vyges_abc_gia_append_ci(self.ptr) }
    }

    /// `Gia_ManHashAnd`: the AND of two literals, structurally hashed.
    pub fn hash_and(&mut self, lit0: i32, lit1: i32) -> i32 {
        unsafe { ffi::vyges_abc_gia_hash_and(self.ptr, lit0, lit1) }
    }

    /// `Gia_ManAppendCo`: the new CO's literal.
    pub fn append_co(&mut self, lit: i32) -> i32 {
        unsafe { ffi::vyges_abc_gia_append_co(self.ptr, lit) }
    }

    /// `Gia_ManCleanup`: the manager without its dangling nodes (a new one; this one freed).
    pub fn cleanup(&mut self) -> Result<(), Error> {
        let next = unsafe { ffi::vyges_abc_gia_cleanup(self.ptr) };
        if next.is_null() {
            return Err(failed("Gia_ManCleanup"));
        }
        self.ptr = next;
        Ok(())
    }

    /// Objects (`nObjs`), the constant and every CI, CO and AND among them.
    pub fn object_count(&self) -> i32 {
        unsafe { ffi::vyges_abc_gia_count(self.ptr, 0) }
    }

    /// `Gia_ManAndNum`.
    pub fn and_count(&self) -> i32 {
        unsafe { ffi::vyges_abc_gia_count(self.ptr, 1) }
    }

    /// `Gia_ManCiNum`.
    pub fn ci_count(&self) -> i32 {
        unsafe { ffi::vyges_abc_gia_count(self.ptr, 2) }
    }

    /// `Gia_ManCoNum`.
    pub fn co_count(&self) -> i32 {
        unsafe { ffi::vyges_abc_gia_count(self.ptr, 3) }
    }

    /// The object id of the `i`-th CI.
    pub fn ci_id(&self, i: i32) -> Option<i32> {
        let id = unsafe { ffi::vyges_abc_gia_ci_id(self.ptr, i) };
        (id >= 0).then_some(id)
    }

    /// The object id of the `i`-th CO.
    pub fn co_id(&self, i: i32) -> Option<i32> {
        let id = unsafe { ffi::vyges_abc_gia_co_id(self.ptr, i) };
        (id >= 0).then_some(id)
    }

    /// Object `id`.
    pub fn object(&self, id: i32) -> Option<GiaObj> {
        let (mut f0, mut c0, mut f1, mut c1) = (0, 0, 0, 0);
        match unsafe { ffi::vyges_abc_gia_obj(self.ptr, id, &mut f0, &mut c0, &mut f1, &mut c1) } {
            0 => Some(GiaObj::Const0),
            1 => Some(GiaObj::Ci),
            2 => Some(GiaObj::Co(f0, c0 != 0)),
            3 => Some(GiaObj::And(f0, c0 != 0, f1, c1 != 0)),
            _ => None,
        }
    }
}

impl Abc {
    /// `Abc_FrameUpdateGia`: the frame's current manager becomes `gia`.
    pub fn set_gia(&mut self, gia: Gia) {
        unsafe { ffi::vyges_abc_frame_update_gia(gia.ptr) };
        std::mem::forget(gia);
    }

    /// `Abc_FrameGetGia`: the frame's current manager, taken from it.
    pub fn take_gia(&mut self) -> Option<Gia> {
        let ptr = unsafe { ffi::vyges_abc_frame_get_gia() };
        (!ptr.is_null()).then_some(Gia { ptr })
    }
}

impl Drop for Gia {
    fn drop(&mut self) {
        unsafe { ffi::vyges_abc_gia_free(self.ptr) };
    }
}

impl Abc {
    /// `Abc_FrameSetLibGen` alone.
    pub fn set_lib_gen(&mut self, lib: GateLib) -> Result<(), Error> {
        match unsafe { ffi::vyges_abc_frame_set_lib_gen(lib.ptr) } {
            0 => Ok(()),
            _ => Err(failed("Abc_FrameSetLibGen")),
        }
    }

    /// `Extra_UtilGetoptReset`.
    pub fn getopt_reset(&mut self) {
        unsafe { ffi::vyges_abc_getopt_reset() };
    }
}

/// Standard output (C and Rust) sent to `/dev/null` while held; restored on drop.
pub struct Quiet {
    saved: i32,
}

impl Quiet {
    pub fn new() -> Quiet {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        Quiet { saved: unsafe { ffi::vyges_abc_stdout_suppress() } }
    }
}

impl Default for Quiet {
    fn default() -> Quiet {
        Quiet::new()
    }
}

impl Drop for Quiet {
    fn drop(&mut self) {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        unsafe { ffi::vyges_abc_stdout_restore(self.saved) };
    }
}
