// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Vyges

#include "shim/vyges_abc.h"

// ⚠️ Order matters: cmd.h uses Abc_Frame_t without declaring it, so main.h comes first.
#include "base/abc/abc.h"
#include "base/main/main.h"
#include "base/cmd/cmd.h"
#include "misc/util/utilCex.h"
// ⚠️ Order matters: mio.h and sclCon.h use declarations they do not include.
// clang-format off
#include "misc/st/st.h"
#include "map/mio/mio.h"
#include "map/mio/mioInt.h"
#include "misc/util/utilNam.h"
#include "map/scl/sclCon.h"
// clang-format on
#include "map/scl/sclLib.h"
#include "map/scl/sclSize.h"
#include "aig/aig/aig.h"
#include "aig/gia/gia.h"
#include "aig/gia/giaAig.h"
#include "map/if/if.h"
#include "misc/extra/extra.h"
#include "proof/dch/dch.h"

#include <fcntl.h>
#include <unistd.h>

#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace abc {
extern Abc_Ntk_t* Abc_NtkFromAigPhase(Aig_Man_t* pMan);
extern Abc_Ntk_t* Abc_NtkFromCellMappedGia(Gia_Man_t* p, int fUseBuffs);
extern Abc_Ntk_t* Abc_NtkFromMappedGia(Gia_Man_t* p, int fFindEnables, int fUseBuffs);
extern Aig_Man_t* Abc_NtkToDar(Abc_Ntk_t* pNtk, int fExors, int fRegisters);
extern Gia_Man_t* Gia_ManAigSynch2(Gia_Man_t* p, void* pPars, int nLutSize, int nRelaxRatio);
extern Vec_Ptr_t* Abc_NtkCollectCiNames(Abc_Ntk_t* pNtk);
extern Vec_Ptr_t* Abc_NtkCollectCoNames(Abc_Ntk_t* pNtk);
extern void Abc_NtkRedirectCiCo(Abc_Ntk_t* pNtk);
// Defined in ABC without a header declaring them.
Abc_Ntk_t* Abc_NtkMap(Abc_Ntk_t* pNtk, Mio_Library_t* userLib, double DelayTarget,
                      double AreaMulti, double DelayMulti, float LogFan, float Slew, float Gain,
                      int nGatesMin, int fRecovery, int fSwitching, int fSkipFanout,
                      int fUseProfile, int fUseBuffs, int fVerbose);
void Abc_FrameSetLibGen(void* pLib);
void Abc_FrameSetDrivingCell(char* pName);
}  // namespace abc

namespace {

// Copies `name` NUL-terminated into `buf` (truncated to len - 1 bytes); its full length, or -1.
int copy_name(const char* name, char* buf, int len) {
  if (name == nullptr || buf == nullptr || len <= 0) {
    return -1;
  }
  const int n = static_cast<int>(std::strlen(name));
  const int k = n < len - 1 ? n : len - 1;
  std::memcpy(buf, name, k);
  buf[k] = '\0';
  return n;
}

abc::SC_Cell* last_cell(void* lib) {
  auto* p = static_cast<abc::SC_Lib*>(lib);
  if (p == nullptr || abc::SC_LibCellNum(p) == 0) {
    return nullptr;
  }
  return abc::SC_LibCell(p, abc::SC_LibCellNum(p) - 1);
}

abc::SC_Pin* last_output(void* lib) {
  abc::SC_Cell* cell = last_cell(lib);
  if (cell == nullptr || cell->n_outputs == 0) {
    return nullptr;
  }
  return abc::SC_CellPin(cell, abc::Vec_PtrSize(&cell->vPins) - 1);
}

abc::Mio_Gate_t* gate_at(void* mio, int gate) {
  if (mio == nullptr || gate < 0) {
    return nullptr;
  }
  abc::Mio_Gate_t* g = abc::Mio_LibraryReadGates(static_cast<abc::Mio_Library_t*>(mio));
  for (int i = 0; g != nullptr && i < gate; i++) {
    g = abc::Mio_GateReadNext(g);
  }
  return g;
}

int gate_index(void* mio, abc::Mio_Gate_t* gate) {
  if (mio == nullptr || gate == nullptr) {
    return -1;
  }
  int i = 0;
  for (abc::Mio_Gate_t* g = abc::Mio_LibraryReadGates(static_cast<abc::Mio_Library_t*>(mio));
       g != nullptr; g = abc::Mio_GateReadNext(g), i++) {
    if (g == gate) {
      return i;
    }
  }
  return -1;
}

abc::Abc_Obj_t* obj_at(void* ntk, int obj) {
  auto* n = static_cast<abc::Abc_Ntk_t*>(ntk);
  if (n == nullptr || obj < 0 || obj >= abc::Abc_NtkObjNumMax(n)) {
    return nullptr;
  }
  return abc::Abc_NtkObj(n, obj);
}

}  // namespace

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

