// SPDX-License-Identifier: Apache-2.0
//! The cell-library and network calls: a library built cell by cell, a mapped netlist built over
//! its gate library, the delay-optimization sequence over it, and the result read back.
use vyges_abc_lib::{Abc, BufferParams, CellLib, GateLib, MapParams, Network, ObjType, Sense, Special, Surface};

const SLEW: [f32; 2] = [5.0, 50.0];
const LOAD: [f32; 2] = [1.0, 10.0];

/// One output pin's timings from each input, every table the same small slope.
fn timed(lib: &mut CellLib, inputs: &[&str], sense: Sense, delay: f32) {
    for i in inputs {
        lib.add_timing(i, sense).unwrap();
        let d = [delay, delay + 10.0, delay + 5.0, delay + 15.0];
        for s in [Surface::CellRise, Surface::CellFall, Surface::RiseTransition, Surface::FallTransition] {
            lib.add_surface(s, &SLEW, &LOAD, &d).unwrap();
        }
    }
}

/// A buffer, an inverter, a NAND2, an AND2 and both tie cells (two TIELO, the smaller second).
fn library(abc: &Abc) -> CellLib {
    let mut lib = CellLib::new(abc, "tiny", "tiny.lib", 1.0, 15, 12, -1.0).unwrap();
    for (name, area, inputs, func, sense, delay) in [
        ("BUF", 2.0, &["A"][..], "A", Sense::PositiveUnate, 20.0),
        ("INV", 1.0, &["A"][..], "!A", Sense::NegativeUnate, 10.0),
        ("NAND2", 2.0, &["A", "B"][..], "!(A*B)", Sense::NegativeUnate, 15.0),
        ("AND2", 3.0, &["A", "B"][..], "(A*B)", Sense::PositiveUnate, 25.0),
    ] {
        lib.add_cell(name, area, Some(0.5)).unwrap();
        for i in inputs {
            lib.add_input_pin(i, 1.0, 1.0).unwrap();
        }
        lib.add_output_pin("Y", Some(50.0), None, func).unwrap();
        timed(&mut lib, inputs, sense, delay);
    }
    for (name, area, func) in [("TIELO_BIG", 2.0, "0"), ("TIELO", 1.0, "0"), ("TIEHI", 1.0, "1")] {
        lib.add_cell(name, area, None).unwrap();
        lib.add_output_pin("Z", None, None, func).unwrap();
    }
    lib.finish().unwrap();
    lib
}

fn gate(lib: &GateLib, name: &str) -> usize {
    (0..lib.gate_count()).find(|&g| lib.gate_name(g).as_deref() == Some(name)).unwrap_or_else(|| panic!("no gate {name}"))
}

/// A library keeps its cells in the order added, each with its pins (inputs first) and its
/// function's truth table; a tie cell's table is all zeros or all ones.
#[test]
fn a_cell_library_keeps_cells_pins_and_functions() {
    let abc = Abc::start().unwrap();
    let lib = library(&abc);
    let cells = lib.cells();
    let names: Vec<&str> = cells.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["BUF", "INV", "NAND2", "AND2", "TIELO_BIG", "TIELO", "TIEHI"]);
    assert_eq!((cells[2].inputs, cells[2].outputs, cells[2].area), (2, 1, 2.0));
    assert_eq!(lib.pin_name(2, 1).as_deref(), Some("B"));
    assert_eq!(lib.pin_name(2, 2).as_deref(), Some("Y"));
    // NAND2 over (A, B): 0 only where both are 1 (minterm 3), the 4-row table replicated.
    assert_eq!(lib.pin_truth(2, 2).map(|w| w & 0xF), Some(0x7));
    assert_eq!(lib.pin_truth(4, 0), Some(0));
    assert_eq!(lib.pin_truth(6, 0), Some(u64::MAX));
    assert!(lib.average_slew() > 0.0);
}

/// An output pin's function that does not parse is refused, not a crash.
#[test]
fn an_unparsable_function_is_an_error() {
    let abc = Abc::start().unwrap();
    let mut lib = CellLib::new(&abc, "bad", "bad.lib", 1.0, 15, 12, -1.0).unwrap();
    lib.add_cell("X", 1.0, None).unwrap();
    lib.add_input_pin("A", 1.0, 1.0).unwrap();
    assert!(lib.add_output_pin("Y", None, None, "A*(").is_err());
}

