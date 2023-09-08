use super::{Name, Number};
use itertools::Itertools;
use std::{collections::HashMap, fmt};

#[derive(Debug, Clone, Default)]
pub struct DefinitionBook {
  pub defs: HashMap<Name, Definition>,
}

#[derive(Debug, Clone)]
pub struct Definition {
  pub name: Name,
  pub rules: Vec<Rule>,
}

#[derive(Debug, Clone)]
pub struct Rule {
  pub name: Name,
  pub pats: Vec<Pattern>,
  pub body: Term,
}

#[derive(Debug, Clone)]
pub enum Pattern {
  _Ctr(Name, Vec<Pattern>),
  _Num(Number),
  _Var(Name),
}

#[derive(Debug, Clone)]
pub enum Term {
  Lam { nam: Name, bod: Box<Term> },
  Var { nam: Name },
  App { fun: Box<Term>, arg: Box<Term> },
  Dup { fst: Name, snd: Name, val: Box<Term>, nxt: Box<Term> },
  Num { val: Number },
  NumOp { op: NumOper, fst: Box<Term>, snd: Box<Term> },
  Sup { fst: Box<Term>, snd: Box<Term> },
  Era,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NumOper {
  Add,
  Sub,
  Mul,
  Div,
  Mod,
  And,
  Or,
  Xor,
  Shl,
  Shr,
  Ltn,
  Lte,
  Gtn,
  Gte,
  Eql,
  Neq,
}

impl From<NumOper> for u8 {
  fn from(value: NumOper) -> Self {
    match value {
      NumOper::Add => 0x0,
      NumOper::Sub => 0x1,
      NumOper::Mul => 0x2,
      NumOper::Div => 0x3,
      NumOper::Mod => 0x4,
      NumOper::And => 0x5,
      NumOper::Or => 0x6,
      NumOper::Xor => 0x7,
      NumOper::Shl => 0x8,
      NumOper::Shr => 0x9,
      NumOper::Ltn => 0xa,
      NumOper::Lte => 0xb,
      NumOper::Gtn => 0xc,
      NumOper::Gte => 0xd,
      NumOper::Eql => 0xe,
      NumOper::Neq => 0xf,
    }
  }
}

impl TryFrom<u8> for NumOper {
  type Error = ();

  fn try_from(value: u8) -> Result<Self, Self::Error> {
    match value {
      0 => Ok(NumOper::Add),
      1 => Ok(NumOper::Sub),
      2 => Ok(NumOper::Mul),
      3 => Ok(NumOper::Div),
      4 ..= 15 => todo!(),
      _ => Err(()),
    }
  }
}

impl DefinitionBook {
  pub fn new() -> Self {
    Default::default()
  }
}

impl From<Pattern> for Term {
  fn from(value: Pattern) -> Self {
    match value {
      Pattern::_Ctr(nam, args) => args
        .into_iter()
        .fold(Term::Var { nam }, |acc, arg| Term::App { fun: Box::new(acc), arg: Box::new(arg.into()) }),
      Pattern::_Num(num) => Term::Num { val: num },
      Pattern::_Var(nam) => Term::Var { nam },
    }
  }
}

impl fmt::Display for NumOper {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      NumOper::Add => write!(f, "+"),
      NumOper::Sub => write!(f, "-"),
      NumOper::Mul => write!(f, "*"),
      NumOper::Div => write!(f, "/"),
      NumOper::Mod => write!(f, "%"),
      NumOper::And => write!(f, "&"),
      NumOper::Or => write!(f, "|"),
      NumOper::Xor => write!(f, "^"),
      NumOper::Shl => write!(f, "<<"),
      NumOper::Shr => write!(f, ">>"),
      NumOper::Ltn => write!(f, "<"),
      NumOper::Lte => write!(f, "<="),
      NumOper::Gtn => write!(f, ">"),
      NumOper::Gte => write!(f, ">="),
      NumOper::Eql => write!(f, "=="),
      NumOper::Neq => write!(f, "!="),
    }
  }
}

impl fmt::Display for Term {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Term::Lam { nam, bod } => write!(f, "λ{nam} {bod}"),
      Term::Var { nam } => write!(f, "{nam}"),
      Term::App { fun, arg } => write!(f, "({fun} {arg})"),
      Term::Dup { fst, snd, val, nxt } => write!(f, "dup {fst} {snd} = {val}; {nxt}"),
      Term::Num { val } => write!(f, "{val}"),
      Term::NumOp { op, fst, snd } => write!(f, "({op} {fst} {snd})"),
      Term::Sup { fst, snd } => write!(f, "{{{fst} {snd}}}"),
      Term::Era => write!(f, "*"),
    }
  }
}

impl fmt::Display for Pattern {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", Term::from(self.clone()))
  }
}

impl fmt::Display for Rule {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let Rule { name, pats, body } = self;
    writeln!(f, "({}{}) = {}", name, pats.iter().map(|x| format!(" {x}")).join(""), body)
  }
}

impl fmt::Display for Definition {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for rule in &self.rules {
      write!(f, "{rule}")?
    }
    Ok(())
  }
}

impl fmt::Display for DefinitionBook {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.defs.values().map(|x| x.to_string()).join("\n"))
  }
}
