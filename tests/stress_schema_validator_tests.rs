//! Stress and Adversarial Challenge Test Suite for Schema Validator Engine (`src/schema_validator.rs`).
//!
//! Written by Empirical Challenger 1 for Milestone M2.
//! Evaluates boundary conditions, fail-closed behavior, lookback evaluations,
//! vector type enforcement, multi-statement scripts, and backfill propagation.

use alphafind::models::schema_validator::{DiagnosticCode, SchemaValidator};

#[test]
fn test_adversarial_vector_folding_rules() {
    let validator = SchemaValidator::new();

    // 1. Folded vectors pass
    let valid_vec =
        validator.validate("group_neutralize(rank(vec_avg(nws18_ssc)), densify(subindustry))");
    assert!(valid_vec.is_valid);
    assert_eq!(valid_vec.errors.len(), 0);

    let valid_sum =
        validator.validate("group_neutralize(rank(vec_sum(nws18_ssc)), densify(subindustry))");
    assert!(valid_sum.is_valid);

    // 2. Unfolded vectors in arithmetic fail
    let bad_add =
        validator.validate("group_neutralize(rank(nws18_ssc + 1.0), densify(subindustry))");
    assert!(!bad_add.is_valid);
    assert!(bad_add
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrVecTypeMismatch));

    let bad_sub =
        validator.validate("group_neutralize(rank(1.0 - nws18_ssc), densify(subindustry))");
    assert!(!bad_sub.is_valid);
    assert!(bad_sub
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrVecTypeMismatch));

    let bad_mul =
        validator.validate("group_neutralize(rank(nws18_ssc * close), densify(subindustry))");
    assert!(!bad_mul.is_valid);
    assert!(bad_mul
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrVecTypeMismatch));

    // 3. Unfolded vector in comparisons
    let bad_cmp =
        validator.validate("group_neutralize(rank(nws18_ssc > 0.0), densify(subindustry))");
    assert!(!bad_cmp.is_valid);
    assert!(bad_cmp
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrVecTypeMismatch));

    // 4. Unfolded vector in unary
    let bad_neg = validator.validate("group_neutralize(rank(-nws18_ssc), densify(subindustry))");
    assert!(!bad_neg.is_valid);
    assert!(bad_neg
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrVecTypeMismatch));
}

#[test]
fn test_adversarial_trade_when_with_vectors_and_backfill() {
    let validator = SchemaValidator::new();

    // 1. Nested trade_when with low coverage unbackfilled field fails closed
    let nested_low_cov = "group_neutralize(rank(trade_when(close > open, trade_when(volume > 1000, fscore_growth, -1), -1)), densify(subindustry))";
    let rep1 = validator.validate(nested_low_cov);
    assert!(
        !rep1.is_valid,
        "Nested trade_when with unbackfilled low coverage must fail"
    );
    assert!(rep1
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrMissingBackfill));

    // 2. Nested trade_when with backfilled field passes
    let nested_backfilled = "group_neutralize(rank(trade_when(close > open, trade_when(volume > 1000, ts_backfill(fscore_growth, 15), -1), -1)), densify(subindustry))";
    let rep2 = validator.validate(nested_backfilled);
    assert!(
        rep2.is_valid,
        "Nested trade_when with backfill must pass: {:?}",
        rep2.errors
    );

    // 3. Unfolded vector inside trade_when condition fails
    let vec_cond =
        "group_neutralize(rank(trade_when(nws18_ssc > 0, close, -1)), densify(subindustry))";
    let rep3 = validator.validate(vec_cond);
    assert!(!rep3.is_valid);
    assert!(rep3
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrVecTypeMismatch));

    // 4. Unfolded vector inside trade_when payload fails
    let vec_payload =
        "group_neutralize(rank(trade_when(close > open, nws18_ssc, -1)), densify(subindustry))";
    let rep4 = validator.validate(vec_payload);
    assert!(!rep4.is_valid);
    assert!(rep4
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrVecTypeMismatch));
}