void* vyges_abc_scl_lib_alloc(const char* name, const char* file_name, float unit_cap_fst,
                              int unit_cap_snd, int unit_time, float default_max_out_slew) {
  try {
    abc::SC_Lib* lib = abc::Abc_SclLibAlloc();
    lib->pName = strdup(name);
    lib->pFileName = strdup(file_name);
    lib->default_wire_load = nullptr;
    lib->default_wire_load_sel = nullptr;
    lib->unit_cap_fst = unit_cap_fst;
    lib->unit_cap_snd = unit_cap_snd;
    lib->unit_time = unit_time;
    lib->default_max_out_slew = default_max_out_slew;
    return lib;
  } catch (...) {
    return nullptr;
  }
}

int vyges_abc_scl_cell_add(void* lib, const char* name, float area, int has_leakage, float leakage) {
  try {
    auto* p = static_cast<abc::SC_Lib*>(lib);
    if (p == nullptr) {
      return -1;
    }
    abc::SC_Cell* cell = abc::Abc_SclCellAlloc();
    cell->Id = abc::SC_LibCellNum(p);
    abc::Vec_PtrPush(&p->vCells, cell);
    cell->pName = strdup(name);
    cell->area = area;
    cell->drive_strength = 0;
    if (has_leakage) {
      cell->leakage = leakage;
    }
    return cell->Id;
  } catch (...) {
    return -1;
  }
}

int vyges_abc_scl_input_pin_add(void* lib, const char* name, float rise_cap, float fall_cap) {
  try {
    abc::SC_Cell* cell = last_cell(lib);
    if (cell == nullptr || cell->n_outputs != 0) {
      return -1;
    }
    abc::SC_Pin* pin = abc::Abc_SclPinAlloc();
    pin->dir = abc::sc_dir_Input;
    pin->pName = strdup(name);
    pin->rise_cap = rise_cap;
    pin->fall_cap = fall_cap;
    abc::Vec_PtrPush(&cell->vPins, pin);
    cell->n_inputs++;
    return 0;
  } catch (...) {
    return -1;
  }
}

int vyges_abc_scl_output_pin_add(void* lib, const char* name, int has_max_cap, float max_out_cap,
                                 int has_max_slew, float max_out_slew, const char* func_text) {
  try {
    abc::SC_Cell* cell = last_cell(lib);
    if (cell == nullptr || func_text == nullptr) {
      return -1;
    }
    abc::SC_Pin* pin = abc::Abc_SclPinAlloc();
    pin->dir = abc::sc_dir_Output;
    pin->pName = strdup(name);
    if (has_max_cap) {
      pin->max_out_cap = max_out_cap;
    }
    if (has_max_slew) {
      pin->max_out_slew = max_out_slew;
    }
    pin->func_text = strdup(func_text);
    std::vector<char*> inputs;
    for (int i = 0; i < cell->n_inputs; i++) {
      inputs.push_back(abc::SC_CellPin(cell, i)->pName);
    }
    abc::Vec_WrdErase(&pin->vFunc);
    abc::Vec_Wrd_t* func = abc::Mio_ParseFormulaTruth(pin->func_text, inputs.data(), cell->n_inputs);
    if (func == nullptr) {
      abc::Abc_SclPinFree(pin);
      return -2;
    }
    pin->vFunc = *func;
    ABC_FREE(func);
    abc::Vec_PtrPush(&cell->vPins, pin);
    cell->n_outputs++;
    return 0;
  } catch (...) {
    return -1;
  }
}

int vyges_abc_scl_timing_add(void* lib, const char* related_pin, int sense) {
  try {
    abc::SC_Pin* pin = last_output(lib);
    if (pin == nullptr || sense < 0 || sense > 2) {
      return -1;
    }
    abc::SC_Timings* timings = abc::Abc_SclTimingsAlloc();
    abc::Vec_PtrPush(&pin->vRTimings, timings);
    timings->pName = strdup(related_pin);
    abc::SC_Timing* timing = abc::Abc_SclTimingAlloc();
    abc::Vec_PtrPush(&timings->vTimings, timing);
    const abc::SC_TSense senses[] = {abc::sc_ts_Pos, abc::sc_ts_Neg, abc::sc_ts_Non};
    timing->tsense = senses[sense];
    return 0;
  } catch (...) {
    return -1;
  }
}

