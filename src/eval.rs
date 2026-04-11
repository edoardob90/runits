//! Tree walker / evaluator for [`Expr`](crate::expr::Expr) trees.
//!
//! Takes a parsed AST and produces a [`Quantity`]. The evaluator is kept
//! separate from the AST so that a future pretty-printer, source-span
//! renderer, or alternative back-end (symbolic algebra?) can walk the same
//! tree without pulling in this module's dependencies.
//!
//! ## Identifier resolution order
//!
//! When `Expr::Ident(name)` is evaluated, the resolver tries, in order:
//!
//! 1. **Unit database first** — handles SI/binary prefix stripping, so
//!    `kmeter`, `µs`, `Gbyte` resolve here.
//! 2. **Constants database second** — constants are deliberately named to
//!    avoid collision with prefix letters (`c_0` not `c`, `g_n` not `g`),
//!    so unit-first doesn't steal constant lookups.
//! 3. **Fuzzy suggestions** from both databases on failure.
//!
//! The order is pinned by the `ident_resolution_units_first` test below and
//! matches Numbat's behaviour.

use crate::database::UnitDatabase;
use crate::database::constants::ConstantsDatabase;
use crate::error::RUnitsError;
use crate::expr::{BinOp, Expr};
use crate::math;
use crate::units::Quantity;
use crate::units::Unit;

/// Evaluation context — supplies the databases and the optional previous
/// result for the `_` variable.
///
/// Held as references so the evaluator can borrow from long-lived singletons
/// and a short-lived REPL state without any ownership gymnastics.
pub struct EvalContext<'a> {
    pub units: &'a UnitDatabase,
    pub constants: &'a ConstantsDatabase,
    pub previous: Option<&'a Quantity>,
}

impl<'a> EvalContext<'a> {
    /// Build a context for one-shot CLI evaluation: no previous result,
    /// global constants singleton. The caller supplies the `UnitDatabase`
    /// so tests can use a fresh instance rather than the global singleton.
    pub fn one_shot(units: &'a UnitDatabase) -> Self {
        Self {
            units,
            constants: crate::database::constants::global(),
            previous: None,
        }
    }

    /// Build a context with an explicit previous-result reference. Used by
    /// the REPL, which owns a `last_quantity: Option<Quantity>`.
    pub fn with_previous(
        units: &'a UnitDatabase,
        constants: &'a ConstantsDatabase,
        previous: Option<&'a Quantity>,
    ) -> Self {
        Self {
            units,
            constants,
            previous,
        }
    }
}