#[test]
fn test_adversarial_multi_statement_scripts() {
    let validator = SchemaValidator::new();

    // 1. Multi-statement variable propagation and folding
    let script_valid = r#"
        raw_vec = nws18_ssc;
        folded = vec_avg(raw_vec);
        dampened = ts_decay_linear(folded, 10);
        group_neutralize(rank(dampened), densify(subindustry))
    "#;
    let rep1 = validator.validate(script_valid);
    assert!(
        rep1.is_valid,
        "Valid multi-statement script must pass: {:?}",
        rep1.errors
    );

    // 2. Multi-statement with unfolded vector in intermediate arithmetic fails
    let script_invalid = r#"
        raw_vec = nws18_ssc;
        bad_math = raw_vec + 1.0;
        group_neutralize(rank(bad_math), densify(subindustry))
    "#;
    let rep2 = validator.validate(script_invalid);
    assert!(!rep2.is_valid);
    assert!(rep2
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrVecTypeMismatch));
}

#[test]
fn test_adversarial_negative_lookbacks_and_zero_lags() {
    let validator = SchemaValidator::new();

    // 1. Negative lag in ts_delay
    let rep_neg =
        validator.validate("group_neutralize(rank(ts_delay(close, -10)), densify(subindustry))");
    assert!(!rep_neg.is_valid);
    assert!(rep_neg
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrNegativeLookback));

    // 2. Zero lag in ts_delay
    let rep_zero =
        validator.validate("group_neutralize(rank(ts_delay(close, 0)), densify(subindustry))");
    assert!(!rep_zero.is_valid);
    assert!(rep_zero
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrNegativeLookback));

    // 3. ts_backfill with window < 5
    let rep_sub5 = validator
        .validate("group_neutralize(rank(ts_backfill(fscore_growth, 4)), densify(subindustry))");
    assert!(!rep_sub5.is_valid);
    assert!(rep_sub5
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrMissingBackfill));

    // 4. ts_backfill with non-positive window
    let rep_bfl_zero = validator
        .validate("group_neutralize(rank(ts_backfill(fscore_growth, 0)), densify(subindustry))");
    assert!(!rep_bfl_zero.is_valid);
    assert!(rep_bfl_zero
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrNegativeLookback));
}

#[test]
fn test_adversarial_densify_enforcement() {
    let validator = SchemaValidator::new();

    // 1. Undensified subindustry fails
    let rep_undensified = validator.validate("group_neutralize(rank(close), subindustry)");
    assert!(!rep_undensified.is_valid);
    assert!(rep_undensified
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrUndensifiedGroup));

    // 2. Undensified sector fails
    let rep_sector = validator.validate("group_neutralize(rank(close), sector)");
    assert!(!rep_sector.is_valid);
    assert!(rep_sector
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrUndensifiedGroup));

    // 3. densify wrapping valid group passes
    let rep_valid = validator.validate("group_neutralize(rank(close), densify(sector))");
    assert!(rep_valid.is_valid);
}

#[test]
fn test_adversarial_option9_syntax_and_trailing_d() {
    let validator = SchemaValidator::new();

    // Trailing 'd' is flagged with error
    let rep_trailing_d =
        validator.validate("group_neutralize(rank(call_breakeven_30d), densify(subindustry))");
    assert!(!rep_trailing_d.is_valid);
    assert!(rep_trailing_d
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrUnknownField));

    // Valid horizon without 'd' passes
    let rep_clean = validator.validate("group_neutralize(rank(pcr_vol_30), densify(subindustry))");
    assert!(rep_clean.is_valid);
}

// =============================================================================
// REMEDIATED CHALLENGER VALIDATION TESTS
// =============================================================================

#[test]
fn test_challenger_reproduce_backfill_poisoning_bypass() {
    let validator = SchemaValidator::new();

    // Vulnerability 1 Remediated:
    // fscore_growth (29.8% coverage) MUST NOT be shielded by backfilled fscore_momentum.
    let expr = "group_neutralize(rank(ts_backfill(fscore_momentum, 15) + fscore_growth), densify(subindustry))";
    let rep = validator.validate(expr);

    assert!(
        !rep.is_valid,
        "Remediated: unbackfilled fscore_growth must fail validation even when combined with backfilled operand"
    );
    assert!(rep
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrMissingBackfill));
}

