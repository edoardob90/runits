//! Expression AST for source-side input parsing.
//!
//! The parser walks the `expression` production in `grammar.pest` into an
//! [`Expr`] tree. Evaluation lives in [`crate::eval`]; the two modules stay
//! separated so the tree is a pure data structure that a future renderer
//! (pretty-printer, source-span error formatter, etc.) can traverse without
//! dragging in the evaluator's context.
//!
//! ## AST shape
//!
//! | Variant | Meaning |
//! |---|---|
//! | `Number(f64)` | literal decimal / scientific |
//! | `Ident(String)` | bare identifier — resolved to unit or constant at eval time |
//! | `Previous` | the `_` previous-result variable |
//! | `BinOp(op, lhs, rhs)` | `+ - * /` |
//! | `Neg(inner)` | unary `-` (unary `+` is a no-op, not represented) |
//! | `Pow(base, n)` | integer exponent only |
//! | `FuncCall(name, args)` | `sqrt(9 m^2)`, `sin(0)`, ... |
//!
//! `_` is deliberately a separate variant and not an `Ident("_")` so a
//! future unit or constant named `_` could not silently hijack the
//! previous-result variable.

use crate::error::RUnitsError;
use crate::parser::{QuantityParser, Rule};
use pest::Parser;
use pest::iterators::Pair;

/// An expression AST node.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Dimensionless numeric literal.
    Number(f64),
    /// Bare identifier — resolved against the unit/constant databases by the
    /// evaluator.
    Ident(String),
    /// The previous-result variable `_`.
    Previous,
    /// Binary arithmetic: `lhs op rhs`.
    BinOp(BinOp, Box<Expr>, Box<Expr>),
    /// Unary negation.
    Neg(Box<Expr>),
    /// Integer power: `base^n`.
    Pow(Box<Expr>, i32),
    /// Function call: `name(args...)`.
    FuncCall(String, Vec<Expr>),
}

/// Binary arithmetic operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl BinOp {
    /// Character form used in error messages.
    pub fn as_char(self) -> char {
        match self {
            BinOp::Add => '+',
            BinOp::Sub => '-',
            BinOp::Mul => '*',
            BinOp::Div => '/',
        }
    }
}

/// Parse a full source-side expression string into an AST.
///
/// Wraps the pest grammar's `expression` entry point, returns the assembled
/// [`Expr`] or a [`RUnitsError::Parse`] pointing at the offending span.
pub fn parse_expression(input: &str) -> Result<Expr, RUnitsError> {
    let mut pairs = QuantityParser::parse(Rule::expression, input.trim()).map_err(Box::new)?;
    let expression_pair = pairs.next().expect("grammar guarantees one expression");
    let add_expr_pair = expression_pair
        .into_inner()
        .next()
        .expect("grammar guarantees add_expr inside expression");
    Ok(build_add(add_expr_pair))
}

// ---------------------------------------------------------------------------
// Tree walkers. Each function corresponds to one grammar rule and folds its
// children left-to-right into an `Expr`.
// ---------------------------------------------------------------------------

fn build_add(pair: Pair<Rule>) -> Expr {
    debug_assert_eq!(pair.as_rule(), Rule::add_expr);
    let mut inner = pair.into_inner();
    let first = inner.next().expect("add_expr has at least one div_expr");
    let mut result = build_div(first);
    while let Some(op_pair) = inner.next() {
        let rhs_pair = inner.next().expect("add_op is followed by a div_expr");
        let op = match op_pair.as_str() {
            "+" => BinOp::Add,
            "-" => BinOp::Sub,
            other => unreachable!("unexpected add_op: {other:?}"),
        };
        let rhs = build_div(rhs_pair);
        result = Expr::BinOp(op, Box::new(result), Box::new(rhs));
    }
    result
}

fn build_div(pair: Pair<Rule>) -> Expr {
    debug_assert_eq!(pair.as_rule(), Rule::div_expr);
    let mut inner = pair.into_inner();
    let first = inner.next().expect("div_expr has at least one mul_expr");
    let mut result = build_mul(first);
    for mul_pair in inner {
        let rhs = build_mul(mul_pair);
        result = Expr::BinOp(BinOp::Div, Box::new(result), Box::new(rhs));
    }
    result
}

fn build_mul(pair: Pair<Rule>) -> Expr {
    debug_assert_eq!(pair.as_rule(), Rule::mul_expr);
    let mut inner = pair.into_inner();
    let first = inner.next().expect("mul_expr has at least one pow_expr");
    let mut result = build_pow_like(first);
    // mul_expr children alternate between `pow_expr` (leading, and after an
    // explicit `*`) and `pow_nosign` (juxtaposed). Pest silently drops the
    // literal `"*"` token, so we don't have to distinguish between explicit
    // and juxtaposed multiplication — both become a `Mul` BinOp here.
    for child in inner {
        let rhs = build_pow_like(child);
        result = Expr::BinOp(BinOp::Mul, Box::new(result), Box::new(rhs));
    }
    result
}