int vyges_abc_scl_surface(void* lib, int which, int n0, const float* index0, int n1,
                          const float* index1, const float* values) {
  try {
    abc::SC_Pin* pin = last_output(lib);
    if (pin == nullptr || abc::Vec_PtrSize(&pin->vRTimings) == 0 || which < 0 || which > 3 || n0 < 0 || n1 < 0) {
      return -1;
    }
    auto* timings = static_cast<abc::SC_Timings*>(abc::Vec_PtrEntryLast(&pin->vRTimings));
    auto* timing = static_cast<abc::SC_Timing*>(abc::Vec_PtrEntryLast(&timings->vTimings));
    abc::SC_Surface* surfaces[] = {&timing->pCellRise, &timing->pCellFall, &timing->pRiseTrans, &timing->pFallTrans};
    abc::SC_Surface* t = surfaces[which];
    for (int i = 0; i < n0; i++) {
      abc::Vec_FltPush(&t->vIndex0, index0[i]);
      abc::Vec_IntPush(&t->vIndex0I, abc::Scl_Flt2Int(index0[i]));
    }
    for (int j = 0; j < n1; j++) {
      abc::Vec_FltPush(&t->vIndex1, index1[j]);
      abc::Vec_IntPush(&t->vIndex1I, abc::Scl_Flt2Int(index1[j]));
    }
    for (int i = 0; i < n0; i++) {
      abc::Vec_Flt_t* row = abc::Vec_FltAlloc(n0);
      abc::Vec_Int_t* row_int = abc::Vec_IntAlloc(n0);
      abc::Vec_PtrPush(&t->vData, row);
      abc::Vec_PtrPush(&t->vDataI, row_int);
      for (int j = 0; j < n1; j++) {
        abc::Vec_FltPush(row, values[i * n1 + j]);
        abc::Vec_IntPush(row_int, abc::Scl_Flt2Int(values[i * n1 + j]));
      }
    }
    return 0;
  } catch (...) {
    return -1;
  }
}

int vyges_abc_scl_lib_finish(void* lib) {
  try {
    auto* p = static_cast<abc::SC_Lib*>(lib);
    if (p == nullptr) {
      return -1;
    }
    abc::Abc_SclLibNormalize(p);
    abc::Abc_SclHashCells(p);
    abc::Abc_SclLinkCells(p);
    return 0;
  } catch (...) {
    return -1;
  }
}

void vyges_abc_scl_lib_free(void* lib) {
  try {
    if (lib != nullptr) {
      abc::Abc_SclLibFree(static_cast<abc::SC_Lib*>(lib));
    }
  } catch (...) {
  }
}

float vyges_abc_scl_average_slew(void* lib) {
  try {
    return abc::Abc_SclComputeAverageSlew(static_cast<abc::SC_Lib*>(lib));
  } catch (...) {
    return -1;
  }
}

int vyges_abc_scl_cell_count(void* lib) {
  return lib == nullptr ? -1 : abc::SC_LibCellNum(static_cast<abc::SC_Lib*>(lib));
}

int vyges_abc_scl_cell_info(void* lib, int cell, int* n_inputs, int* n_outputs, float* area) {
  auto* p = static_cast<abc::SC_Lib*>(lib);
  if (p == nullptr || cell < 0 || cell >= abc::SC_LibCellNum(p)) {
    return -1;
  }
  abc::SC_Cell* c = abc::SC_LibCell(p, cell);
  *n_inputs = c->n_inputs;
  *n_outputs = c->n_outputs;
  *area = c->area;
  return 0;
}

int vyges_abc_scl_name(void* lib, int cell, int pin, char* buf, int len) {
  auto* p = static_cast<abc::SC_Lib*>(lib);
  if (p == nullptr || cell < 0 || cell >= abc::SC_LibCellNum(p)) {
    return -1;
  }
  abc::SC_Cell* c = abc::SC_LibCell(p, cell);
  if (pin < 0) {
    return copy_name(c->pName, buf, len);
  }
  if (pin >= abc::Vec_PtrSize(&c->vPins)) {
    return -1;
  }
  return copy_name(abc::SC_CellPin(c, pin)->pName, buf, len);
}

int vyges_abc_scl_pin_truth(void* lib, int cell, int pin, unsigned long long* word) {
  auto* p = static_cast<abc::SC_Lib*>(lib);
  if (p == nullptr || cell < 0 || cell >= abc::SC_LibCellNum(p)) {
    return 0;
  }
  abc::SC_Cell* c = abc::SC_LibCell(p, cell);
  if (pin < 0 || pin >= abc::Vec_PtrSize(&c->vPins)) {
    return 0;
  }
  abc::SC_Pin* q = abc::SC_CellPin(c, pin);
  if (abc::Vec_WrdSize(&q->vFunc) == 0) {
    return 0;
  }
  *word = abc::Vec_WrdEntry(&q->vFunc, 0);
  return 1;
}

void* vyges_abc_scl_derive_genlib(void* lib) {
  try {
    return abc::Abc_SclDeriveGenlibSimple(lib);
  } catch (...) {
    return nullptr;
  }
}