#[test]
fn test_challenger_reproduce_literal_scalar_breaks_price_normalization() {
    let validator = SchemaValidator::new();

    // Vulnerability 2 Remediated:
    // Subtracting 1.0 from (call_breakeven_30 / close) must NOT trigger WarnUnnormalizedPrice.
    let expr = "group_neutralize(rank(call_breakeven_30 / close - 1.0), densify(subindustry))";
    let rep = validator.validate(expr);

    assert!(
        !rep.warnings
            .iter()
            .any(|w| w.code == DiagnosticCode::WarnUnnormalizedPrice),
        "Remediated: Literal 1.0 is dimensionless and must not emit WarnUnnormalizedPrice"
    );
    assert!(
        rep.is_valid,
        "Valid normalized ratio minus 1.0 must pass: {:?}",
        rep.errors
    );
}

#[test]
fn test_challenger_reproduce_binary_expr_negative_lookback_bypass() {
    let validator = SchemaValidator::new();

    // Vulnerability 3 Remediated:
    // Binary expressions like `1 - 2` or variable lookbacks must NOT bypass negative lookback check.
    let expr = "group_neutralize(rank(ts_delay(close, 1 - 2)), densify(subindustry))";
    let rep = validator.validate(expr);

    assert!(
        !rep.is_valid,
        "Remediated: ts_delay(close, 1 - 2) evaluates to -1 and must fail with ErrNegativeLookback"
    );
    assert!(rep
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrNegativeLookback));

    // Also test constant variable lag
    let var_expr = "lag = -5; group_neutralize(rank(ts_delay(close, lag)), densify(subindustry))";
    let rep_var = validator.validate(var_expr);
    assert!(!rep_var.is_valid);
    assert!(rep_var
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrNegativeLookback));
}

#[test]
fn test_challenger_reproduce_missing_arity_checks() {
    let validator = SchemaValidator::new();

    // Vulnerability 4 Remediated:
    // Function arity checks strictly enforced.
    let rep_ts_delay =
        validator.validate("group_neutralize(rank(ts_delay()), densify(subindustry))");
    let rep_rank = validator.validate("group_neutralize(rank(), densify(subindustry))");
    let rep_densify = validator.validate("group_neutralize(rank(close), densify())");

    assert!(
        !rep_ts_delay.is_valid,
        "Remediated: ts_delay() with 0 args must fail"
    );
    assert!(rep_ts_delay
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));

    assert!(
        !rep_rank.is_valid,
        "Remediated: rank() with 0 args must fail"
    );
    assert!(rep_rank
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));

    assert!(
        !rep_densify.is_valid,
        "Remediated: densify() with 0 args must fail"
    );
    assert!(rep_densify
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));
}

#[test]
fn test_challenger_reproduce_unknown_field_emitted_as_warning_not_error() {
    let validator = SchemaValidator::new();

    // Vulnerability 5 Remediated:
    // Unknown field emits ErrUnknownField and fails validation.
    let expr = "group_neutralize(rank(completely_invented_fake_field_12345), densify(subindustry))";
    let rep = validator.validate(expr);

    assert!(
        !rep.is_valid,
        "Remediated: unknown field must fail validation with ErrUnknownField"
    );
    assert!(rep
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrUnknownField));
}

#[test]
fn test_challenger_reproduce_empty_and_assignment_only_scripts_pass() {
    let validator = SchemaValidator::new();

    // Vulnerability 6 Remediated:
    // Empty strings, comments, and assignment-only scripts must fail validation.
    let rep_empty = validator.validate("");
    let rep_comment = validator.validate("# Just a comment");
    let rep_assign_only = validator.validate("x = 1; y = 2;");

    assert!(
        !rep_empty.is_valid,
        "Remediated: empty expression must fail"
    );
    assert!(rep_empty
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));

    assert!(
        !rep_comment.is_valid,
        "Remediated: comment-only string must fail"
    );
    assert!(rep_comment
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));

    assert!(
        !rep_assign_only.is_valid,
        "Remediated: assignment-only script must fail"
    );
    assert!(rep_assign_only
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));
}

// =============================================================================
// ITERATION 3 ADVERSARIAL CHALLENGER REMEDIATION TESTS
// =============================================================================