/// Evaluate an [`Expr`] under the given context.
///
/// Every successful result is a [`Quantity`]. Every failure is a
/// [`RUnitsError`] carrying enough structured context for the follow-up
/// colored-errors step to render directly.
pub fn eval(expr: &Expr, ctx: &EvalContext) -> Result<Quantity, RUnitsError> {
    match expr {
        Expr::Number(x) => Ok(Quantity::new(*x, Unit::dimensionless())),

        // Resolution order is load-bearing and pinned by test
        // `ident_resolution_units_first`. The order is:
        //
        //   1. **Unit database first.** Prefixed units (`kmeter`, `µs`, `GW`)
        //      are common input and live under prefix-stripped lookup inside
        //      `UnitDatabase::lookup`. Trying units first means a single-char
        //      input like `m` cleanly resolves to `meter` instead of to a
        //      non-existent constant.
        //   2. **Constants database second.** Constants are deliberately
        //      named to avoid collision with unit-prefix letters (`c_0` for
        //      speed_of_light, not `c`; `g_n` for gravity, not `g`) — see
        //      `src/database/constants.rs` module-level docs. So unit-first
        //      doesn't steal constant lookups.
        //   3. **Fuzzy suggestions from BOTH DBs on failure.** Users who
        //      typo a unit might get a constant suggestion and vice versa;
        //      merging both pools gives better hints.
        //
        // **Never call `math::lookup` here.** Bare identifiers are never
        // function references. A function call arrives as
        // `Expr::FuncCall(name, args)` from the parser — the grammar
        // separates them because `sin(0)` and `sin` have different meanings
        // (the latter would be a "the name `sin`" reference, which is not a
        // value we support).
        //
        // **Why `c.unit.clone()`.** `ctx.constants.lookup` returns
        // `Option<&Constant>` (borrowed from the global singleton).
        // `Quantity::new` takes an owned `Unit`. We clone. The clone cost is
        // modest (a small `HashMap` + a short `String`) and happens per
        // expression evaluation, not per function call inside an expression.
        // If profiling ever says this matters, the fix is to make
        // `Quantity::unit` a `Cow<Unit>` — not to thread lifetimes through
        // the evaluator.
        //
        // **What about `c` alone?** Unit DB has no `c` (centi is a prefix,
        // not a unit). Constants DB has no `c` (speed_of_light is `c_0` to
        // avoid the prefix collision). So `eval(Ident("c"))` produces
        // `UnknownIdentifier` — this is the intended behaviour and has a
        // dedicated test.
        Expr::Ident(name) => {
            if let Some(unit) = ctx.units.lookup(name) {
                return Ok(Quantity::new(1.0, unit));
            }
            if let Some(c) = ctx.constants.lookup(name) {
                return Ok(Quantity::new(c.value, c.unit.clone()));
            }
            let mut suggestions = ctx.units.suggest(name, 3);
            for c_suggestion in ctx.constants.suggest(name, 3) {
                if !suggestions.contains(&c_suggestion) {
                    suggestions.push(c_suggestion);
                }
            }
            suggestions.truncate(5);
            Err(RUnitsError::UnknownIdentifier {
                name: name.clone(),
                suggestions,
            })
        }

        Expr::Previous => ctx
            .previous
            .cloned()
            .ok_or(RUnitsError::PreviousResultUnavailable),

        Expr::BinOp(op, lhs, rhs) => {
            let lhs_q = eval(lhs, ctx)?;
            let rhs_q = eval(rhs, ctx)?;
            match op {
                BinOp::Add => lhs_q.try_add(rhs_q),
                BinOp::Sub => lhs_q.try_sub(rhs_q),
                BinOp::Mul => lhs_q.mul(rhs_q),
                BinOp::Div => lhs_q.div(rhs_q),
            }
        }

        Expr::Neg(inner) => Ok(eval(inner, ctx)?.neg()),

        Expr::Pow(base, n) => eval(base, ctx)?.pow_i32(*n),

        Expr::FuncCall(name, args) => {
            let func = math::lookup(name).ok_or_else(|| RUnitsError::UnknownFunction {
                name: name.clone(),
                suggestions: math::suggest(name, 3),
            })?;
            let arg_values: Result<Vec<Quantity>, _> = args.iter().map(|a| eval(a, ctx)).collect();
            func.apply(&arg_values?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::UnitDatabase;
    use crate::expr::parse_expression;

    fn eval_one_shot(src: &str) -> Result<Quantity, RUnitsError> {
        let db = UnitDatabase::new();
        let ctx = EvalContext::one_shot(&db);
        let expr = parse_expression(src)?;
        eval(&expr, &ctx)
    }

    #[test]
    fn bare_number_is_dimensionless() {
        let q = eval_one_shot("42").unwrap();
        assert_eq!(q.value, 42.0);
        assert!(q.unit.dimensions.is_empty());
    }

    #[test]
    fn ident_resolution_units_first() {
        // `m` must resolve to meter (unit DB) even though constants could
        // theoretically introduce a collision. This test pins the order.
        let q = eval_one_shot("m").unwrap();
        assert_eq!(q.unit.name, "meter");
    }

    #[test]
    fn ident_resolution_constants_when_not_in_unit_db() {
        // `c_0` is not a unit; should resolve to speed_of_light.
        let q = eval_one_shot("c_0").unwrap();
        assert!((q.value - 299_792_458.0).abs() < 1.0);
    }

    #[test]
    fn ident_unknown_returns_unknown_identifier() {
        let err = eval_one_shot("xyzzy").unwrap_err();
        assert!(matches!(err, RUnitsError::UnknownIdentifier { .. }));
    }

    #[test]
    fn c_alone_is_unknown_identifier() {
        // Neither a unit (centi is a prefix) nor a constant (speed_of_light
        // is c_0). Intentional — pinned by this test.
        let err = eval_one_shot("c").unwrap_err();
        assert!(matches!(err, RUnitsError::UnknownIdentifier { .. }));
    }

    #[test]
    fn juxtaposition_yields_quantity() {
        let q = eval_one_shot("10 m").unwrap();
        assert_eq!(q.value, 10.0);
        assert_eq!(q.unit.name, "meter");
    }

    #[test]
    fn addition_same_unit() {
        let q = eval_one_shot("5 m + 3 m").unwrap();
        assert!((q.value - 8.0).abs() < 1e-12);
        assert_eq!(q.unit.name, "meter");
    }

    #[test]
    fn addition_compatible_unit_lhs_wins() {
        // 5 m + 3 ft → 5.9144 m (result stays in LHS unit)
        let q = eval_one_shot("5 m + 3 ft").unwrap();
        assert!((q.value - 5.9144).abs() < 1e-9);
        assert_eq!(q.unit.name, "meter");
    }

    #[test]
    fn addition_incompatible_fails_with_dim_strings() {
        let err = eval_one_shot("5 m + 3 s").unwrap_err();
        match err {
            RUnitsError::IncompatibleAddition {
                lhs_dim, rhs_dim, ..
            } => {
                assert!(lhs_dim.contains("Length"));
                assert!(rhs_dim.contains("Time"));
            }
            other => panic!("expected IncompatibleAddition, got {other:?}"),
        }
    }

    #[test]
    fn pow_then_juxtapose() {
        // 2^10 byte → 1024 byte
        let q = eval_one_shot("2^10 byte").unwrap();
        assert!((q.value - 1024.0).abs() < 1e-12);
    }

    #[test]
    fn explicit_mul_chain() {
        // 3*4 meter → 12 meter
        let q = eval_one_shot("3*4 meter").unwrap();
        assert!((q.value - 12.0).abs() < 1e-12);
        assert_eq!(q.unit.name, "meter");
    }

    #[test]
    fn negation() {
        let q = eval_one_shot("-5 m").unwrap();
        assert!((q.value + 5.0).abs() < 1e-12);
    }

    #[test]
    fn double_negation() {
        let q = eval_one_shot("-(-2 m)").unwrap();
        assert!((q.value - 2.0).abs() < 1e-12);
    }

    #[test]
    fn func_call_sqrt_dimension_transform() {
        let q = eval_one_shot("sqrt(9 m^2)").unwrap();
        // 9 m² → 3 m (m² in base factor, so value 3 m)
        assert!((q.value - 3.0).abs() < 1e-12);
        assert_eq!(q.unit.dimension_string(), "Length");
    }

    #[test]
    fn func_call_sin_zero() {
        let q = eval_one_shot("sin(0)").unwrap();
        assert!(q.value.abs() < 1e-12);
    }

    #[test]
    fn unknown_function_suggests() {
        let err = eval_one_shot("sxrt(9)").unwrap_err();
        match err {
            RUnitsError::UnknownFunction { suggestions, .. } => {
                assert!(suggestions.contains(&"sqrt".to_string()));
            }
            other => panic!("expected UnknownFunction, got {other:?}"),
        }
    }

    #[test]
    fn previous_unavailable_errors_cleanly() {
        let err = eval_one_shot("_").unwrap_err();
        assert!(matches!(err, RUnitsError::PreviousResultUnavailable));
    }

    #[test]
    fn previous_when_set_returns_it() {
        let db = UnitDatabase::new();
        let prev = Quantity::new(42.0, Unit::meter());
        let ctx = EvalContext {
            units: &db,
            constants: crate::database::constants::global(),
            previous: Some(&prev),
        };
        let expr = parse_expression("_ + 5 m").unwrap();
        let q = eval(&expr, &ctx).unwrap();
        assert!((q.value - 47.0).abs() < 1e-12);
    }

    #[test]
    fn constant_arithmetic() {
        // 2 * c_0 * 1 s ≈ 599,584,916 m (just dimensions check, big value)
        let q = eval_one_shot("2 * c_0 * 1 s").unwrap();
        // 2 * c_0 has dimensions of velocity, then * s gives length
        assert_eq!(q.unit.dimension_string(), "Length");
        // Numeric: 2 * 299_792_458 = 599_584_916
        assert!((q.value - 599_584_916.0).abs() < 1.0);
    }

    #[test]
    fn affine_addition_fails() {
        let err = eval_one_shot("20 celsius + 5 celsius").unwrap_err();
        assert!(matches!(err, RUnitsError::AffineInExpression { .. }));
    }

    #[test]
    fn arity_mismatch_via_parser() {
        // sqrt takes 1 arg; call with 0 should fail at arity check.
        // (Can't easily pass 0 from parse_expression — func_call requires
        // at least one arg via arg_list? — but we can hand-build the AST.)
        let db = UnitDatabase::new();
        let ctx = EvalContext::one_shot(&db);
        let expr = Expr::FuncCall("sqrt".to_string(), vec![]);
        let err = eval(&expr, &ctx).unwrap_err();
        assert!(matches!(err, RUnitsError::ArityMismatch { .. }));
    }
}
