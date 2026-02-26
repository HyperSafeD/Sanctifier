use serde::Serialize;
use z3::ast::Int;
use z3::{Context, SatResult, Solver};

/// Represents an invariant issue found by the SMT solver.
#[derive(Debug, Serialize, Clone)]
pub struct SmtInvariantIssue {
    pub function_name: String,
    pub description: String,
    pub location: String,
}

pub struct SmtVerifier<'ctx> {
    ctx: &'ctx Context,
    solver: Solver<'ctx>,
}

impl<'ctx> SmtVerifier<'ctx> {
    pub fn new(ctx: &'ctx Context) -> Self {
        Self {
            ctx,
            solver: Solver::new(ctx),
        }
    }

    /// Proof-of-Concept: Uses Z3 to prove if `a + b` can overflow a 64-bit integer
    /// under unconstrained conditions.
    pub fn verify_addition_overflow(
        &self,
        fn_name: &str,
        location: &str,
    ) -> Option<SmtInvariantIssue> {
        let a = Int::new_const(self.ctx, "a");
        let b = Int::new_const(self.ctx, "b");

        // u64 bounds
        let zero = Int::from_u64(self.ctx, 0);
        let max_u64 = Int::from_u64(self.ctx, u64::MAX);

        // Constrain variables to valid u64 limits: 0 <= a, b <= u64::MAX
        self.solver.assert(&a.ge(&zero));
        self.solver.assert(&a.le(&max_u64));
        self.solver.assert(&b.ge(&zero));
        self.solver.assert(&b.le(&max_u64));

        // To prove overflow is IMPOSSIBLE, we assert the violation (a + b > max_u64)
        // and check if the solver can SATISFY this violation.
        let sum = Int::add(self.ctx, &[&a, &b]);
        self.solver.assert(&sum.gt(&max_u64));

        if self.solver.check() == SatResult::Sat {
            // A model exists where a + b > u64::MAX, meaning an overflow is mathematically possible
            Some(SmtInvariantIssue {
                function_name: fn_name.to_string(),
                description: "SMT Solver (Z3) proved that this addition can overflow u64 bounds."
                    .to_string(),
                location: location.to_string(),
            })
        } else {
            None
        }
    }
}
