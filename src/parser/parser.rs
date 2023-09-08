use std::{iter::Map, ops::Range};

use crate::{
  ast::{Definition, DefinitionBook, Name, NumOper, Rule, Term},
  parser::lexer::Token,
};
use chumsky::{
  extra,
  input::{Emitter, SpannedInput, Stream, ValueInput},
  prelude::{Input, Rich},
  primitive::{choice, just},
  recursive::recursive,
  select,
  span::SimpleSpan,
  IterParser, Parser,
};
use logos::{Logos, SpannedIter};

use super::lexer::LexingError;

// TODO: Pattern matching on rules
// TODO: Other types of numbers
/// <Name>   ::= <name_token> // [_a-zA-Z][_a-zA-Z0-9]*
/// <Number> ::= <number_token> // [0-9]+
/// <Var>    ::= <Name>
/// <Nested> ::= "(" <newline_token>* <Term> <newline_token>* ")"
/// <Item>   ::= <Var> | <Number> | <Nested>
/// <App>    ::= <Item> <Item>+
/// <Lam>    ::= ("λ"|"\") <Name> <Term>
/// <Dup>    ::= "dup" <Name> <Name> "=" <Term> ";" <NewLine>* <Term>
/// <Let>    ::= "let" <Name> "=" <Term> ";" <NewLine>* <Term>
/// <NumOp>  ::= <numop_token> <Item> <Item>
/// <Term>   ::= <Lam> | <App> | <Dup> | <Let> | <NumOp> | <Item>
/// <Rule>   ::= "(" <Name> ")" "=" <newline_token>* <Term>
/// <Def>    ::= <NewLine>* <Rule> (<NewLine>+ <Rule>)*
/// <Book>   ::= <Def>+ // Sequential rules grouped by name
pub fn parse_definition_book(code: &str) -> Result<DefinitionBook, Vec<Rich<Token>>> {
  book_parser().parse(token_stream(code)).into_result()
}

pub fn parse_term(code: &str) -> Result<Term, Vec<Rich<Token>>> {
  // TODO: Make a function that calls a parser. I couldn't figure out how to type it correctly.
  term_parser().parse(token_stream(code)).into_result()
}

fn token_stream(
  code: &str,
) -> SpannedInput<
  Token,
  SimpleSpan,
  Stream<
    Map<SpannedIter<Token>, impl FnMut((Result<Token, LexingError>, Range<usize>)) -> (Token, SimpleSpan)>,
  >,
> {
  // TODO: Maybe change to just using chumsky.
  // The integration is not so smooth and we need to figure out
  // errors, spans and other things that are not so obvious.
  let token_iter = Token::lexer(code).spanned().map(|(token, span)| match token {
    Ok(t) => (t, SimpleSpan::from(span)),
    Err(e) => (Token::Error(e), SimpleSpan::from(span)),
  });
  Stream::from_iter(token_iter).spanned(SimpleSpan::from(code.len() .. code.len()))
}

// Parsers
const MAX_NAME_LEN: u32 = (u64::BITS - u16::BITS) / 64_u32.ilog2();

fn name_parser<'a, I>() -> impl Parser<'a, I, Name, extra::Err<Rich<'a, Token>>>
where
  I: ValueInput<'a, Token = Token, Span = SimpleSpan>,
{
  select!(Token::Name(name) => Name(name)).validate(|name, span, emitter| {
    if name.len() > MAX_NAME_LEN as usize {
      // TODO: Implement some kind of name mapping for definitions so that we can fit any def size.
      // e.g. sequential mapping, mangling, hashing, etc
      emitter.emit(Rich::custom(span, format!("'{}' exceed maximum name length of {}", *name, MAX_NAME_LEN)))
    }
    name
  })
}

