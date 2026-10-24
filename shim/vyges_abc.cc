// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Vyges

#include "shim/vyges_abc.h"

// ⚠️ Order matters: cmd.h uses Abc_Frame_t without declaring it, so main.h comes first.
#include "base/abc/abc.h"
#include "base/main/main.h"
#include "base/cmd/cmd.h"
#include "misc/util/utilCex.h"

int vyges_abc_start(void) {
  try {
    abc::Abc_Start();
    return abc::Abc_FrameGetGlobalFrame() ? 0 : -1;
  } catch (...) {
    return -1;
  }
}

void vyges_abc_stop(void) {
  try {
    abc::Abc_Stop();
  } catch (...) {
  }
}

int vyges_abc_execute(const char* command) {
  try {
    return abc::Cmd_CommandExecute(abc::Abc_FrameGetGlobalFrame(), command);
  } catch (...) {
    return -1;
  }
}

int vyges_abc_prob_status(void) {
  try {
    return abc::Abc_FrameReadProbStatus(abc::Abc_FrameGetGlobalFrame());
  } catch (...) {
    return -1;
  }
}

int vyges_abc_cex(int* po, int* frame) {
  try {
    auto* cex = static_cast<abc::Abc_Cex_t*>(abc::Abc_FrameReadCex(abc::Abc_FrameGetGlobalFrame()));
    if (cex == nullptr) {
      return 0;
    }
    *po = cex->iPo;
    *frame = cex->iFrame;
    return 1;
  } catch (...) {
    return 0;
  }
}

int vyges_abc_network_counts(int* pis, int* pos, int* latches, int* nodes) {
  try {
    abc::Abc_Ntk_t* ntk = abc::Abc_FrameReadNtk(abc::Abc_FrameGetGlobalFrame());
    if (ntk == nullptr) {
      return 0;
    }
    *pis = abc::Abc_NtkPiNum(ntk);
    *pos = abc::Abc_NtkPoNum(ntk);
    *latches = abc::Abc_NtkLatchNum(ntk);
    *nodes = abc::Abc_NtkNodeNum(ntk);
    return 1;
  } catch (...) {
    return 0;
  }
}
