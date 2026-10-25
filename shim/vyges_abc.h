// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Vyges
//
// C API over ABC's global frame. One function per ABC call; nothing is batched or reordered.
//
// ABC keeps ONE process-wide frame (the current network, the current AIG, the last proof status and
// counterexample), so these functions are not thread-safe; the Rust crate serialises them.
// Every function catches any C++ exception and reports it as -1; none crosses this boundary.
#ifndef VYGES_ABC_H
#define VYGES_ABC_H

#ifdef __cplusplus
extern "C" {
#endif

// Abc_Start. 0 on success, -1 on failure.
int vyges_abc_start(void);

// Abc_Stop.
void vyges_abc_stop(void);

// Cmd_CommandExecute on the global frame: its return value (0 = the command succeeded), or -1 if
// it threw. The command's own output goes to stdout, as ABC prints it.
int vyges_abc_execute(const char* command);

// vyges_abc_execute with the process's standard output (file descriptor 1) redirected to the file
// `path` (created or truncated) for the duration of the command, then restored. ABC prints its
// results and verdicts to standard output; this keeps them out of the caller's own output.
// Returns as vyges_abc_execute, or -2 if the redirection could not be set up.
int vyges_abc_execute_captured(const char* command, const char* path);

// Abc_FrameReadProbStatus: -1 undecided, 0 satisfiable (property fails / not equivalent; a
// counterexample may be present), 1 unsatisfiable (property holds / equivalent).
int vyges_abc_prob_status(void);

// The counterexample the last proof command left (Abc_FrameReadCex): 1 if there is one and
// *po / *frame are set to its failing output and time frame, 0 if there is none.
int vyges_abc_cex(int* po, int* frame);

// The primary-input values of the last counterexample in time frame `frame` (0-based), in the
// order of the counterexample's primary inputs: writes min(len, nPis) values (0 or 1) into `bits`
// and returns nPis, or -1 if there is no counterexample or `frame` is outside it.
int vyges_abc_cex_inputs(int frame, int* bits, int len);

// The current logic network's counts: primary inputs, primary outputs, latches, internal nodes.
// 1 on success, 0 if there is no current network.
int vyges_abc_network_counts(int* pis, int* pos, int* latches, int* nodes);

// The name of the current network's primary input (kind 0) or primary output (kind 1) number
// `index`: copies it NUL-terminated into `buf` (truncated to len - 1 bytes) and returns its full
// length, or -1 if there is no network or no such signal.
int vyges_abc_network_name(int kind, int index, char* buf, int len);

// ---- Standard-cell library (SC_Lib), built cell by cell as a caller reads its Liberty ----
// A library handle is an `SC_Lib*`; pins and timings are added to the LAST cell added, and a
// surface to the last timing of its last output pin. Values arrive in the library's own units.

// Abc_SclLibAlloc, then its name, file name, units and default max output slew. NULL on failure.
void* vyges_abc_scl_lib_alloc(const char* name, const char* file_name, float unit_cap_fst,
                              int unit_cap_snd, int unit_time, float default_max_out_slew);

// Abc_SclCellAlloc, its Id the cell count, pushed to the library; drive strength 0. `has_leakage`
// 0 leaves the leakage 0. Returns the cell's index, or -1.
int vyges_abc_scl_cell_add(void* lib, const char* name, float area, int has_leakage, float leakage);

// Abc_SclPinAlloc as an input pin of the last cell, with its rise and fall capacitance. 0 or -1.
int vyges_abc_scl_input_pin_add(void* lib, const char* name, float rise_cap, float fall_cap);

// Abc_SclPinAlloc as an output pin of the last cell: its limits where they exist, its function
// text, and that function's truth table over the cell's input pins (Mio_ParseFormulaTruth).
// 0, or -1 (-2: the formula does not parse).
int vyges_abc_scl_output_pin_add(void* lib, const char* name, int has_max_cap, float max_out_cap,
                                 int has_max_slew, float max_out_slew, const char* func_text);

// A timing (Abc_SclTimingsAlloc + Abc_SclTimingAlloc) on the last output pin, from `related_pin`,
// with sense 0 positive unate, 1 negative unate, 2 non unate. 0 or -1.
int vyges_abc_scl_timing_add(void* lib, const char* related_pin, int sense);

// One surface of the last timing — `which` 0 cell rise, 1 cell fall, 2 rise transition, 3 fall
// transition — its two axes and its n0 x n1 values row by row. Each value is kept as given and
// as Scl_Flt2Int of it. 0 or -1.
int vyges_abc_scl_surface(void* lib, int which, int n0, const float* index0, int n1,
                          const float* index1, const float* values);

// Abc_SclLibNormalize, Abc_SclHashCells, Abc_SclLinkCells. 0 or -1.
int vyges_abc_scl_lib_finish(void* lib);

// Abc_SclLibFree.
void vyges_abc_scl_lib_free(void* lib);

// Abc_SclComputeAverageSlew.
float vyges_abc_scl_average_slew(void* lib);

// SC_LibCellNum, and cell `cell`'s input and output counts and area. -1 if there is no such cell.
int vyges_abc_scl_cell_count(void* lib);
int vyges_abc_scl_cell_info(void* lib, int cell, int* n_inputs, int* n_outputs, float* area);

// The name of cell `cell` (pin < 0) or of its pin `pin`, copied as vyges_abc_network_name does.
int vyges_abc_scl_name(void* lib, int cell, int pin, char* buf, int len);

// The first word of pin `pin`'s truth table (its whole table when the cell has under 7 inputs).
// 1, or 0 if there is no such pin or the table is empty.
int vyges_abc_scl_pin_truth(void* lib, int cell, int pin, unsigned long long* word);

// ---- Gate library (Mio) ----
// Gates are numbered in the library's gate-list order.

// Abc_SclDeriveGenlibSimple: a gate library from a finished cell library (it prints a line).
void* vyges_abc_scl_derive_genlib(void* lib);

// Mio_LibraryDelete.
void vyges_abc_mio_free(void* mio);

// The number of gates; a gate's pin count; -1 if there is no such gate.
int vyges_abc_mio_gate_count(void* mio);
int vyges_abc_mio_gate_pin_count(void* mio, int gate);

// A gate's name (pin -1), its output pin's name (pin -2), or the name of its input pin `pin`.
int vyges_abc_mio_name(void* mio, int gate, int pin, char* buf, int len);

// The gate Mio_LibraryReadConst0 (value 0), Mio_LibraryReadConst1 (1) or Mio_LibraryReadBuf (2)
// returns, or -1 if there is none.
int vyges_abc_mio_special_gate(void* mio, int which);

// Renames a gate and its output pin (both copied). 0 or -1.
int vyges_abc_mio_rename(void* mio, int gate, const char* name, const char* out_name);

// ---- Networks ----
// A network handle is an `Abc_Ntk_t*`; objects are named by their ids.

// Abc_NtkAlloc(netlist, mapped, memory manager) with `mio` as its gate library. NULL on failure.
void* vyges_abc_ntk_alloc_mapped_netlist(void* mio);

// Abc_NtkDelete.
void vyges_abc_ntk_delete(void* ntk);

// Abc_NtkCreatePi (kind 0), Abc_NtkCreatePo (1), Abc_NtkCreateNet (2), or Abc_NtkCreateNode with
// gate `gate` as its data (3). The object's id, or -1.
int vyges_abc_ntk_create(void* ntk, int kind, int gate);

// Abc_ObjAssignName (no suffix); Abc_ObjAddFanin. 0 or -1.
int vyges_abc_obj_assign_name(void* ntk, int obj, const char* name);
int vyges_abc_obj_add_fanin(void* ntk, int obj, int fanin);

// Abc_NtkToLogic, Abc_NtkStrash, Abc_NtkBalance, Abc_NtkToNetlist: the new network, or NULL.
// The input network is left to the caller.
void* vyges_abc_ntk_to_logic(void* ntk);
void* vyges_abc_ntk_strash(void* ntk, int all_nodes, int cleanup, int record);
void* vyges_abc_ntk_balance(void* ntk, int duplicate, int selective, int update_level);
void* vyges_abc_ntk_to_netlist(void* ntk);

// Abc_FrameSetLibGen with the network's gate library, and Abc_FrameSetDrivingCell with a copy of
// that library's buffer gate's name. 0, or -1 if the library has no buffer.
int vyges_abc_frame_set_lib_and_driver(void* ntk);

// Abc_NtkMap with no user library (the frame's), then the new network, or NULL.
void* vyges_abc_ntk_map(void* ntk, double delay_target, double area_multi, double delay_multi,
                        float log_fan, float slew, float gain, int n_gates_min, int recovery,
                        int switching, int skip_fanout, int use_profile, int use_buffs, int verbose);

// Abc_NtkCleanup: the number of nodes removed, or -1.
int vyges_abc_ntk_cleanup(void* ntk, int verbose);

// Abc_SclBufferingPerform with these parameters, the slew Abc_SclComputeAverageSlew(lib)
// converted as the parameter's int field takes it. The new network, or NULL.
void* vyges_abc_scl_buffering(void* ntk, void* lib, int gain_ratio, int degree, int size_only,
                              int add_bufs, int buf_pis, int use_wire_loads, int verbose,
                              int very_verbose);

// 1 if the network is a netlist with a mapping (Abc_NtkIsNetlist && Abc_NtkHasMapping).
int vyges_abc_ntk_is_mapped_netlist(void* ntk);

// The network's gate library (its pManFunc).
void* vyges_abc_ntk_mio(void* ntk);

// Abc_NtkObjNumMax; an object's type (Abc_ObjType_t; -1 for an empty slot); its name
// (Abc_ObjName, copied as vyges_abc_network_name does); its fanin and fanout counts and ids; a
// node's gate (its index in the network's library; -1 if none).
int vyges_abc_ntk_obj_num_max(void* ntk);
int vyges_abc_obj_type(void* ntk, int obj);
int vyges_abc_obj_name(void* ntk, int obj, char* buf, int len);
int vyges_abc_obj_fanin_count(void* ntk, int obj);
int vyges_abc_obj_fanin(void* ntk, int obj, int k);
int vyges_abc_obj_fanout_count(void* ntk, int obj);
int vyges_abc_obj_fanout(void* ntk, int obj, int k);
int vyges_abc_obj_gate(void* ntk, int obj);

#ifdef __cplusplus
}
#endif

#endif  // VYGES_ABC_H