void vyges_abc_mio_free(void* mio) {
  try {
    if (mio != nullptr) {
      abc::Mio_LibraryDelete(static_cast<abc::Mio_Library_t*>(mio));
    }
  } catch (...) {
  }
}

int vyges_abc_mio_gate_count(void* mio) {
  if (mio == nullptr) {
    return -1;
  }
  int n = 0;
  for (abc::Mio_Gate_t* g = abc::Mio_LibraryReadGates(static_cast<abc::Mio_Library_t*>(mio));
       g != nullptr; g = abc::Mio_GateReadNext(g)) {
    n++;
  }
  return n;
}

int vyges_abc_mio_gate_pin_count(void* mio, int gate) {
  abc::Mio_Gate_t* g = gate_at(mio, gate);
  if (g == nullptr) {
    return -1;
  }
  int n = 0;
  for (abc::Mio_Pin_t* p = abc::Mio_GateReadPins(g); p != nullptr; p = abc::Mio_PinReadNext(p)) {
    n++;
  }
  return n;
}

int vyges_abc_mio_name(void* mio, int gate, int pin, char* buf, int len) {
  abc::Mio_Gate_t* g = gate_at(mio, gate);
  if (g == nullptr) {
    return -1;
  }
  if (pin == -1) {
    return copy_name(abc::Mio_GateReadName(g), buf, len);
  }
  if (pin == -2) {
    return copy_name(abc::Mio_GateReadOutName(g), buf, len);
  }
  abc::Mio_Pin_t* p = abc::Mio_GateReadPins(g);
  for (int i = 0; p != nullptr && i < pin; i++) {
    p = abc::Mio_PinReadNext(p);
  }
  return p == nullptr || pin < 0 ? -1 : copy_name(abc::Mio_PinReadName(p), buf, len);
}

int vyges_abc_mio_special_gate(void* mio, int which) {
  auto* lib = static_cast<abc::Mio_Library_t*>(mio);
  if (lib == nullptr) {
    return -1;
  }
  abc::Mio_Gate_t* g = which == 0   ? abc::Mio_LibraryReadConst0(lib)
                       : which == 1 ? abc::Mio_LibraryReadConst1(lib)
                       : which == 2 ? abc::Mio_LibraryReadBuf(lib)
                                    : nullptr;
  return gate_index(mio, g);
}

int vyges_abc_mio_rename(void* mio, int gate, const char* name, const char* out_name) {
  abc::Mio_Gate_t* g = gate_at(mio, gate);
  if (g == nullptr || name == nullptr || out_name == nullptr) {
    return -1;
  }
  // The gate's fields, as a caller renaming a gate in place sets them.
  free(g->pName);
  free(g->pOutName);
  g->pName = strdup(name);
  g->pOutName = strdup(out_name);
  return 0;
}

void* vyges_abc_ntk_alloc_mapped_netlist(void* mio) {
  try {
    abc::Abc_Ntk_t* ntk = abc::Abc_NtkAlloc(abc::ABC_NTK_NETLIST, abc::ABC_FUNC_MAP, 1);
    ntk->pManFunc = mio;
    return ntk;
  } catch (...) {
    return nullptr;
  }
}

void vyges_abc_ntk_delete(void* ntk) {
  try {
    if (ntk != nullptr) {
      abc::Abc_NtkDelete(static_cast<abc::Abc_Ntk_t*>(ntk));
    }
  } catch (...) {
  }
}

int vyges_abc_ntk_create(void* ntk, int kind, int gate) {
  try {
    auto* n = static_cast<abc::Abc_Ntk_t*>(ntk);
    if (n == nullptr) {
      return -1;
    }
    abc::Abc_Obj_t* obj = nullptr;
    switch (kind) {
      case 0: obj = abc::Abc_NtkCreatePi(n); break;
      case 1: obj = abc::Abc_NtkCreatePo(n); break;
      case 2: obj = abc::Abc_NtkCreateNet(n); break;
      case 3: {
        abc::Mio_Gate_t* g = gate_at(n->pManFunc, gate);
        if (g == nullptr) {
          return -1;
        }
        obj = abc::Abc_NtkCreateNode(n);
        abc::Abc_ObjSetData(obj, g);
        break;
      }
      default: return -1;
    }
    return abc::Abc_ObjId(obj);
  } catch (...) {
    return -1;
  }
}

int vyges_abc_obj_assign_name(void* ntk, int obj, const char* name) {
  try {
    abc::Abc_Obj_t* o = obj_at(ntk, obj);
    if (o == nullptr || name == nullptr) {
      return -1;
    }
    std::string copy(name);
    abc::Abc_ObjAssignName(o, copy.data(), nullptr);
    return 0;
  } catch (...) {
    return -1;
  }
}

