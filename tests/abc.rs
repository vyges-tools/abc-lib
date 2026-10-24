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