/// The derived gate library names each gate and its pins in order, and finds the buffer and the
/// constant gates; renaming a gate changes its name and output pin name.
#[test]
fn the_gate_library_has_the_cells_pins_and_special_gates() {
    let abc = Abc::start().unwrap();
    let lib = library(&abc);
    let mut mio = lib.derive_genlib().unwrap();
    let nand = gate(&mio, "NAND2");
    assert_eq!(mio.gate_pins(nand), ["A", "B"]);
    assert_eq!(mio.gate_out_name(nand).as_deref(), Some("Y"));
    assert_eq!(mio.special(Special::Buffer).and_then(|g| mio.gate_name(g)).as_deref(), Some("BUF"));
    let c0 = mio.special(Special::Const0).expect("a constant-0 gate");
    mio.rename(c0, "TIELO", "Z").unwrap();
    assert_eq!(mio.gate_name(c0).as_deref(), Some("TIELO"));
    assert_eq!(mio.gate_out_name(c0).as_deref(), Some("Z"));
    mio.free();
}

/// A mapped netlist of a NAND driving an inverter (an AND of a and b) goes through the delay
/// sequence — to logic, strash, balance, map, cleanup, buffering, to netlist — and comes back a
/// mapped netlist with the same two named inputs and output, every node a gate of the library.
#[test]
fn the_delay_sequence_maps_a_netlist_back_to_library_gates() {
    let mut abc = Abc::start().unwrap();
    let lib = library(&abc);
    let mio = lib.derive_genlib().unwrap();
    let mut ntk = Network::mapped_netlist(&abc, mio).unwrap();
    let pi = |ntk: &mut Network, name: &str| {
        let p = ntk.create_pi().unwrap();
        let n = ntk.create_net().unwrap();
        ntk.assign_name(n, name).unwrap();
        ntk.add_fanin(n, p).unwrap();
        n
    };
    let (a, b) = (pi(&mut ntk, "a"), pi(&mut ntk, "b"));
    let po = ntk.create_po().unwrap();
    let y = ntk.create_net().unwrap();
    ntk.assign_name(y, "y").unwrap();
    ntk.add_fanin(po, y).unwrap();
    let nand = ntk.create_node(gate(&mio, "NAND2")).unwrap();
    let inv = ntk.create_node(gate(&mio, "INV")).unwrap();
    ntk.assign_name(nand, "u1").unwrap();
    ntk.assign_name(inv, "u2").unwrap();
    let mid = ntk.create_net().unwrap();
    ntk.add_fanin(nand, a).unwrap();
    ntk.add_fanin(nand, b).unwrap();
    ntk.add_fanin(mid, nand).unwrap();
    ntk.add_fanin(inv, mid).unwrap();
    ntk.add_fanin(y, inv).unwrap();
    assert!(ntk.is_mapped_netlist());

    let logic = ntk.to_logic().unwrap();
    let aig = logic.strash(false, true, false).unwrap();
    let balanced = aig.balance(true, true, true).unwrap();
    ntk.install_library(&mut abc).unwrap();
    let params = MapParams {
        delay_target: 1.0,
        area_multi: 0.0,
        delay_multi: 2.5,
        log_fan: 0.0,
        slew: 0.0,
        gain: 250.0,
        gates_min: 0,
        recovery: true,
        switching: false,
        skip_fanout: false,
        use_profile: false,
        use_buffs: false,
        verbose: false,
    };
    let mut mapped = balanced.map(&params).unwrap();
    mapped.cleanup(false).unwrap();
    let buffers = BufferParams { gain_ratio: 300, degree: 8, size_only: false, add_bufs: true, buf_pis: true, use_wire_loads: false, verbose: false, very_verbose: false };
    let buffered = mapped.buffering(&lib, &buffers).unwrap();
    let out = buffered.to_netlist().unwrap();
    assert!(out.is_mapped_netlist());

    let mio = out.gate_lib();
    let mut pis = Vec::new();
    let mut pos = Vec::new();
    let mut gates = Vec::new();
    for o in 0..out.obj_num_max() {
        match out.obj_type(o) {
            Some(ObjType::Pi) => pis.push(out.obj_name(out.fanouts(o)[0]).unwrap()),
            Some(ObjType::Po) => pos.push(out.obj_name(out.fanins(o)[0]).unwrap()),
            Some(ObjType::Node) => gates.push(mio.gate_name(out.obj_gate(o).unwrap()).unwrap()),
            _ => {}
        }
    }
    assert_eq!((pis, pos), (vec!["a".to_string(), "b".to_string()], vec!["y".to_string()]));
    assert!(!gates.is_empty() && gates.iter().all(|g| ["BUF", "INV", "NAND2", "AND2"].contains(&g.as_str())), "{gates:?}");
}
