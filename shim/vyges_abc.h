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

// Abc_FrameReadProbStatus: -1 undecided, 0 satisfiable (property fails / not equivalent; a
// counterexample may be present), 1 unsatisfiable (property holds / equivalent).
int vyges_abc_prob_status(void);

// The counterexample the last proof command left (Abc_FrameReadCex): 1 if there is one and
// *po / *frame are set to its failing output and time frame, 0 if there is none.
int vyges_abc_cex(int* po, int* frame);

// The current logic network's counts: primary inputs, primary outputs, latches, internal nodes.
// 1 on success, 0 if there is no current network.
int vyges_abc_network_counts(int* pis, int* pos, int* latches, int* nodes);

#ifdef __cplusplus
}
#endif

#endif  // VYGES_ABC_H
