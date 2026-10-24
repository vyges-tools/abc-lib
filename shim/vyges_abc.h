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

#ifdef __cplusplus
}
#endif

#endif  // VYGES_ABC_H