/// Dispatch a mul_expr child to the right pow builder.
///
/// Either `pow_expr` (allows unary prefix) or `pow_nosign` (no prefix,
/// juxtaposed). Both have the same shape beneath the first child, so the
/// two implementations are thin wrappers around the shared exponent logic.
fn build_pow_like(pair: Pair<Rule>) -> Expr {
    match pair.as_rule() {
        Rule::pow_expr => build_pow(pair),
        Rule::pow_nosign => {
            let mut inner = pair.into_inner();
            let base_pair = inner
                .next()
                .expect("pow_nosign has at least one atom_expr child");
            let base = build_atom(base_pair);
            if let Some(exp_pair) = inner.next() {
                debug_assert_eq!(exp_pair.as_rule(), Rule::integer);
                let n: i32 = exp_pair
                    .as_str()
                    .parse()
                    .expect("grammar validated integer");
                Expr::Pow(Box::new(base), n)
            } else {
                base
            }
        }
        other => unreachable!("unexpected rule in mul_expr child: {other:?}"),
    }
}

fn build_pow(pair: Pair<Rule>) -> Expr {
    debug_assert_eq!(pair.as_rule(), Rule::pow_expr);
    let mut inner = pair.into_inner();
    let base_pair = inner.next().expect("pow_expr has at least one unary_atom");
    let base = build_unary(base_pair);
    if let Some(exp_pair) = inner.next() {
        debug_assert_eq!(exp_pair.as_rule(), Rule::integer);
        let n: i32 = exp_pair
            .as_str()
            .parse()
            .expect("grammar validated integer");
        Expr::Pow(Box::new(base), n)
    } else {
        base
    }
}

fn build_unary(pair: Pair<Rule>) -> Expr {
    debug_assert_eq!(pair.as_rule(), Rule::unary_atom);
    let mut inner = pair.into_inner();
    let first = inner.next().expect("unary_atom has at least an atom");
    if first.as_rule() == Rule::unary_op {
        let atom = inner.next().expect("unary_op is followed by an atom");
        let inner_expr = build_atom(atom);
        match first.as_str() {
            "-" => Expr::Neg(Box::new(inner_expr)),
            // Unary `+` is a no-op — skip the wrapper.
            "+" => inner_expr,
            other => unreachable!("unexpected unary_op: {other:?}"),
        }
    } else {
        build_atom(first)
    }
}

