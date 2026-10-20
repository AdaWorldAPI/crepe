//! `crepe_plan!` reports the same binding analysis `crepe!` uses, as data.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub relations: Vec<PlanRelation>,
    pub strata: Vec<Vec<&'static str>>,
    pub rules: Vec<PlanRule>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRelation {
    pub name: &'static str,
    pub arity: usize,
    pub kind: PlanRelKind,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanRelKind {
    Input,
    Output,
    Intermediate,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRule {
    pub goal: PlanAtom,
    pub body: Vec<PlanClause>,
    pub stratum: usize,
    pub recursive: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanAtom {
    pub relation: &'static str,
    pub negated: bool,
    pub fields: Vec<PlanField>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanClause {
    Atom(PlanAtom),
    Unsupported(&'static str),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanField {
    Ignored,
    Ref(&'static str),
    NewVar(&'static str),
    BoundVar(&'static str),
    Const(u64),
    Expr(&'static str),
}

mod star {
    use super::*;
    use crepe::crepe_plan;
    crepe_plan! {
        @input
        struct Parent(u32, u32);
        @input
        struct Allowed(u32);
        @output
        struct Answer(u32);
        Answer(z) <- Parent(17, y), Parent(y, z), Allowed(y);
    }
}

mod mixed {
    use super::*;
    use crepe::crepe_plan;
    crepe_plan! {
        @input
        struct Edge(u32, u32);
        @input
        struct Blocked(u32);
        @output
        struct Reach(u32, u32);
        @output
        struct Free(u32);
        Reach(x, y) <- Edge(x, y);
        Reach(x, z) <- Edge(x, y), Reach(y, z);
        Free(x) <- Edge(x, _), !Blocked(x), (x > 3);
    }
}

use PlanField::*;

fn atom(relation: &'static str, negated: bool, fields: Vec<PlanField>) -> PlanAtom {
    PlanAtom {
        relation,
        negated,
        fields,
    }
}

#[test]
fn star_binding_analysis_matches_crepe() {
    let p = star::crepe_plan();
    assert_eq!(p.rules.len(), 1);
    let r = &p.rules[0];
    assert_eq!(r.goal, atom("Answer", false, vec![BoundVar("z")]));
    assert_eq!(
        r.body,
        vec![
            PlanClause::Atom(atom("Parent", false, vec![Const(17), NewVar("y")])),
            PlanClause::Atom(atom("Parent", false, vec![BoundVar("y"), NewVar("z")])),
            PlanClause::Atom(atom("Allowed", false, vec![BoundVar("y")])),
        ]
    );
    assert!(!r.recursive);
    let kinds: Vec<_> = p
        .relations
        .iter()
        .map(|r| (r.name, r.arity, r.kind))
        .collect();
    assert_eq!(
        kinds,
        vec![
            ("Allowed", 1, PlanRelKind::Input),
            ("Answer", 1, PlanRelKind::Output),
            ("Parent", 2, PlanRelKind::Input),
        ]
    );
}

#[test]
fn recursion_negation_and_conditions_are_reported_not_hidden() {
    let p = mixed::crepe_plan();
    let reach_rec = &p.rules[1];
    assert!(reach_rec.recursive, "Reach depends on itself");
    assert!(!p.rules[0].recursive);
    let free = &p.rules[2];
    assert_eq!(
        free.body,
        vec![
            PlanClause::Atom(atom("Edge", false, vec![NewVar("x"), Ignored])),
            PlanClause::Atom(atom("Blocked", true, vec![BoundVar("x")])),
            PlanClause::Unsupported("condition"),
        ]
    );
    // Strata are in evaluation order: a relation's stratum comes after the
    // strata of everything it reads.
    let at = |n: &str| p.strata.iter().position(|s| s.contains(&n)).unwrap();
    assert!(at("Edge") < at("Reach"));
    assert!(at("Blocked") < at("Free"));
    assert_eq!(free.stratum, at("Free"));
}