int vyges_abc_obj_add_fanin(void* ntk, int obj, int fanin) {
  try {
    abc::Abc_Obj_t* o = obj_at(ntk, obj);
    abc::Abc_Obj_t* f = obj_at(ntk, fanin);
    if (o == nullptr || f == nullptr) {
      return -1;
    }
    abc::Abc_ObjAddFanin(o, f);
    return 0;
  } catch (...) {
    return -1;
  }
}

void* vyges_abc_ntk_to_logic(void* ntk) {
  try {
    return abc::Abc_NtkToLogic(static_cast<abc::Abc_Ntk_t*>(ntk));
  } catch (...) {
    return nullptr;
  }
}

void* vyges_abc_ntk_strash(void* ntk, int all_nodes, int cleanup, int record) {
  try {
    return abc::Abc_NtkStrash(static_cast<abc::Abc_Ntk_t*>(ntk), all_nodes, cleanup, record);
  } catch (...) {
    return nullptr;
  }
}

void* vyges_abc_ntk_balance(void* ntk, int duplicate, int selective, int update_level) {
  try {
    return abc::Abc_NtkBalance(static_cast<abc::Abc_Ntk_t*>(ntk), duplicate, selective, update_level);
  } catch (...) {
    return nullptr;
  }
}

void* vyges_abc_ntk_to_netlist(void* ntk) {
  try {
    return abc::Abc_NtkToNetlist(static_cast<abc::Abc_Ntk_t*>(ntk));
  } catch (...) {
    return nullptr;
  }
}

int vyges_abc_frame_set_lib_and_driver(void* ntk) {
  try {
    auto* n = static_cast<abc::Abc_Ntk_t*>(ntk);
    if (n == nullptr) {
      return -1;
    }
    auto* lib = static_cast<abc::Mio_Library_t*>(n->pManFunc);
    abc::Abc_FrameSetLibGen(lib);
    abc::Mio_Gate_t* buffer = abc::Mio_LibraryReadBuf(lib);
    if (buffer == nullptr) {
      return -1;
    }
    abc::Abc_FrameSetDrivingCell(strdup(abc::Mio_GateReadName(buffer)));
    return 0;
  } catch (...) {
    return -1;
  }
}

void* vyges_abc_ntk_map(void* ntk, double delay_target, double area_multi, double delay_multi,
                        float log_fan, float slew, float gain, int n_gates_min, int recovery,
                        int switching, int skip_fanout, int use_profile, int use_buffs, int verbose) {
  try {
    return abc::Abc_NtkMap(static_cast<abc::Abc_Ntk_t*>(ntk), nullptr, delay_target, area_multi,
                           delay_multi, log_fan, slew, gain, n_gates_min, recovery, switching,
                           skip_fanout, use_profile, use_buffs, verbose);
  } catch (...) {
    return nullptr;
  }
}

int vyges_abc_ntk_cleanup(void* ntk, int verbose) {
  try {
    return abc::Abc_NtkCleanup(static_cast<abc::Abc_Ntk_t*>(ntk), verbose);
  } catch (...) {
    return -1;
  }
}

void* vyges_abc_scl_buffering(void* ntk, void* lib, int gain_ratio, int degree, int size_only,
                              int add_bufs, int buf_pis, int use_wire_loads, int verbose,
                              int very_verbose) {
  try {
    abc::SC_BusPars pars;
    std::memset(&pars, 0, sizeof(pars));
    pars.GainRatio = gain_ratio;
    pars.Slew = abc::Abc_SclComputeAverageSlew(static_cast<abc::SC_Lib*>(lib));
    pars.nDegree = degree;
    pars.fSizeOnly = size_only;
    pars.fAddBufs = add_bufs;
    pars.fBufPis = buf_pis;
    pars.fUseWireLoads = use_wire_loads;
    pars.fVerbose = verbose;
    pars.fVeryVerbose = very_verbose;
    return abc::Abc_SclBufferingPerform(static_cast<abc::Abc_Ntk_t*>(ntk), static_cast<abc::SC_Lib*>(lib), &pars);
  } catch (...) {
    return nullptr;
  }
}

int vyges_abc_ntk_is_mapped_netlist(void* ntk) {
  auto* n = static_cast<abc::Abc_Ntk_t*>(ntk);
  return n != nullptr && abc::Abc_NtkIsNetlist(n) && abc::Abc_NtkHasMapping(n) ? 1 : 0;
}

void* vyges_abc_ntk_mio(void* ntk) {
  auto* n = static_cast<abc::Abc_Ntk_t*>(ntk);
  return n == nullptr ? nullptr : n->pManFunc;
}