fn build_atom(pair: Pair<Rule>) -> Expr {
    match pair.as_rule() {
        Rule::number => {
            let v: f64 = pair.as_str().parse().expect("grammar validated number");
            Expr::Number(v)
        }
        Rule::func_call => {
            let mut inner = pair.into_inner();
            let name = inner
                .next()
                .expect("func_call has an ident")
                .as_str()
                .to_string();
            let args = match inner.next() {
                Some(arg_list_pair) => arg_list_pair
                    .into_inner()
                    .map(build_add)
                    .collect::<Vec<_>>(),
                None => Vec::new(),
            };
            Expr::FuncCall(name, args)
        }
        Rule::paren_expr => {
            let inner_add = pair
                .into_inner()
                .next()
                .expect("paren_expr wraps an add_expr");
            build_add(inner_add)
        }
        Rule::previous => Expr::Previous,
        Rule::ident_atom => {
            let ident = pair.into_inner().next().expect("ident_atom wraps an ident");
            Expr::Ident(ident.as_str().to_string())
        }
        other => unreachable!("unexpected atom rule: {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Expr {
        parse_expression(s).unwrap()
    }

    #[test]
    fn number_literal() {
        assert_eq!(parse("42"), Expr::Number(42.0));
    }

    #[test]
    fn scientific_number() {
        assert_eq!(parse("6.022e23"), Expr::Number(6.022e23));
    }

    #[test]
    fn negative_scientific() {
        // Parsed as Neg(Number), not a negative literal — the grammar's
        // `number` token is unsigned and `-` is an `unary_op`.
        assert_eq!(parse("-2e-3"), Expr::Neg(Box::new(Expr::Number(2e-3))));
    }

    #[test]
    fn bare_ident_resolves_at_eval_time() {
        assert_eq!(parse("meter"), Expr::Ident("meter".to_string()));
    }

    #[test]
    fn previous_variable() {
        assert_eq!(parse("_"), Expr::Previous);
    }

    #[test]
    fn juxtaposition_is_multiplication() {
        // `10 m` → `10 * m`
        let e = parse("10 m");
        let expected = Expr::BinOp(
            BinOp::Mul,
            Box::new(Expr::Number(10.0)),
            Box::new(Expr::Ident("m".to_string())),
        );
        assert_eq!(e, expected);
    }

    #[test]
    fn explicit_mul_and_juxtaposition_mix() {
        // `3*4 m` → `((3*4) * m)`
        let e = parse("3*4 m");
        let expected = Expr::BinOp(
            BinOp::Mul,
            Box::new(Expr::BinOp(
                BinOp::Mul,
                Box::new(Expr::Number(3.0)),
                Box::new(Expr::Number(4.0)),
            )),
            Box::new(Expr::Ident("m".to_string())),
        );
        assert_eq!(e, expected);
    }

    #[test]
    fn pow_binds_tighter_than_mul() {
        // `2^10 byte` → `((2^10) * byte)`
        let e = parse("2^10 byte");
        let expected = Expr::BinOp(
            BinOp::Mul,
            Box::new(Expr::Pow(Box::new(Expr::Number(2.0)), 10)),
            Box::new(Expr::Ident("byte".to_string())),
        );
        assert_eq!(e, expected);
    }

    #[test]
    fn div_binds_looser_than_mul() {
        // `kg/m*s` → `kg / (m*s)`. Matches Phase 3's GNU-Units convention.
        let e = parse("kg/m*s");
        let expected = Expr::BinOp(
            BinOp::Div,
            Box::new(Expr::Ident("kg".to_string())),
            Box::new(Expr::BinOp(
                BinOp::Mul,
                Box::new(Expr::Ident("m".to_string())),
                Box::new(Expr::Ident("s".to_string())),
            )),
        );
        assert_eq!(e, expected);
    }

    #[test]
    fn add_binds_loosest() {
        // `5 m + 3 ft` → `(5*m) + (3*ft)`
        let e = parse("5 m + 3 ft");
        let expected = Expr::BinOp(
            BinOp::Add,
            Box::new(Expr::BinOp(
                BinOp::Mul,
                Box::new(Expr::Number(5.0)),
                Box::new(Expr::Ident("m".to_string())),
            )),
            Box::new(Expr::BinOp(
                BinOp::Mul,
                Box::new(Expr::Number(3.0)),
                Box::new(Expr::Ident("ft".to_string())),
            )),
        );
        assert_eq!(e, expected);
    }

    #[test]
    fn func_call_single_arg() {
        let e = parse("sqrt(9)");
        let expected = Expr::FuncCall("sqrt".to_string(), vec![Expr::Number(9.0)]);
        assert_eq!(e, expected);
    }

    #[test]
    fn func_call_arg_is_full_expression() {
        let e = parse("sqrt(9 m^2)");
        let expected = Expr::FuncCall(
            "sqrt".to_string(),
            vec![Expr::BinOp(
                BinOp::Mul,
                Box::new(Expr::Number(9.0)),
                Box::new(Expr::Pow(Box::new(Expr::Ident("m".to_string())), 2)),
            )],
        );
        assert_eq!(e, expected);
    }

    #[test]
    fn double_negation() {
        // `-(-2 m)` parses as `-(  (-2) * m  )` — unary binds tighter than
        // juxtaposition, so the inner `-2 m` becomes `Neg(2) * m` and the
        // outer `-` wraps the whole parenthesized result. Numerically this
        // is `-(-2 * 1)` = `2`, same as `Neg(Neg(2*m))`.
        let e = parse("-(-2 m)");
        let expected = Expr::Neg(Box::new(Expr::BinOp(
            BinOp::Mul,
            Box::new(Expr::Neg(Box::new(Expr::Number(2.0)))),
            Box::new(Expr::Ident("m".to_string())),
        )));
        assert_eq!(e, expected);
    }

    #[test]
    fn underscore_prefix_is_not_previous() {
        // `_foo` must NOT match `_` then leave `foo` dangling — it should
        // fail the grammar cleanly (no identifier starting with `_`).
        assert!(parse_expression("_foo m").is_err());
    }

    #[test]
    fn lone_underscore_parses() {
        assert_eq!(parse("_"), Expr::Previous);
    }

    #[test]
    fn unary_plus_is_noop() {
        // `+5 m` should parse as `5*m`, not `Neg(5*m)`.
        let e = parse("+5 m");
        let expected = Expr::BinOp(
            BinOp::Mul,
            Box::new(Expr::Number(5.0)),
            Box::new(Expr::Ident("m".to_string())),
        );
        assert_eq!(e, expected);
    }
}
