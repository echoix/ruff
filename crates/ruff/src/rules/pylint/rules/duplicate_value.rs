use rustc_hash::FxHashSet;
use rustpython_parser::ast::{self, Expr, Ranged};

use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};
use ruff_python_ast::comparable::ComparableExpr;

use crate::checkers::ast::Checker;

#[violation]
pub struct DuplicateValue {
    value: String,
}

impl Violation for DuplicateValue {
    #[derive_message_formats]
    fn message(&self) -> String {
        let DuplicateValue { value } = self;
        format!("Duplicate value `{value}` in set")
    }
}

/// PLW0130
pub(crate) fn duplicate_value(checker: &mut Checker, elts: &Vec<Expr>) {
    let mut seen_values: FxHashSet<ComparableExpr> = FxHashSet::default();
    for elt in elts {
        if let Expr::Constant(ast::ExprConstant { value, .. }) = elt {
            let comparable_value: ComparableExpr = elt.into();

            if !seen_values.insert(comparable_value) {
                checker.diagnostics.push(Diagnostic::new(
                    DuplicateValue {
                        value: checker.generator().constant(value),
                    },
                    elt.range(),
                ));
            }
        };
    }
}