int vyges_abc_ntk_obj_num_max(void* ntk) {
  auto* n = static_cast<abc::Abc_Ntk_t*>(ntk);
  return n == nullptr ? -1 : abc::Abc_NtkObjNumMax(n);
}

int vyges_abc_obj_type(void* ntk, int obj) {
  abc::Abc_Obj_t* o = obj_at(ntk, obj);
  return o == nullptr ? -1 : static_cast<int>(abc::Abc_ObjType(o));
}

int vyges_abc_obj_name(void* ntk, int obj, char* buf, int len) {
  try {
    abc::Abc_Obj_t* o = obj_at(ntk, obj);
    return o == nullptr ? -1 : copy_name(abc::Abc_ObjName(o), buf, len);
  } catch (...) {
    return -1;
  }
}

int vyges_abc_obj_fanin_count(void* ntk, int obj) {
  abc::Abc_Obj_t* o = obj_at(ntk, obj);
  return o == nullptr ? -1 : abc::Abc_ObjFaninNum(o);
}

int vyges_abc_obj_fanin(void* ntk, int obj, int k) {
  abc::Abc_Obj_t* o = obj_at(ntk, obj);
  if (o == nullptr || k < 0 || k >= abc::Abc_ObjFaninNum(o)) {
    return -1;
  }
  return abc::Abc_ObjFaninId(o, k);
}

int vyges_abc_obj_fanout_count(void* ntk, int obj) {
  abc::Abc_Obj_t* o = obj_at(ntk, obj);
  return o == nullptr ? -1 : abc::Abc_ObjFanoutNum(o);
}

int vyges_abc_obj_fanout(void* ntk, int obj, int k) {
  abc::Abc_Obj_t* o = obj_at(ntk, obj);
  if (o == nullptr || k < 0 || k >= abc::Abc_ObjFanoutNum(o)) {
    return -1;
  }
  return abc::Abc_ObjId(abc::Abc_ObjFanout(o, k));
}

int vyges_abc_obj_gate(void* ntk, int obj) {
  abc::Abc_Obj_t* o = obj_at(ntk, obj);
  if (o == nullptr || !abc::Abc_ObjIsNode(o)) {
    return -1;
  }
  return gate_index(static_cast<abc::Abc_Ntk_t*>(ntk)->pManFunc, static_cast<abc::Mio_Gate_t*>(abc::Abc_ObjData(o)));
}

// ---- GIA ----------------------------------------------------------------------------------------

static abc::Gia_Man_t* gia_of(void* p) {
  return static_cast<abc::Gia_Man_t*>(p);
}

void* vyges_abc_gia_from_network(void* ntk) {
  try {
    auto* n = static_cast<abc::Abc_Ntk_t*>(ntk);
    abc::Abc_Ntk_t* strash = abc::Abc_NtkStrash(n, 0, 1, 0);
    if (strash == nullptr) {
      return nullptr;
    }
    abc::Aig_Man_t* aig = abc::Abc_NtkToDar(strash, 0, 0);
    abc::Gia_Man_t* gia = aig ? abc::Gia_ManFromAig(aig) : nullptr;
    if (gia != nullptr) {
      char* inits = abc::Abc_NtkCollectLatchValuesStr(n);
      abc::Gia_Man_t* zeroed = abc::Gia_ManDupZeroUndc(gia, inits, 0, 0, 0);
      abc::Gia_ManStop(gia);
      ABC_FREE(inits);
      gia = zeroed;
      gia->vNamesIn = abc::Abc_NtkCollectCiNames(n);
      gia->vNamesOut = abc::Abc_NtkCollectCoNames(n);
    }
    if (aig != nullptr) {
      abc::Aig_ManStop(aig);
    }
    abc::Abc_NtkDelete(strash);
    return gia;
  } catch (...) {
    return nullptr;
  }
}

void vyges_abc_gia_free(void* gia) {
  if (gia != nullptr) {
    abc::Gia_ManStop(gia_of(gia));
  }
}

void* vyges_abc_gia_replace(void* gia, void* next) {
  auto* g = gia_of(gia);
  auto* n = gia_of(next);
  if (n == nullptr || g == n) {
    return g;
  }
  if (g->vNamesIn && !n->vNamesIn) {
    std::swap(g->vNamesIn, n->vNamesIn);
  }
  if (g->vNamesOut && !n->vNamesOut) {
    std::swap(g->vNamesOut, n->vNamesOut);
  }
  if (g->vNamesNode && !n->vNamesNode) {
    std::swap(g->vNamesNode, n->vNamesNode);
  }
  abc::Gia_ManStop(g);
  return n;
}

int vyges_abc_gia_query(void* gia, int which) {
  auto* g = gia_of(gia);
  switch (which) {
    case 0:
      return g->pReprs != nullptr;
    case 1:
      return abc::Gia_ManHasMapping(g);
    case 2:
      return abc::Gia_ManHasCellMapping(g);
    case 3:
      return g->pMuxes != nullptr;
    case 4:
      return abc::Gia_ManHasDangling(g);
    default:
      return -1;
  }
}

