// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Vyges

#include "shim/vyges_abc.h"

// ⚠️ Order matters: cmd.h uses Abc_Frame_t without declaring it, so main.h comes first.
#include "base/abc/abc.h"
#include "base/main/main.h"
#include "base/cmd/cmd.h"
#include "misc/util/utilCex.h"

#include <fcntl.h>
#include <unistd.h>

#include <cstdio>
#include <cstring>

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

int vyges_abc_execute_captured(const char* command, const char* path) {
  std::fflush(stdout);
  const int saved = dup(STDOUT_FILENO);
  if (saved < 0) {
    return -2;
  }
  const int fd = open(path, O_WRONLY | O_CREAT | O_TRUNC, 0600);
  if (fd < 0 || dup2(fd, STDOUT_FILENO) < 0) {
    if (fd >= 0) {
      close(fd);
    }
    close(saved);
    return -2;
  }
  close(fd);
  const int code = vyges_abc_execute(command);
  std::fflush(stdout);
  dup2(saved, STDOUT_FILENO);
  close(saved);
  return code;
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

int vyges_abc_cex_inputs(int frame, int* bits, int len) {
  try {
    auto* cex = static_cast<abc::Abc_Cex_t*>(abc::Abc_FrameReadCex(abc::Abc_FrameGetGlobalFrame()));
    if (cex == nullptr || frame < 0 || frame > cex->iFrame) {
      return -1;
    }
    // Bit layout: the initial register values (nRegs), then nPis inputs per frame.
    for (int i = 0; i < cex->nPis && i < len; i++) {
      bits[i] = abc::Abc_InfoHasBit(cex->pData, cex->nRegs + frame * cex->nPis + i) ? 1 : 0;
    }
    return cex->nPis;
  } catch (...) {
    return -1;
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

int vyges_abc_network_name(int kind, int index, char* buf, int len) {
  try {
    abc::Abc_Ntk_t* ntk = abc::Abc_FrameReadNtk(abc::Abc_FrameGetGlobalFrame());
    if (ntk == nullptr || index < 0 || len <= 0) {
      return -1;
    }
    abc::Abc_Obj_t* obj;
    if (kind == 0 && index < abc::Abc_NtkPiNum(ntk)) {
      obj = abc::Abc_NtkPi(ntk, index);
    } else if (kind == 1 && index < abc::Abc_NtkPoNum(ntk)) {
      obj = abc::Abc_NtkPo(ntk, index);
    } else {
      return -1;
    }
    const char* name = abc::Abc_ObjName(obj);
    const int n = static_cast<int>(std::strlen(name));
    const int k = n < len - 1 ? n : len - 1;
    std::memcpy(buf, name, k);
    buf[k] = '\0';
    return n;
  } catch (...) {
    return -1;
  }
}