#[test]
fn test_challenger_reproduce_ts_lookback_fail_closed_enforcement() {
    let validator = SchemaValidator::new();

    // 1. Direct ts_delay with power operator evaluating to negative lag (-8) fails closed
    let rep_pow_neg = validator.validate("ts_delay(close, 0 - (2 ^ 3))");
    assert!(
        !rep_pow_neg.is_valid,
        "ts_delay with 0 - (2 ^ 3) = -8 must fail closed"
    );
    assert!(
        rep_pow_neg
            .errors
            .iter()
            .any(|e| e.code == DiagnosticCode::ErrNegativeLookback),
        "Expected ErrNegativeLookback, found: {:?}",
        rep_pow_neg.errors
    );

    // 2. Direct ts_delay with power operator evaluating to zero lag fails closed
    let rep_pow_zero = validator.validate("ts_delay(close, 0 * (2 ^ 1))");
    assert!(
        !rep_pow_zero.is_valid,
        "ts_delay with 0 * (2 ^ 1) = 0 must fail closed"
    );
    assert!(rep_pow_zero
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrNegativeLookback));

    // 3. Wrapped ts_delay with power negative lookback in group_neutralize fails closed
    let rep_wrapped_neg = validator
        .validate("group_neutralize(rank(ts_delay(close, 0 - (2 ^ 3))), densify(subindustry))");
    assert!(
        !rep_wrapped_neg.is_valid,
        "Wrapped look-ahead ts_delay must fail closed"
    );
    assert!(rep_wrapped_neg
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrNegativeLookback));

    // 4. Lookback with division by zero fails closed (cannot be statically resolved)
    let rep_div_zero = validator.validate("ts_decay_linear(close, 10 / 0)");
    assert!(
        !rep_div_zero.is_valid,
        "Unresolvable division by zero lag must fail closed"
    );
    assert!(rep_div_zero
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrNegativeLookback));

    // 5. Lookback with non-scalar variable fails closed
    let rep_matrix_lag = validator.validate("ts_delay(close, open)");
    assert!(
        !rep_matrix_lag.is_valid,
        "Matrix field passed as lag must fail closed"
    );
    assert!(rep_matrix_lag
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrNegativeLookback));

    // 6. Valid ts_delay with positive integer lag passes cleanly
    let rep_valid_direct = validator.validate("ts_delay(close, 5)");
    assert!(
        rep_valid_direct.is_valid,
        "Valid ts_delay(close, 5) must pass: {:?}",
        rep_valid_direct.errors
    );
    assert_eq!(rep_valid_direct.errors.len(), 0);

    let rep_valid_wrapped =
        validator.validate("group_neutralize(rank(ts_delay(close, 5)), densify(subindustry))");
    assert!(
        rep_valid_wrapped.is_valid,
        "Valid wrapped ts_delay must pass: {:?}",
        rep_valid_wrapped.errors
    );
    assert_eq!(rep_valid_wrapped.errors.len(), 0);
}