#define VYGES_GIA_CALL(expr) \
  try {                      \
    return (expr);           \
  } catch (...) {            \
    return nullptr;          \
  }

void* vyges_abc_gia_rehash(void* gia, int add_strash) {
  VYGES_GIA_CALL(abc::Gia_ManRehash(gia_of(gia), add_strash))
}

static void dch_defaults(abc::Dch_Pars_t* pars, int bt_limit) {
  std::memset(pars, 0, sizeof(*pars));
  abc::Dch_ManSetDefaultParams(pars);
  if (bt_limit >= 0) {
    pars->nBTLimit = bt_limit;
  }
}

void* vyges_abc_gia_dch(void* gia, int bt_limit) {
  abc::Dch_Pars_t pars;
  dch_defaults(&pars, bt_limit);
  VYGES_GIA_CALL(abc::Gia_ManPerformDch(gia_of(gia), &pars))
}

void* vyges_abc_gia_equiv_reduce(void* gia, int use_all, int dual_out, int skip_phase, int verbose) {
  VYGES_GIA_CALL(abc::Gia_ManEquivReduce(gia_of(gia), use_all, dual_out, skip_phase, verbose))
}

void* vyges_abc_gia_syn2(void* gia, int old_algo, int coarsen, int cut_min, int relax_ratio,
                         int delay_min, int verbose, int very_verbose) {
  VYGES_GIA_CALL(abc::Gia_ManAigSyn2(gia_of(gia), old_algo, coarsen, cut_min, relax_ratio, delay_min,
                                     verbose, very_verbose))
}

void* vyges_abc_gia_syn3(void* gia, int verbose, int very_verbose) {
  VYGES_GIA_CALL(abc::Gia_ManAigSyn3(gia_of(gia), verbose, very_verbose))
}

void* vyges_abc_gia_syn4(void* gia, int verbose, int very_verbose) {
  VYGES_GIA_CALL(abc::Gia_ManAigSyn4(gia_of(gia), verbose, very_verbose))
}

void* vyges_abc_gia_retime_forward(void* gia, int max_iters, int verbose) {
  VYGES_GIA_CALL(abc::Gia_ManRetimeForward(gia_of(gia), max_iters, verbose))
}

void* vyges_abc_gia_compress2(void* gia, int update_level, int verbose) {
  VYGES_GIA_CALL(abc::Gia_ManCompress2(gia_of(gia), update_level, verbose))
}

void* vyges_abc_gia_area_balance(void* gia, int simple_and, int new_nodes_max, int verbose,
                                 int very_verbose) {
  VYGES_GIA_CALL(abc::Gia_ManAreaBalance(gia_of(gia), simple_and, new_nodes_max, verbose, very_verbose))
}

void* vyges_abc_gia_balance(void* gia, int simple_and, int strict, int verbose) {
  VYGES_GIA_CALL(abc::Gia_ManBalance(gia_of(gia), simple_and, strict, verbose))
}

void* vyges_abc_gia_equiv_reduce_and_remap(void* gia, int seq, int miter_pairs) {
  VYGES_GIA_CALL(abc::Gia_ManEquivReduceAndRemap(gia_of(gia), seq, miter_pairs))
}

void* vyges_abc_gia_if_map(void* gia, int delay_opt, int lut_size, int truth, int cut_min, int exp_red) {
  abc::If_Par_t pars;
  std::memset(&pars, 0, sizeof(pars));
  abc::Gia_ManSetIfParsDefault(&pars);
  pars.fDelayOpt = delay_opt;
  pars.nLutSize = lut_size;
  pars.fTruth = truth;
  pars.fCutMin = cut_min;
  pars.fExpRed = exp_red;
  VYGES_GIA_CALL(abc::Gia_ManPerformMapping(gia_of(gia), &pars))
}

void* vyges_abc_gia_synch2(void* gia, int bt_limit, int lut_size, int relax_ratio) {
  abc::Dch_Pars_t pars;
  dch_defaults(&pars, bt_limit);
  VYGES_GIA_CALL(abc::Gia_ManAigSynch2(gia_of(gia), &pars, lut_size, relax_ratio))
}

void* vyges_abc_ntk_from_cell_mapped_gia(void* gia, int use_buffs) {
  VYGES_GIA_CALL(abc::Abc_NtkFromCellMappedGia(gia_of(gia), use_buffs))
}

void* vyges_abc_ntk_from_mapped_gia(void* gia, int find_enables, int use_buffs) {
  VYGES_GIA_CALL(abc::Abc_NtkFromMappedGia(gia_of(gia), find_enables, use_buffs))
}