fn term_parser<'a, I>() -> impl Parser<'a, I, Term, extra::Err<Rich<'a, Token>>>
where
  I: ValueInput<'a, Token = Token, Span = SimpleSpan>,
{
  let new_line = just(Token::NewLine).repeated();
  let number = select!(Token::Number(num) => Term::Num{val: num});

  let num_oper = select! {
    Token::Add => NumOper::Add,
    Token::Sub => NumOper::Sub,
    Token::Mul => NumOper::Mul,
    Token::Div => NumOper::Div,
    Token::Mod => NumOper::Mod,
    Token::And => NumOper::And,
    Token::Or => NumOper::Or,
    Token::Xor => NumOper::Xor,
    Token::Shl => NumOper::Shl,
    Token::Shr => NumOper::Shr,
    Token::Lte => NumOper::Lte,
    Token::Ltn => NumOper::Ltn,
    Token::Gte => NumOper::Gte,
    Token::Gtn => NumOper::Gtn,
    Token::EqualsEquals => NumOper::Eql,
    Token::NotEquals => NumOper::Neq,
  };

  let var = name_parser().map(|name| Term::Var { nam: name }).boxed();

  recursive(|term| {
    let nested = term
      .clone()
      .delimited_by(new_line.clone(), new_line.clone())
      .delimited_by(just(Token::LParen), just(Token::RParen));

    let item = choice((var, number, nested));

    let app = item
      .clone()
      .foldl(item.clone().repeated(), |fun, arg| Term::App { fun: Box::new(fun), arg: Box::new(arg) });

    let lam = just(Token::Lambda)
      .ignore_then(name_parser())
      .then(term.clone())
      .map(|(name, body)| Term::Lam { nam: name, bod: Box::new(body) })
      .boxed();

    let dup = just(Token::Dup)
      .ignore_then(name_parser())
      .then(name_parser())
      .then_ignore(just(Token::Equals))
      .then(term.clone())
      .then_ignore(just(Token::Semicolon))
      .then_ignore(new_line.clone())
      .then(term.clone())
      .map(|(((fst, snd), val), next)| Term::Dup { fst, snd, val: Box::new(val), nxt: Box::new(next) })
      .boxed();

    let let_ = just(Token::Let)
      .ignore_then(name_parser())
      .then_ignore(just(Token::Equals))
      .then(term.clone())
      .then_ignore(just(Token::Semicolon))
      .then_ignore(new_line.clone())
      .then(term.clone())
      .map(|((name, body), next)| Term::App {
        fun: Box::new(Term::Lam { nam: name, bod: next.into() }),
        arg: Box::new(body),
      })
      .boxed();

    let num_op = num_oper.then(item.clone()).then(item.clone()).map(|((op, fst), snd)| Term::NumOp {
      op,
      fst: Box::new(fst),
      snd: Box::new(snd),
    });

    choice((lam, app, dup, let_, num_op, item))
  })
}

fn rule_parser<'a, I>() -> impl Parser<'a, I, Rule, extra::Err<Rich<'a, Token>>>
where
  I: ValueInput<'a, Token = Token, Span = SimpleSpan>,
{
  just(Token::LParen)
    .ignore_then(name_parser())
    .then_ignore(just(Token::RParen))
    .then_ignore(just(Token::Equals))
    .then_ignore(just(Token::NewLine).repeated())
    .then(term_parser())
    .map(|(name, body)| Rule { name, pats: vec![], body })
}

fn book_parser<'a, I>() -> impl Parser<'a, I, DefinitionBook, extra::Err<Rich<'a, Token>>>
where
  I: ValueInput<'a, Token = Token, Span = SimpleSpan>,
{
  fn rules_to_book(
    rules: Vec<(Rule, SimpleSpan)>,
    _span: SimpleSpan,
    emitter: &mut Emitter<Rich<Token>>,
  ) -> DefinitionBook {
    let mut book = DefinitionBook::new();

    // Check for repeated defs (could be rules out of order or actually repeated names)
    for def_rules in rules.group_by(|(rule1, _), (rule2, _)| rule1.name == rule2.name) {
      if def_rules.len() > 1 {
        // TODO: Enable definitions with multiple rules when implementing pattern matching
        let def_span = SimpleSpan::new(def_rules.first().unwrap().1.start, def_rules.last().unwrap().1.end);
        emitter
          .emit(Rich::custom(def_span, format!("Definition with multiple rules '{}'", *def_rules[0].0.name)));
      } else {
        let (rule, span) = &def_rules[0];
        let def = Definition { name: rule.name.clone(), rules: vec![rule.clone()] };
        if book.defs.contains_key(&def.name) {
          emitter.emit(Rich::custom(*span, format!("Repeated definition '{}'", *def.name)));
        } else {
          book.defs.insert(def.name.clone(), def);
        }
      }
    }
    book
  }

  let new_line = just(Token::NewLine).repeated();

  let parsed_rules = rule_parser()
    .map_with_span(|rule, span| (rule, span))
    .separated_by(new_line.clone().at_least(1))
    .allow_leading()
    .allow_trailing()
    .collect::<Vec<(Rule, SimpleSpan)>>();

  parsed_rules.validate(rules_to_book)
}