#[test]
fn test_challenger_reproduce_densify_group_type_verification() {
    let validator = SchemaValidator::new();

    // 1. densify(close) with continuous MATRIX fails closed with ErrGroupTypeMismatch
    let rep_matrix = validator.validate("densify(close)");
    assert!(
        !rep_matrix.is_valid,
        "densify(close) matrix must fail closed"
    );
    assert!(
        rep_matrix
            .errors
            .iter()
            .any(|e| e.code == DiagnosticCode::ErrGroupTypeMismatch),
        "Expected ErrGroupTypeMismatch, found: {:?}",
        rep_matrix.errors
    );

    // 2. Wrapped densify(close) inside group_neutralize fails closed
    let rep_wrapped_matrix = validator.validate("group_neutralize(rank(close), densify(close))");
    assert!(
        !rep_wrapped_matrix.is_valid,
        "group_neutralize with densify(close) must fail closed"
    );
    assert!(rep_wrapped_matrix
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrGroupTypeMismatch));

    // 3. densify(nws18_ssc) with 3D VECTOR field fails closed with ErrVecTypeMismatch and ErrGroupTypeMismatch
    let rep_vec = validator.validate("densify(nws18_ssc)");
    assert!(
        !rep_vec.is_valid,
        "densify(nws18_ssc) vector must fail closed"
    );
    assert!(rep_vec
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrVecTypeMismatch));
    assert!(rep_vec
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrGroupTypeMismatch));

    let rep_wrapped_vec = validator.validate("group_neutralize(rank(close), densify(nws18_ssc))");
    assert!(
        !rep_wrapped_vec.is_valid,
        "group_neutralize with densify(nws18_ssc) must fail closed"
    );
    assert!(rep_wrapped_vec
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrVecTypeMismatch));

    // 4. densify(fscore_growth) with unbackfilled low-coverage field propagates low_coverage_origins and fails
    let rep_sparse = validator.validate("group_neutralize(rank(close), densify(fscore_growth))");
    assert!(
        !rep_sparse.is_valid,
        "densify(fscore_growth) must not erase missing backfill error"
    );
    assert!(
        rep_sparse
            .errors
            .iter()
            .any(|e| e.code == DiagnosticCode::ErrMissingBackfill),
        "Missing backfill error must be preserved, found: {:?}",
        rep_sparse.errors
    );
    assert!(rep_sparse
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrGroupTypeMismatch));

    // 5. densify with scalar literal fails closed
    let rep_scalar = validator.validate("densify(100.0)");
    assert!(
        !rep_scalar.is_valid,
        "densify(100.0) scalar literal must fail closed"
    );
    assert!(rep_scalar
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrGroupTypeMismatch));

    // 6. Bare densify(subindustry) fails terminal matrix check (must evaluate to 2D numerical matrix)
    let rep_bare_densify = validator.validate("densify(subindustry)");
    assert!(
        !rep_bare_densify.is_valid,
        "Bare densify(subindustry) must fail terminal matrix check: {:?}",
        rep_bare_densify.errors
    );
    assert!(rep_bare_densify
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));

    let rep_valid_wrapped =
        validator.validate("group_neutralize(rank(close), densify(subindustry))");
    assert!(
        rep_valid_wrapped.is_valid,
        "Valid wrapped densify(subindustry) must pass: {:?}",
        rep_valid_wrapped.errors
    );
    assert_eq!(rep_valid_wrapped.errors.len(), 0);

    // 7. Other valid grouping fields (industry, sector) pass cleanly
    let rep_sector = validator.validate("group_neutralize(rank(close), densify(sector))");
    assert!(
        rep_sector.is_valid,
        "Valid densify(sector) must pass: {:?}",
        rep_sector.errors
    );

    let rep_industry = validator.validate("group_neutralize(rank(close), densify(industry))");
    assert!(
        rep_industry.is_valid,
        "Valid densify(industry) must pass: {:?}",
        rep_industry.errors
    );
}

#[test]
fn test_challenger_reproduce_vec_input_type_verification() {
    let validator = SchemaValidator::new();

    // 1. Direct vec_avg(close) on MATRIX field fails closed with ErrVecTypeMismatch
    let rep_avg_close = validator.validate("vec_avg(close)");
    assert!(!rep_avg_close.is_valid, "vec_avg(close) must fail closed");
    assert!(
        rep_avg_close
            .errors
            .iter()
            .any(|e| e.code == DiagnosticCode::ErrVecTypeMismatch),
        "Expected ErrVecTypeMismatch, found: {:?}",
        rep_avg_close.errors
    );

    // 2. Direct vec_sum(close) on MATRIX field fails closed with ErrVecTypeMismatch
    let rep_sum_close = validator.validate("vec_sum(close)");
    assert!(!rep_sum_close.is_valid, "vec_sum(close) must fail closed");
    assert!(rep_sum_close
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrVecTypeMismatch));

    // 3. Wrapped vec_avg(close) inside group_neutralize fails closed
    let rep_wrapped =
        validator.validate("group_neutralize(rank(vec_avg(close)), densify(subindustry))");
    assert!(
        !rep_wrapped.is_valid,
        "group_neutralize with vec_avg(close) must fail closed"
    );
    assert!(rep_wrapped
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrVecTypeMismatch));

    // 4. vec_avg on scalar literal fails closed
    let rep_scalar = validator.validate("vec_avg(1.0)");
    assert!(!rep_scalar.is_valid, "vec_avg(1.0) must fail closed");
    assert!(rep_scalar
        .errors
        .iter()
        .any(|e| e.code == DiagnosticCode::ErrVecTypeMismatch));

    // 5. Valid vec_avg(nws18_ssc) on VECTOR field passes cleanly
    let rep_valid_direct = validator.validate("vec_avg(nws18_ssc)");
    assert!(
        rep_valid_direct.is_valid,
        "Valid vec_avg(nws18_ssc) must pass: {:?}",
        rep_valid_direct.errors
    );
    assert_eq!(rep_valid_direct.errors.len(), 0);

    // 6. Valid vec_sum(nws18_ssc) on VECTOR field passes cleanly
    let rep_valid_sum = validator.validate("vec_sum(nws18_ssc)");
    assert!(
        rep_valid_sum.is_valid,
        "Valid vec_sum(nws18_ssc) must pass: {:?}",
        rep_valid_sum.errors
    );
    assert_eq!(rep_valid_sum.errors.len(), 0);

    // 7. Valid wrapped vec_avg(nws18_ssc) in group_neutralize passes cleanly
    let rep_valid_wrapped =
        validator.validate("group_neutralize(rank(vec_avg(nws18_ssc)), densify(subindustry))");
    assert!(
        rep_valid_wrapped.is_valid,
        "Valid wrapped vec_avg must pass: {:?}",
        rep_valid_wrapped.errors
    );
    assert_eq!(rep_valid_wrapped.errors.len(), 0);
}

