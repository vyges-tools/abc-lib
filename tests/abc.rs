// SPDX-License-Identifier: Apache-2.0
use vyges_abc_lib::{Abc, Error, ProbStatus};

fn data(name: &str) -> String {
    format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"))
}

/// Reading a network makes it the current one, with the counts the file declares.
#[test]
fn reading_a_blif_makes_it_the_current_network() {
    let mut abc = Abc::start().unwrap();
    assert_eq!(abc.network_counts(), None);
    abc.execute(&format!("read_blif {}", data("counter.blif"))).unwrap();
    let n = abc.network_counts().unwrap();
    assert_eq!((n.pis, n.pos, n.latches), (1, 1, 2));
}

/// An unknown command is the interpreter's non-zero return, not a crash.
#[test]
fn an_unknown_command_is_an_error() {
    let mut abc = Abc::start().unwrap();
    match abc.execute("no_such_command") {
        Err(Error::Command { code, .. }) => assert_ne!(code, 0),
        other => panic!("{other:?}"),
    }
}

/// A captured command returns what it printed, and only that: each call starts a fresh capture.
#[test]
fn execute_captured_returns_the_commands_own_output() {
    let mut abc = Abc::start().unwrap();
    abc.execute(&format!("read_blif {}", data("counter.blif"))).unwrap();
    let stats = abc.execute_captured("print_stats").unwrap();
    assert!(stats.contains("lat =    2"), "{stats:?}");
    abc.execute("strash").unwrap();
    let verdict = abc.execute_captured("bmc3 -F 10").unwrap();
    assert!(verdict.contains("asserted in frame 3"), "{verdict:?}");
    assert!(!verdict.contains("lat ="), "the first capture leaked into the second: {verdict:?}");
    match abc.execute_captured("no_such_command") {
        Err(Error::Command { code, .. }) => assert_ne!(code, 0),
        other => panic!("{other:?}"),
    }
}

/// Two XORs written differently are equivalent: the miter is unsatisfiable.
#[test]
fn equivalent_netlists_prove_unsat() {
    let mut abc = Abc::start().unwrap();
    abc.execute(&format!("miter {} {}", data("xor_a.blif"), data("xor_b.blif"))).unwrap();
    abc.execute("strash; &get -n; &cec -m").unwrap();
    assert_eq!(abc.prob_status(), ProbStatus::Unsat);
}

/// XOR against OR differs at a = b = 1: satisfiable, with a counterexample on output 0.
#[test]
fn different_netlists_prove_sat_with_a_counterexample() {
    let mut abc = Abc::start().unwrap();
    abc.execute(&format!("miter {} {}", data("xor_a.blif"), data("or.blif"))).unwrap();
    abc.execute("strash; &get -n; &cec -m").unwrap();
    assert_eq!(abc.prob_status(), ProbStatus::Sat);
    let cex = abc.cex().expect("a counterexample");
    assert_eq!((cex.po, cex.frame), (0, 0));
}

/// PDR proves the safe counter never reaches 3.
#[test]
fn pdr_proves_a_property_that_holds() {
    let mut abc = Abc::start().unwrap();
    abc.execute(&format!("read_blif {}", data("counter_safe.blif"))).unwrap();
    abc.execute("strash; pdr").unwrap();
    assert_eq!(abc.prob_status(), ProbStatus::Unsat);
}

/// BMC finds the plain counter reaching 3 after three enabled cycles: frame 3.
#[test]
fn bmc3_finds_the_shortest_counterexample() {
    let mut abc = Abc::start().unwrap();
    abc.execute(&format!("read_blif {}", data("counter.blif"))).unwrap();
    abc.execute("strash; bmc3 -F 10").unwrap();
    assert_eq!(abc.prob_status(), ProbStatus::Sat);
    assert_eq!(abc.cex().map(|c| (c.po, c.frame)), Some((0, 3)));
    // Reaching 3 by frame 3 needs `en` in each of frames 0, 1 and 2 (bits are laid out after the
    // initial register values, one block of inputs per frame).
    for frame in 0..3 {
        assert_eq!(abc.cex_inputs(frame), Some(vec![true]), "frame {frame}");
    }
    assert_eq!(abc.cex_inputs(4), None);
}

/// The frame is reset between sessions: a new session has no network and no verdict.
#[test]
fn a_new_session_starts_empty() {
    {
        let mut abc = Abc::start().unwrap();
        abc.execute(&format!("read_blif {}", data("counter.blif"))).unwrap();
    }
    let abc = Abc::start().unwrap();
    assert_eq!(abc.network_counts(), None);
    assert_eq!(abc.prob_status(), ProbStatus::Undecided);
    assert!(abc.cex().is_none());
}

/// `miter -m` keeps one output per output pair, in the first file's output order; `iprove` stops at
/// the first output it disproves and leaves that output's number and the inputs that show it.
/// two_a and two_b differ only in y1 (output 1), at a = b = 1.
#[test]
fn iprove_names_the_failing_output_of_a_multi_output_miter() {
    let mut abc = Abc::start().unwrap();
    abc.execute(&format!("miter -m {} {}", data("two_a.blif"), data("two_b.blif"))).unwrap();
    abc.execute("strash; iprove").unwrap();
    assert_eq!(abc.prob_status(), ProbStatus::Sat);
    assert_eq!(abc.cex().map(|c| (c.po, c.frame)), Some((1, 0)));
    assert_eq!((abc.pi_name(0).as_deref(), abc.pi_name(1).as_deref()), (Some("a"), Some("b")));
    assert_eq!(abc.pi_name(2), None);
    assert_eq!(abc.cex_inputs(0), Some(vec![true, true]));
    assert_eq!(abc.cex_inputs(1), None, "a combinational counterexample has one frame");
}

