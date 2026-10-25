# Dispatch Assignment — worker_m5_remediation

## Objective
Remediate the 5 adversarial failure modes and edge cases identified by `challenger_m5_1` during Milestone M5 Tier 5 validation, add comprehensive tests for all 5 cases, and verify zero regressions.

## Challenger Handoff Reference
Read the full adversarial findings in:
`/Users/phamtoan/Developer/alphafind/.agents/teamwork/challenger_m5_1/handoff.md`

## MANDATORY INTEGRITY WARNING
> DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_preview_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## Detailed Tasks to Implement
1. **Parser Recursion Depth Limit (`src/schema_validator.rs`)**:
   - Add `depth: usize` and `const MAX_RECURSION_DEPTH: usize = 128;` to `Parser`.
   - Increment `depth` when entering nested expressions / function calls and decrement on exit.
   - If `depth >= MAX_RECURSION_DEPTH`, push an AST syntax error (`DiagnosticCode::ErrSyntaxMalformed`) and fail closed. Verify that 1500 parens (`'(' * 1500 + 'close' + ')' * 1500`) fails closed gracefully without stack overflow (`SIGABRT`).
2. **Tokenizer & Multi-Statement Syntax Enforcement (`src/schema_validator.rs`)**:
   - In `tokenize`: Do NOT silently skip unrecognized characters (like `$`, `@`). Push `DiagnosticCode::ErrSyntaxMalformed` ("Unrecognized token: ...") so expressions like `close $ 5` fail closed.
   - In `Parser::parse_statements`: Require explicit semicolons `;` between consecutive statements. If a statement concludes without a semicolon and more tokens remain, push `DiagnosticCode::ErrSyntaxMalformed` ("Consecutive statements must be separated by semicolons").
3. **Enforce Matrix Type on Terminal Alpha Expressions (`src/schema_validator.rs`)**:
   - In `SchemaValidator::validate`: Check the evaluated type of the terminal expression. In WorldQuant BRAIN, an alpha expression MUST evaluate to a 2D cross-sectional matrix tensor (`FastExprType::Matrix` or `FastExprType::BooleanMatrix`).
   - If the terminal expression evaluates to `Scalar`, `StringLit`, `Group`, `DensifiedGroup`, or raw unfolded `Vector`, reject it with a structured error (`DiagnosticCode::ErrSyntaxMalformed` or `DiagnosticCode::ErrVecTypeMismatch`).
   - In `print_diagnostics`: Only display `[VECTOR (folded)]` if the vector field is actually folded; otherwise display `[VECTOR]`.
4. **Fix Expression Generation Count (`src/expression_builder.rs`)**:
   - In `generate_batch`: If `count == 0`, immediately return an empty vector `Vec::new()`. Fix loop bounds so requesting `count = 0` yields exactly 0 candidates.
5. **Fail-Fast on Unknown Datasets & Pillars (`src/commands/research.rs`)**:
   - In `cmd_generate`: If user supplies an unknown `--dataset` or `--pillar`, fail fast with an error (`anyhow::bail!("Unknown dataset '{}' ...")`). Do not silently fall back to `model51` or all pillars.
6. **Documentation & Typos**:
   - In `docs/FACTOR_TAXONOMY_ALTERNATIVE_DATA.md` line 417: Change `+ \text{PV}(\text{Div})` to `- \text{PV}(\text{Div})`.
   - In `TEST_READY.md` line 146: Change `SchemaValidator::auto_repair_expression` to `ExpressionBuilder::auto_repair`.
7. **Comprehensive Tests**:
   - Add test cases in `tests/stress_schema_validator_tests.rs` and `tests/stress_expression_builder_tests.rs` covering:
     * Stack overflow prevention on deep nesting (e.g. 200 parens)
     * Rejection of dropped characters (`close $ 5`)
     * Rejection of unseparated statements (`close open volume`)
     * Rejection of terminal non-matrix expressions (`42`, `"hello"`, `subindustry`, `densify(subindustry)`, `nws18_ssc`)
     * `generate -n 0` returning 0 expressions
     * Rejection of unknown datasets and pillars
8. **Verification & Cleanliness**:
   - Run `cargo test -- --test-threads=1` (ensure all tests pass, including new ones).
   - Run `cargo clippy --tests -- -D warnings`.
   - Run `cargo fmt --check`.
   - Run `bash scripts/sanitize_check.sh`.
   - Update `TEST_READY.md` with updated test counts.

## Deliverable
Write your handoff report to:
`/Users/phamtoan/Developer/alphafind/.agents/teamwork/worker_m5_remediation/handoff.md`
Send completion message to parent conversation ID `8b020a34-8c2d-4255-98f9-544909ff7652`.


## 2026-10-24T23:14:12Z
You are worker_m5_remediation. Your working directory is /Users/phamtoan/Developer/alphafind/.agents/teamwork/worker_m5_remediation.
Read your full task instructions at /Users/phamtoan/Developer/alphafind/.agents/teamwork/worker_m5_remediation/DISPATCH.md.
MANDATORY: Read /Users/phamtoan/Developer/alphafind/.agents/teamwork/ORIGINAL_REQUEST.md, /Users/phamtoan/Developer/alphafind/PROJECT.md, /Users/phamtoan/Developer/alphafind/AGENTS.md, and /Users/phamtoan/Developer/alphafind/.agents/teamwork/challenger_m5_1/handoff.md before starting work.
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_preview_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.
Implement all 6 remediation items in src/schema_validator.rs, src/expression_builder.rs, src/commands/research.rs, docs, and tests.
Verify with cargo test -- --test-threads=1, cargo clippy --tests -- -D warnings, cargo fmt --check, and bash scripts/sanitize_check.sh.
Update TEST_READY.md.
Write your handoff report to /Users/phamtoan/Developer/alphafind/.agents/teamwork/worker_m5_remediation/handoff.md.
Send your completion message to parent conversation ID 8b020a34-8c2d-4255-98f9-544909ff7652 via send_message.