#[test]
fn test_remediation_deeply_nested_recursion_limit() {
    let validator = SchemaValidator::new();

    // 1. 200 parens fails closed gracefully with recursion depth error
    let deep_200 = format!("{}close{}", "(".repeat(200), ")".repeat(200));
    let rep_200 = validator.validate(&deep_200);
    assert!(
        !rep_200.is_valid,
        "200 nested parens must fail closed gracefully without stack overflow"
    );
    assert!(rep_200.errors.iter().any(|e| {
        e.code == DiagnosticCode::ErrSyntaxMalformed && e.message.contains("recursion depth")
    }));

    // 2. 1500 parens fails closed gracefully without stack overflow abort (SIGABRT)
    let deep_1500 = format!("{}close{}", "(".repeat(1500), ")".repeat(1500));
    let rep_1500 = validator.validate(&deep_1500);
    assert!(
        !rep_1500.is_valid,
        "1500 nested parens must fail closed without SIGABRT"
    );
    assert!(rep_1500.errors.iter().any(|e| {
        e.code == DiagnosticCode::ErrSyntaxMalformed && e.message.contains("recursion depth")
    }));

    // 3. Deeply nested unary operators also bound depth
    let deep_unary = format!("{}close", "-".repeat(200));
    let rep_unary = validator.validate(&deep_unary);
    assert!(
        !rep_unary.is_valid,
        "200 nested unaries must fail closed gracefully"
    );
    assert!(rep_unary.errors.iter().any(|e| {
        e.code == DiagnosticCode::ErrSyntaxMalformed && e.message.contains("recursion depth")
    }));
}

#[test]
fn test_remediation_tokenizer_illegal_tokens() {
    let validator = SchemaValidator::new();

    // 1. Corrupted token '$' fails closed with ErrSyntaxMalformed
    let rep_dollar = validator.validate("close $ 5");
    assert!(!rep_dollar.is_valid, "Expression with '$' must fail closed");
    assert!(rep_dollar.errors.iter().any(|e| {
        e.code == DiagnosticCode::ErrSyntaxMalformed && e.message.contains("Unrecognized token")
    }));

    // 2. Corrupted token '@' fails closed
    let rep_at = validator.validate("rank(close) @ 10");
    assert!(!rep_at.is_valid, "Expression with '@' must fail closed");
    assert!(rep_at.errors.iter().any(|e| {
        e.code == DiagnosticCode::ErrSyntaxMalformed && e.message.contains("Unrecognized token")
    }));

    // 3. Corrupted token '~' fails closed
    let rep_tilde = validator.validate("close ~ open");
    assert!(!rep_tilde.is_valid, "Expression with '~' must fail closed");
    assert!(rep_tilde.errors.iter().any(|e| {
        e.code == DiagnosticCode::ErrSyntaxMalformed && e.message.contains("Unrecognized token")
    }));
}