/// ⛔ `&cec -m` ASSUMES A SINGLE-OUTPUT MITER ("Assuming the current network is a single-output
/// miter"). Given the multi-output miter above it checks output 0 only, which agrees, and reports
/// the two designs EQUIVALENT although y1 differs. Pinned so a caller never builds on it: use
/// `iprove` on a multi-output miter, or `&cec -m` only after a plain (single-output) `miter`.
#[test]
fn cec_m_on_a_multi_output_miter_checks_output_0_only() {
    let mut abc = Abc::start().unwrap();
    abc.execute(&format!("miter -m {} {}", data("two_a.blif"), data("two_b.blif"))).unwrap();
    abc.execute("strash; &get -n; &cec -m").unwrap();
    assert_eq!(abc.prob_status(), ProbStatus::Unsat);
}

/// A SEQUENTIAL multi-output miter: `miter -m` on two designs with registers keeps them, and
/// `dprove` proves the pair equal in every reachable state. seq_dup duplicates seq_one's register
/// and ANDs the copies, which differs with the registers cut (q1 = 1, q2 = 0) but never in a state
/// the two can reach from 0.
#[test]
fn dprove_proves_a_sequential_miter_that_a_register_cut_would_not() {
    let mut abc = Abc::start().unwrap();
    abc.execute(&format!("miter -m {} {}", data("seq_one.blif"), data("seq_dup.blif"))).unwrap();
    abc.execute("strash; dprove").unwrap();
    assert_eq!(abc.prob_status(), ProbStatus::Unsat);
}

/// `dprove` on a sequential multi-output miter names the output and the FRAME where the designs
/// first differ, and the inputs of every frame up to it: seq_late's y1 leaves seq_one's after three
/// enabled cycles (output 1, frame 3).
#[test]
fn dprove_names_the_output_and_frame_of_a_sequential_difference() {
    let mut abc = Abc::start().unwrap();
    abc.execute(&format!("miter -m {} {}", data("seq_one.blif"), data("seq_late.blif"))).unwrap();
    abc.execute("strash; dprove").unwrap();
    assert_eq!(abc.prob_status(), ProbStatus::Sat);
    assert_eq!(abc.cex().map(|c| (c.po, c.frame)), Some((1, 3)));
    for frame in 0..3 {
        assert_eq!(abc.cex_inputs(frame), Some(vec![true]), "frame {frame}");
    }
}

/// ⛔ `dsec` gives the verdict but leaves NO counterexample: a caller that must show where two
/// designs differ builds the miter itself (`miter -m` + `dprove`, above).
#[test]
fn dsec_decides_but_leaves_no_counterexample() {
    {
        let mut abc = Abc::start().unwrap();
        abc.execute(&format!("dsec {} {}", data("seq_one.blif"), data("seq_dup.blif"))).unwrap();
        assert_eq!(abc.prob_status(), ProbStatus::Unsat);
    }
    let mut abc = Abc::start().unwrap();
    abc.execute(&format!("dsec {} {}", data("seq_one.blif"), data("seq_late.blif"))).unwrap();
    assert_eq!(abc.prob_status(), ProbStatus::Sat);
    assert!(abc.cex().is_none());
}

/// Property checking on several outputs at once: `pdr` names the first output it disproves and its
/// frame, and does not blame an output that holds. counter_two's `never` (output 0) cannot rise;
/// `bad` (output 1) rises after two enabled cycles.
#[test]
fn pdr_names_the_failing_property_among_several() {
    let mut abc = Abc::start().unwrap();
    abc.execute(&format!("read_blif {}", data("counter_two.blif"))).unwrap();
    abc.execute("strash; pdr").unwrap();
    assert_eq!(abc.prob_status(), ProbStatus::Sat);
    assert_eq!(abc.cex().map(|c| (c.po, c.frame)), Some((1, 2)));
    for frame in 0..2 {
        assert_eq!(abc.cex_inputs(frame), Some(vec![true]), "frame {frame}");
    }
}

/// ⛔ `bmc3 -F N` checks frames 0 to N-1: the failure in frame 2 is out of reach of `-F 2` (no
/// verdict) and found by `-F 3`. A bound of N cycles is `-F N+1`.
#[test]
fn bmc3_frame_bound_excludes_frame_f() {
    {
        let mut abc = Abc::start().unwrap();
        abc.execute(&format!("read_blif {}", data("counter_two.blif"))).unwrap();
        abc.execute("strash; bmc3 -F 2").unwrap();
        assert_eq!(abc.prob_status(), ProbStatus::Undecided);
    }
    let mut abc = Abc::start().unwrap();
    abc.execute(&format!("read_blif {}", data("counter_two.blif"))).unwrap();
    abc.execute("strash; bmc3 -F 3").unwrap();
    assert_eq!(abc.prob_status(), ProbStatus::Sat);
    assert_eq!(abc.cex().map(|c| (c.po, c.frame)), Some((1, 2)));
}