void* vyges_abc_ntk_from_gia_aig(void* gia) {
  try {
    abc::Aig_Man_t* aig = abc::Gia_ManToAig(gia_of(gia), 0);
    if (aig == nullptr) {
      return nullptr;
    }
    abc::Abc_Ntk_t* ntk = abc::Abc_NtkFromAigPhase(aig);
    if (ntk != nullptr) {
      ntk->pName = abc::Extra_UtilStrsav(aig->pName);
    }
    abc::Aig_ManStop(aig);
    return ntk;
  } catch (...) {
    return nullptr;
  }
}

int vyges_abc_ntk_name_from_gia(void* ntk, void* gia) {
  try {
    auto* n = static_cast<abc::Abc_Ntk_t*>(ntk);
    auto* g = gia_of(gia);
    if (g->vNamesIn == nullptr || g->vNamesOut == nullptr) {
      return -1;
    }
    for (int i = 0; i < abc::Abc_NtkCiNum(n); i++) {
      if (i >= abc::Vec_PtrSize(g->vNamesIn)) {
        return -1;
      }
      abc::Abc_Obj_t* obj = abc::Abc_NtkCi(n, i);
      abc::Nm_ManDeleteIdName(n->pManName, obj->Id);
      abc::Abc_ObjAssignName(obj, static_cast<char*>(abc::Vec_PtrEntry(g->vNamesIn, i)), nullptr);
    }
    for (int i = 0; i < abc::Abc_NtkCoNum(n); i++) {
      if (i >= abc::Vec_PtrSize(g->vNamesOut)) {
        return -1;
      }
      abc::Abc_Obj_t* obj = abc::Abc_NtkCo(n, i);
      abc::Nm_ManDeleteIdName(n->pManName, obj->Id);
      abc::Abc_ObjAssignName(obj, static_cast<char*>(abc::Vec_PtrEntry(g->vNamesOut, i)), nullptr);
    }
    return 0;
  } catch (...) {
    return -1;
  }
}

int vyges_abc_ntk_redirect_ci_co(void* ntk) {
  try {
    abc::Abc_NtkRedirectCiCo(static_cast<abc::Abc_Ntk_t*>(ntk));
    return 0;
  } catch (...) {
    return -1;
  }
}

int vyges_abc_ntk_is_strash(void* ntk) {
  return abc::Abc_NtkIsStrash(static_cast<abc::Abc_Ntk_t*>(ntk));
}

void vyges_abc_getopt_reset(void) {
  abc::Extra_UtilGetoptReset();
}

void* vyges_abc_ntk_dup_dfs(void* ntk) {
  VYGES_GIA_CALL(abc::Abc_NtkDupDfs(static_cast<abc::Abc_Ntk_t*>(ntk)))
}

int vyges_abc_scl_resize(void* lib, void* ntk, int upsize, int iters, int iter_no_change, int window,
                         int ratio, int notches, int use_dept) {
  try {
    abc::SC_SizePars pars;
    std::memset(&pars, 0, sizeof(pars));
    pars.nIters = iters;
    pars.nIterNoChange = iter_no_change;
    pars.Window = window;
    pars.Ratio = ratio;
    pars.Notches = notches;
    pars.fUseDept = use_dept;
    auto* l = static_cast<abc::SC_Lib*>(lib);
    auto* n = static_cast<abc::Abc_Ntk_t*>(ntk);
    if (upsize) {
      abc::Abc_SclUpsizePerform(l, n, &pars, nullptr);
    } else {
      abc::Abc_SclDnsizePerform(l, n, &pars, nullptr);
    }
    return 0;
  } catch (...) {
    return -1;
  }
}

int vyges_abc_frame_set_lib_gen(void* mio) {
  try {
    abc::Abc_FrameSetLibGen(mio);
    return 0;
  } catch (...) {
    return -1;
  }
}

void vyges_abc_ntk_set_man_func(void* ntk, void* mio) {
  static_cast<abc::Abc_Ntk_t*>(ntk)->pManFunc = mio;
}

int vyges_abc_stdout_suppress(void) {
  std::fflush(stdout);
  const int saved = dup(STDOUT_FILENO);
  if (saved < 0) {
    return -1;
  }
  const int null_fd = open("/dev/null", O_WRONLY);
  if (null_fd < 0 || dup2(null_fd, STDOUT_FILENO) < 0) {
    if (null_fd >= 0) {
      close(null_fd);
    }
    close(saved);
    return -1;
  }
  close(null_fd);
  return saved;
}

void vyges_abc_stdout_restore(int saved) {
  std::fflush(stdout);
  if (saved >= 0) {
    dup2(saved, STDOUT_FILENO);
    close(saved);
  }
}