#[test]
fn test_remediation_unseparated_statements() {
    let validator = SchemaValidator::new();

    // 1. Missing semicolons between consecutive statements fails closed
    let rep_unsep = validator.validate("close open volume");
    assert!(
        !rep_unsep.is_valid,
        "'close open volume' without semicolons must fail closed"
    );
    assert!(rep_unsep.errors.iter().any(|e| {
        e.code == DiagnosticCode::ErrSyntaxMalformed
            && e.message.contains("separated by semicolons")
    }));

    // 2. Assignment without semicolon preceding statement fails closed
    let rep_unsep_assign = validator
        .validate("x = rank(close) y = rank(open); group_neutralize(x, densify(subindustry))");
    assert!(
        !rep_unsep_assign.is_valid,
        "Unseparated assignment must fail closed"
    );
    assert!(rep_unsep_assign.errors.iter().any(|e| {
        e.code == DiagnosticCode::ErrSyntaxMalformed
            && e.message.contains("separated by semicolons")
    }));

    // 3. Properly semicolon-separated multi-statement script passes
    let rep_sep = validator
        .validate("x = rank(close); y = rank(open); group_neutralize(x - y, densify(subindustry))");
    assert!(
        rep_sep.is_valid,
        "Properly separated statements must pass: {:?}",
        rep_sep.errors
    );
    assert_eq!(rep_sep.errors.len(), 0);
}

#[test]
fn test_remediation_terminal_non_matrix_rejection() {
    let validator = SchemaValidator::new();

    // 1. Constant scalar 42 fails terminal matrix check
    let rep_42 = validator.validate("42");
    assert!(
        !rep_42.is_valid,
        "Terminal scalar constant 42 must fail validation"
    );
    assert!(rep_42
        .errors
        .iter()
        .any(|e| { e.code == DiagnosticCode::ErrSyntaxMalformed && e.message.contains("SCALAR") }));

    // 2. String literal fails terminal matrix check
    let rep_str = validator.validate("\"hello\"");
    assert!(
        !rep_str.is_valid,
        "Terminal string literal must fail validation"
    );
    assert!(rep_str
        .errors
        .iter()
        .any(|e| { e.code == DiagnosticCode::ErrSyntaxMalformed && e.message.contains("SCALAR") }));

    // 3. Categorical group variable fails terminal matrix check
    let rep_grp = validator.validate("subindustry");
    assert!(
        !rep_grp.is_valid,
        "Terminal categorical group 'subindustry' must fail validation"
    );
    assert!(rep_grp.errors.iter().any(|e| {
        e.code == DiagnosticCode::ErrSyntaxMalformed && e.message.contains("categorical group")
    }));

    // 4. Densified group fails terminal matrix check
    let rep_dens = validator.validate("densify(subindustry)");
    assert!(
        !rep_dens.is_valid,
        "Terminal densified group must fail validation"
    );
    assert!(rep_dens.errors.iter().any(|e| {
        e.code == DiagnosticCode::ErrSyntaxMalformed && e.message.contains("categorical group")
    }));

    // 5. Bare unfolded vector field fails terminal matrix check and is not labeled as folded
    let rep_vec = validator.validate("nws18_ssc");
    assert!(
        !rep_vec.is_valid,
        "Terminal bare vector nws18_ssc must fail validation"
    );
    assert!(rep_vec
        .errors
        .iter()
        .any(|e| { e.code == DiagnosticCode::ErrVecTypeMismatch && e.message.contains("VECTOR") }));
    let f_vec = rep_vec
        .extracted_fields
        .iter()
        .find(|f| f.id == "nws18_ssc")
        .expect("nws18_ssc extracted");
    assert!(f_vec.is_vector);
    assert!(!f_vec.folded, "Unfolded vector must have folded == false");

    // 6. Folded vector field passes validation and is labeled as folded
    let rep_folded = validator.validate("vec_avg(nws18_ssc)");
    assert!(
        rep_folded.is_valid,
        "Folded vector vec_avg(nws18_ssc) must pass: {:?}",
        rep_folded.errors
    );
    let f_folded = rep_folded
        .extracted_fields
        .iter()
        .find(|f| f.id == "nws18_ssc")
        .expect("nws18_ssc extracted");
    assert!(f_folded.is_vector);
    assert!(f_folded.folded, "Folded vector must have folded == true");
}
