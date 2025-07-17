use std::rc::Rc;

use once_cell::sync::Lazy;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Label(u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Condition_ {
    True,
    False,
    Base(u64),
    Not(Condition),
    And(Vec<Condition>),
    Or(Vec<Condition>),
}

pub type Condition = Rc<Condition_>;

#[derive(Clone, Debug)]
/// Initial / Input Nodes
pub enum Node {
    Virtual,
    Code {
        label: Label,
        id: u64,
        // Without loss of generality, we assume we have added a virtual return
        next: Label,
    },
    Conditional {
        label: Label,
        test: Condition,
        conseq: Label,
        alt: Label,
    },
    Structured(Label, Structured),
}

#[derive(Clone, Debug)]
/// Result / Structured Nodes
pub enum Structured {
    CodeBlock(u64),
    Seq(Vec<Structured>),
    If {
        test: Condition,
        conseq: Box<Structured>,
        alt: Option<Box<Structured>>,
    },
    Loop {
        body: Box<Structured>,
    },
    WhileLoop {
        test: Condition,
        body: Box<Structured>,
    },
    Break,
}

impl Label {
    pub fn dummy() -> Self {
        Self(u64::MAX)
    }
}

impl Node {
    pub fn label(&self) -> Label {
        match self {
            Node::Virtual => Label::dummy(),
            Node::Code { label, .. } => *label,
            Node::Conditional { label, .. } => *label,
            Node::Structured(label, _) => *label,
        }
    }
}

thread_local! {
    pub static TRUE: Condition = Rc::new(Condition_::True);
    pub static FALSE: Condition = Rc::new(Condition_::False);
}

pub trait Predicate {
    fn simplify(&self) -> Self;
    fn is_true(&self) -> bool;
    fn true_value() -> Self;
    fn false_value() -> Self;
}

impl Predicate for Condition {
    // TODO: This does not do the "obvious" flattenings of AND(A, NOT(A)) or OR(A, NOT(A)), or
    // removal of duplicates, etc. A realistic version of this code would also consider what the
    // actual tested predicates are, e.g., `rax == 0`, etc., toward restructuring switches.
    fn simplify(&self) -> Self {
        // TODO: test this code -- a lot
        fn distribute_and(entries: Vec<Condition>) -> Condition {
            if entries.is_empty() {
                return Predicate::true_value();
            }

            let mut result: Vec<Vec<Condition>> = vec![vec![]];

            for cond in entries {
                match &*cond {
                    Condition_::Or(alts) => {
                        let mut new_result = vec![];
                        for alt in alts {
                            for clause in &result {
                                let mut new_clause = clause.clone();
                                new_clause.push(Rc::clone(alt));
                                new_result.push(new_clause);
                            }
                        }
                        result = new_result;
                    }
                    _other => {
                        for clause in &mut result {
                            clause.push(cond.clone());
                        }
                    }
                }
            }

            let disjuncts: Vec<Condition> = result
                .into_iter()
                .map(|clause| {
                    if clause.len() == 1 {
                        clause.into_iter().next().unwrap()
                    } else {
                        Rc::new(Condition_::And(clause))
                    }
                })
                .collect();

            if disjuncts.len() == 1 {
                disjuncts.into_iter().next().unwrap()
            } else {
                Rc::new(Condition_::Or(disjuncts))
            }
        }

        match &**self {
            Condition_::True | Condition_::False | Condition_::Base(_) => Rc::clone(self),
            Condition_::Not(inner) => {
                match &**inner {
                    Condition_::True => Self::false_value(),
                    Condition_::False => Self::true_value(),
                    Condition_::Base(_) => Rc::clone(self),
                    Condition_::Not(inner) => inner.simplify(),
                    // De Morgans
                    Condition_::And(inners) => {
                        let inners = inners
                            .into_iter()
                            .map(|inner| Rc::new(Condition_::Not(Rc::clone(inner))))
                            .collect();
                        Rc::new(Condition_::Or(inners)).simplify()
                    }
                    Condition_::Or(inners) => {
                        let inners = inners
                            .into_iter()
                            .map(|inner| Rc::new(Condition_::Not(Rc::clone(inner))))
                            .collect();
                        Rc::new(Condition_::And(inners)).simplify()
                    }
                }
            }
            Condition_::And(inners) => {
                let simplified: Vec<_> = inners.into_iter().map(|inner| inner.simplify()).collect();
                distribute_and(simplified)
            }
            Condition_::Or(inners) => {
                let mut result = vec![];
                for inner in inners {
                    let simplified_inner = inner.simplify();
                    match &*simplified_inner {
                        Condition_::Or(inner) => result.extend(inner.into_iter().map(Rc::clone)),
                        _other => result.push(simplified_inner),
                    }
                }
                if result.is_empty() {
                    Self::false_value() // No entries implies `False` for `or`
                } else if result.iter().any(|partial| partial.is_true()) {
                    Self::true_value()
                } else if result.len() == 1 {
                    Rc::clone(&result[0])
                } else {
                    Rc::new(Condition_::Or(result))
                }
            }
        }
    }

    fn is_true(&self) -> bool {
        match &**self {
            Condition_::True => true,
            _ => false,
        }
    }

    fn true_value() -> Self {
        TRUE.with(Rc::clone)
    }
    fn false_value() -> Self {
        FALSE.with(Rc::clone)
    }
}
/*
impl Condition {
    /// Convert to Disjunctive Normal Form
    pub fn simplify(&self) -> Self {
        fn distribute_and(conds: Vec<Rc<Condition>>) -> Condition {
            if conds.is_empty() {
                return Condition::True;
            }

            let mut result: Vec<Vec<Rc<Condition>>> = vec![vec![]];

            for cond in conds {
                match &*cond {
                    Condition::Or(alts) => {
                        let mut new_result = vec![];
                        for alt in alts {
                            for clause in &result {
                                let mut new_clause = clause.clone();
                                new_clause.push(alt.clone());
                                new_result.push(new_clause);
                            }
                        }
                        result = new_result;
                    }
                    other => {
                        for clause in &mut result {
                            clause.push(cond.clone());
                        }
                    }
                }
            }

            let disjuncts: Vec<Condition> = result
                .into_iter()
                .map(|clause| {
                    if clause.len() == 1 {
                        clause.into_iter().next().unwrap()
                    } else {
                        Condition::And(clause)
                    }
                })
                .collect();

            if disjuncts.len() == 1 {
                disjuncts.into_iter().next().unwrap()
            } else {
                Condition::Or(disjuncts)
            }
        }

        match self {
            Condition::True | Condition::False | Condition::Base(_) => self.clone(),
            Condition::Not(inner) => match &**inner {
                Condition::True => Condition::False,
                Condition::False => Condition::True,
                Condition::Base(_) => Condition::Not(inner.clone()),
                Condition::Not(inner) => inner.simplify(),
                // De Morgans
                Condition::And(inners) => {
                    let inners = inners
                        .into_iter()
                        .map(|inner| Rc::new(Condition::Not(inner.clone())))
                        .collect();
                    Condition::Or(inners).simplify()
                }
                Condition::Or(inners) => {
                    let inners = inners
                        .into_iter()
                        .map(|inner| Rc::new(Condition::Not(inner.clone())))
                        .collect();
                    Condition::And(inners).simplify()
                }
            },
            Condition::And(inners) => {
                let simplified: Vec<_> = inners.into_iter().map(|inner| inner.simplify()).collect();
                distribute_and(simplified)
            }
            Condition::Or(inners) => {
                let mut result = vec![];
                for inner in inners {
                    match inner.simplify() {
                        Condition::Or(inner) => result.extend(inner),
                        other => result.push(Rc::new(other)),
                    }
                }
                Condition::Or(result)
            }
        }
    }
}
*/
