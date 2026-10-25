# Remediation Specification: Schema Validator Engine & Metadata Tool
**Milestone**: M2 (Field Metadata Extraction & Schema Validator) — Iteration 2  
**Target Files**: `src/schema_validator.rs`, `scratch/metadata_tool.py`, `tests/stress_schema_validator_tests.rs`  
**Author**: Schema Validator Remediation Strategist (`explorer_m2_remediation`)  
**Status**: APPROVED REMEDIATION PLAN FOR WORKER  

---

## 1. Executive Summary

During Milestone M2 Iteration 1, adversarial testing and software review by **Challenger 1** (`challenger_m2_1`) and **Reviewer 2** (`reviewer_m2_2`) revealed critical fail-open vulnerabilities in `src/schema_validator.rs` and `scratch/metadata_tool.py`:
1. **Backfill Poisoning Bypass**: Combining backfilled and unbackfilled sparse fields in binary expressions shields unbackfilled fields from validation failure.
2. **Lookback Parameter Bypass**: Non-positive lookbacks wrapped in binary arithmetic (e.g. `1 - 2`) or variables silently bypass the look-ahead bias guard.
3. **Price Normalization False Positives**: Subtracting `1.0` or multiplying constants marks normalized price ratios as unnormalized price levels, emitting false warnings.
4. **Python Scratch Tool Regex Breakdown**: Greedy regex in `scratch/metadata_tool.py` splits nested function commas inside `group_neutralize(...)`.
5. **Missing Function Arity Checks & Script Handling**: 0-argument function calls (`ts_delay()`, `rank()`, `densify()`) and empty or assignment-only scripts evaluate as valid alphas.
6. **Auxiliary Findings**: Unknown fields fail open as warnings rather than errors; unfolded vectors are accepted directly inside `ts_backfill(...)`.

This document specifies the exact root causes, line-by-line patch instructions, and test updates for the Worker (`worker_m2_1`).

---

## 2. Root Cause Analysis & Architectural Decisions

### Issue 1: Backfill Poisoning Bypass in Binary Expressions
- **Locations**: `src/schema_validator.rs:1520` and `src/schema_validator.rs:1282`
- **Root Cause**:
  1. In `eval_expr` for `AstExpr::Binary`, `is_backfilled` was calculated as `l.is_backfilled || r.is_backfilled`. If one operand was backfilled, `is_backfilled` became `true` for the combined tree.
  2. In the statement check (line 1282), the code conditioned error emission on `!final_meta.low_coverage_origins.is_empty() && !final_meta.is_backfilled`. Because `is_backfilled` was poisoned to `true`, the error check was skipped, allowing unbackfilled sparse fields to pass.
- **Architectural Decision**:
  - `low_coverage_origins` is the authoritative multi-origin accumulator. When `ts_backfill(sig, d >= 5)` is applied, it clears `low_coverage_origins = Vec::new()`. Therefore, if `!final_meta.low_coverage_origins.is_empty()` at the root, sparse fields remain unbackfilled regardless of other subtrees. Remove `&& !final_meta.is_backfilled` from line 1282.
  - In `AstExpr::Binary`, propagate backfill status strictly via conjunction: `l.is_backfilled && r.is_backfilled`.
  - In `ts_backfill`, reject unfolded `FastExprType::Vector` operands with `ErrVecTypeMismatch`.

### Issue 2: Lookback Parameter Bypass via Binary Expressions & Variables
- **Locations**: `src/schema_validator.rs:1618-1624` and `src/schema_validator.rs:1684-1698`
- **Root Cause**:
  `extract_literal_number` only matched `AstExpr::Literal(n)` and `AstExpr::Unary { op: "-", expr }`. Binary expressions (`1 - 2`) and variables (`lag = -5`) returned `None`. Because line 1685 conditioned on `if let Some(lag) = extract_literal_number(...)`, `None` caused silent pass-through of invalid lookbacks.
- **Architectural Decision**:
  - Add `pub scalar_value: Option<f64>` to `ValueMeta` so scalar evaluation results propagate through expressions and assignments in `symbol_table`.
  - Extend `extract_literal_number(expr: &AstExpr, symbol_table: &HashMap<String, ValueMeta>) -> Option<f64>` to recursively evaluate:
    - `AstExpr::Literal(n)` => `Some(*n)`
    - `AstExpr::Unary` (+ and -)
    - `AstExpr::Binary` (+, -, *, /)
    - `AstExpr::Variable(var)` looked up in `symbol_table`
  - In `ts_*` and `ts_backfill`, if `extract_literal_number` returns `Some(lag)` and `lag <= 0.0`, emit `ErrNegativeLookback`. If `extract_literal_number` returns `None` and the argument is not a scalar, emit `ErrNegativeLookback` / `ErrSyntaxMalformed`.

### Issue 3: Price Normalization False Positives
- **Locations**: `src/schema_validator.rs:1329`, `1336`, `1505-1513`
- **Root Cause**:
  `AstExpr::Literal` and `AstExpr::StringLit` initialized `is_normalized_price: false`. When subtracting `1.0` from a normalized price ratio `call_breakeven_30 / close`, the binary difference `l.is_normalized_price && r.is_normalized_price` evaluated to `true && false = false`, erroneously classifying the ratio as an unnormalized price level.
- **Architectural Decision**:
  Literal scalars and strings are dimensionless constants. Initialize `is_normalized_price: true` for `AstExpr::Literal`, `AstExpr::StringLit`, and `ValueMeta::default()`. This preserves normalization when subtracting `1.0` (`true && true = true`), while raw prices still fail (`false && true = false`).

### Issue 4: Python Scratch Tool Regex Comma Splitting
- **Location**: `scratch/metadata_tool.py:263-268`
- **Root Cause**:
  `re.findall(r"group_neutralize\s*\([^,]+,\s*([^)]+)\)", expr)` split on the first comma inside `group_neutralize(...)`. For `group_neutralize(ts_decay_linear(rank(close), 5), densify(subindustry))`, it matched `5` as the grouping argument.
- **Architectural Decision**:
  Implement a parenthesis-aware argument extractor `extract_call_args(text, func_name)` that counts nesting depth `(` and `)` before splitting on top-level commas.

### Issue 5: Missing Function Arity Validation & Script Structure
- **Locations**: `src/schema_validator.rs:1240-1297`, `1534`, `1559`, `1571`, `1628`, `1668`, `1710`, `1740`
- **Root Cause**:
  - `validate("")` and `validate("x = 1; y = 2;")` produced 0 errors and evaluated to `is_valid: true`.
  - Functions (`densify`, `rank`, `ts_delay`, `vec_avg`) accepted 0 arguments without failing closed.
  - Unknown field identifiers pushed `ValidationWarning` with `WarnHorizonSyntax` instead of `ValidationError` with `ErrUnknownField`.
- **Architectural Decision**:
  - `validate()` must reject empty statement lists with `ErrSyntaxMalformed`.
  - `validate()` must require that the final statement is an `AstStatement::Expression`.
  - All built-in operators must enforce exact arity:
    - `densify`: exactly 1 argument
    - `rank`: exactly 1 argument
    - `vec_*`: exactly 1 argument
    - `group_neutralize`: exactly 2 arguments
    - `ts_backfill`: exactly 2 arguments
    - `ts_*`: at least 2 arguments
    - Default function handler: at least 1 argument
  - Unknown fields not found in catalog or keyword lists must emit `ValidationError { code: DiagnosticCode::ErrUnknownField, ... }` and return `FastExprType::TypeError`.

---

## 3. Patch Specifications for `src/schema_validator.rs`

### Patch 1: `ValueMeta` Struct & Default Implementation
**Target Lines**: 1124–1142
```rust
<<<< EXISTING
#[derive(Debug, Clone, PartialEq)]
pub struct ValueMeta {
    pub expr_type: FastExprType,
    pub is_backfilled: bool,
    pub backfill_window: Option<i64>,
    pub low_coverage_origins: Vec<String>,
    pub is_normalized_price: bool,
}

impl Default for ValueMeta {
    fn default() -> Self {
        Self {
            expr_type: FastExprType::Scalar,
            is_backfilled: false,
            backfill_window: None,
            low_coverage_origins: Vec::new(),
            is_normalized_price: false,
        }
    }
}
==== REPLACEMENT
#[derive(Debug, Clone, PartialEq)]
pub struct ValueMeta {
    pub expr_type: FastExprType,
    pub is_backfilled: bool,
    pub backfill_window: Option<i64>,
    pub low_coverage_origins: Vec<String>,
    pub is_normalized_price: bool,
    pub scalar_value: Option<f64>,
}

impl Default for ValueMeta {
    fn default() -> Self {
        Self {
            expr_type: FastExprType::Scalar,
            is_backfilled: false,
            backfill_window: None,
            low_coverage_origins: Vec::new(),
            is_normalized_price: true,
            scalar_value: None,
        }
    }
}
>>>>
```

### Patch 2: Script Statement Validation (Empty & Assignment-Only Scripts)
**Target Lines**: 1258–1260
```rust
<<<< EXISTING
        // 3. Traverse statements and validate types
        for stmt in &statements {
==== REPLACEMENT
        // 3. Validate script structure: non-empty statements and terminal expression requirement
        if statements.is_empty() {
            errors.push(ValidationError {
                code: DiagnosticCode::ErrSyntaxMalformed,
                field_id: None,
                message: "Expression is empty; expected a valid FastExpr formula".to_string(),
                suggestion: Some("Provide a valid FastExpr formula, e.g. group_neutralize(rank(close), densify(subindustry))".to_string()),
            });
            return ValidationReport {
                is_valid: false,
                expression: expression.to_string(),
                errors,
                warnings,
                extracted_fields: Vec::new(),
            };
        }

        let has_terminal_expr = matches!(statements.last(), Some(AstStatement::Expression(_)));
        if !has_terminal_expr {
            errors.push(ValidationError {
                code: DiagnosticCode::ErrSyntaxMalformed,
                field_id: None,
                message: "Script must end with a terminal alpha expression, not an assignment".to_string(),
                suggestion: Some("Add a terminal expression after assignments, e.g., group_neutralize(rank(signal), densify(subindustry))".to_string()),
            });
        }

        // 4. Traverse statements and validate types
        for stmt in &statements {
>>>>
```

### Patch 3: Backfill Origin Root Condition
**Target Lines**: 1281–1295
```rust
<<<< EXISTING
                    // Final expression check: any low-coverage field without backfill
                    if !final_meta.low_coverage_origins.is_empty() && !final_meta.is_backfilled {
                        for fid in &final_meta.low_coverage_origins {
                            errors.push(ValidationError {
                                code: DiagnosticCode::ErrMissingBackfill,
                                field_id: Some(fid.clone()),
                                message: format!(
                                    "Field '{}' has low cross-sectional coverage (<50%) and lacks ancestral ts_backfill(..., d >= 5)",
                                    fid
                                ),
                                suggestion: Some(format!("Wrap '{}' with ts_backfill({}, 15)", fid, fid)),
                            });
                        }
                    }
==== REPLACEMENT
                    // Final expression check: any low-coverage field without backfill
                    if !final_meta.low_coverage_origins.is_empty() {
                        for fid in &final_meta.low_coverage_origins {
                            errors.push(ValidationError {
                                code: DiagnosticCode::ErrMissingBackfill,
                                field_id: Some(fid.clone()),
                                message: format!(
                                    "Field '{}' has low cross-sectional coverage (<50%) and lacks ancestral ts_backfill(..., d >= 5)",
                                    fid
                                ),
                                suggestion: Some(format!("Wrap '{}' with ts_backfill({}, 15)", fid, fid)),
                            });
                        }
                    }
>>>>
```

### Patch 4: Literal & StringLit Initialization
**Target Lines**: 1324–1337
```rust
<<<< EXISTING
            AstExpr::Literal(_) => ValueMeta {
                expr_type: FastExprType::Scalar,
                is_backfilled: false,
                backfill_window: None,
                low_coverage_origins: Vec::new(),
                is_normalized_price: false,
            },
            AstExpr::StringLit(_) => ValueMeta {
                expr_type: FastExprType::Scalar,
                is_backfilled: false,
                backfill_window: None,
                low_coverage_origins: Vec::new(),
                is_normalized_price: false,
            },
==== REPLACEMENT
            AstExpr::Literal(n) => ValueMeta {
                expr_type: FastExprType::Scalar,
                is_backfilled: false,
                backfill_window: None,
                low_coverage_origins: Vec::new(),
                is_normalized_price: true,
                scalar_value: Some(*n),
            },
            AstExpr::StringLit(_) => ValueMeta {
                expr_type: FastExprType::Scalar,
                is_backfilled: false,
                backfill_window: None,
                low_coverage_origins: Vec::new(),
                is_normalized_price: true,
                scalar_value: None,
            },
>>>>
```

### Patch 5: Unknown Field Fail-Closed Diagnostic Inversion Fix
**Target Lines**: 1447–1464
```rust
<<<< EXISTING
                // If not found in catalog, treat as unknown field error unless recognized market token
                if !matches!(name_lower.as_str(), "true" | "false" | "nan") {
                    warnings.push(ValidationWarning {
                        code: DiagnosticCode::WarnHorizonSyntax,
                        field_id: Some(name.clone()),
                        message: format!("Field '{}' not found in alternative data field catalog", name),
                        suggestion: Some("Verify field name against BRAIN /data-fields".to_string()),
                    });
                }

                ValueMeta {
                    expr_type: FastExprType::Matrix,
                    is_backfilled: true,
                    backfill_window: None,
                    low_coverage_origins: Vec::new(),
                    is_normalized_price: true,
                }
==== REPLACEMENT
                // If not found in catalog, check recognized constants or fail closed with ErrUnknownField
                if matches!(name_lower.as_str(), "true" | "false") {
                    return ValueMeta {
                        expr_type: FastExprType::BooleanMatrix,
                        is_backfilled: true,
                        backfill_window: None,
                        low_coverage_origins: Vec::new(),
                        is_normalized_price: true,
                        scalar_value: None,
                    };
                }
                if name_lower == "nan" {
                    return ValueMeta {
                        expr_type: FastExprType::Scalar,
                        is_backfilled: true,
                        backfill_window: None,
                        low_coverage_origins: Vec::new(),
                        is_normalized_price: true,
                        scalar_value: None,
                    };
                }

                errors.push(ValidationError {
                    code: DiagnosticCode::ErrUnknownField,
                    field_id: Some(name.clone()),
                    message: format!("Field '{}' not found in alternative data field catalog", name),
                    suggestion: Some("Verify field name against BRAIN /data-fields".to_string()),
                });

                ValueMeta {
                    expr_type: FastExprType::TypeError,
                    ..Default::default()
                }
>>>>
```

### Patch 6: Unary Operator Scalar Value Propagation
**Target Lines**: 1465–1483
```rust
<<<< EXISTING
            AstExpr::Unary { op, expr } => {
                let inner = self.eval_expr(expr, symbol_table, errors, warnings, extracted_fields);
                if inner.expr_type == FastExprType::Vector {
                    errors.push(ValidationError {
                        code: DiagnosticCode::ErrVecTypeMismatch,
                        field_id: None,
                        message: format!(
                            "Unary operator '{}' cannot be applied to VECTOR field without folding",
                            op
                        ),
                        suggestion: Some("Wrap vector field in vec_avg(...) or vec_sum(...) first".to_string()),
                    });
                    return ValueMeta {
                        expr_type: FastExprType::TypeError,
                        ..Default::default()
                    };
                }
                inner
            }
==== REPLACEMENT
            AstExpr::Unary { op, expr } => {
                let inner = self.eval_expr(expr, symbol_table, errors, warnings, extracted_fields);
                if inner.expr_type == FastExprType::Vector {
                    errors.push(ValidationError {
                        code: DiagnosticCode::ErrVecTypeMismatch,
                        field_id: None,
                        message: format!(
                            "Unary operator '{}' cannot be applied to VECTOR field without folding",
                            op
                        ),
                        suggestion: Some("Wrap vector field in vec_avg(...) or vec_sum(...) first".to_string()),
                    });
                    return ValueMeta {
                        expr_type: FastExprType::TypeError,
                        ..Default::default()
                    };
                }
                let scalar_val = match op.as_str() {
                    "-" => inner.scalar_value.map(|v| -v),
                    "+" => inner.scalar_value,
                    _ => None,
                };
                ValueMeta {
                    scalar_value: scalar_val,
                    ..inner
                }
            }
>>>>
```

### Patch 7: Binary Expressions: Backfill Conjunction, Price Norm, and Constant Folding
**Target Lines**: 1515–1525
```rust
<<<< EXISTING
                let mut combined_origins = l.low_coverage_origins;
                combined_origins.extend(r.low_coverage_origins);

                ValueMeta {
                    expr_type: FastExprType::Matrix,
                    is_backfilled: l.is_backfilled || r.is_backfilled,
                    backfill_window: l.backfill_window.or(r.backfill_window),
                    low_coverage_origins: combined_origins,
                    is_normalized_price: is_norm,
                }
==== REPLACEMENT
                let mut combined_origins = l.low_coverage_origins;
                combined_origins.extend(r.low_coverage_origins);

                let scalar_val = match (l.scalar_value, r.scalar_value) {
                    (Some(lv), Some(rv)) => match op.as_str() {
                        "+" => Some(lv + rv),
                        "-" => Some(lv - rv),
                        "*" => Some(lv * rv),
                        "/" => if rv.abs() > 1e-9 { Some(lv / rv) } else { None },
                        _ => None,
                    },
                    _ => None,
                };

                ValueMeta {
                    expr_type: if l.expr_type == FastExprType::Scalar && r.expr_type == FastExprType::Scalar {
                        FastExprType::Scalar
                    } else {
                        FastExprType::Matrix
                    },
                    is_backfilled: l.is_backfilled && r.is_backfilled,
                    backfill_window: l.backfill_window.or(r.backfill_window),
                    low_coverage_origins: combined_origins,
                    is_normalized_price: is_norm,
                    scalar_value: scalar_val,
                }
>>>>
```

### Patch 8: Arity & Lookback Validation in Function Calls
**Target Lines**: 1526–1740
Replace the helper and function call checks with strict arity, vector rejection on `ts_backfill`, and symbol-aware lookback resolution:

```rust
<<<< EXISTING
                // Vector folding functions: vec_avg, vec_sum, vec_max, vec_min, vec_count, vec_std
                if matches!(
                    func_lower.as_str(),
                    "vec_avg" | "vec_sum" | "vec_max" | "vec_min" | "vec_count" | "vec_std"
                ) {
                    if args.is_empty() {
                        errors.push(ValidationError {
                            code: DiagnosticCode::ErrSyntaxMalformed,
                            field_id: None,
                            message: format!("'{}' expects 1 argument", func),
                            suggestion: None,
                        });
                        return ValueMeta {
                            expr_type: FastExprType::TypeError,
                            ..Default::default()
                        };
                    }

                    let arg_meta = self.eval_expr(&args[0], symbol_table, errors, warnings, extracted_fields);
                    // Mark as folded to Matrix
                    return ValueMeta {
                        expr_type: FastExprType::Matrix,
                        is_backfilled: arg_meta.is_backfilled,
                        backfill_window: arg_meta.backfill_window,
                        low_coverage_origins: arg_meta.low_coverage_origins,
                        is_normalized_price: true,
                    };
                }

                // densify(group)
                if func_lower == "densify" {
                    return ValueMeta {
                        expr_type: FastExprType::DensifiedGroup,
                        is_backfilled: true,
                        backfill_window: None,
                        low_coverage_origins: Vec::new(),
                        is_normalized_price: true,
                    };
                }

                // group_neutralize(signal, densify(group))
                if func_lower == "group_neutralize" {
                    if args.len() < 2 {
                        errors.push(ValidationError {
                            code: DiagnosticCode::ErrSyntaxMalformed,
                            field_id: None,
                            message: "group_neutralize requires 2 arguments: (signal, densify(group))".to_string(),
                            suggestion: Some("Usage: group_neutralize(signal, densify(subindustry))".to_string()),
                        });
                        return ValueMeta {
                            expr_type: FastExprType::TypeError,
                            ..Default::default()
                        };
                    }

                    let sig_meta = self.eval_expr(&args[0], symbol_table, errors, warnings, extracted_fields);
                    let group_meta = self.eval_expr(&args[1], symbol_table, errors, warnings, extracted_fields);

                    if sig_meta.expr_type == FastExprType::Vector {
                        errors.push(ValidationError {
                            code: DiagnosticCode::ErrVecTypeMismatch,
                            field_id: None,
                            message: "group_neutralize signal argument cannot be of type VECTOR".to_string(),
                            suggestion: Some("Fold vector signal with vec_avg(...) before neutralizing".to_string()),
                        });
                    }

                    // Enforce densify on second argument
                    let is_densified = group_meta.expr_type == FastExprType::DensifiedGroup
                        || matches!(&args[1], AstExpr::Call { func, .. } if func.eq_ignore_ascii_case("densify"));

                    if !is_densified {
                        errors.push(ValidationError {
                            code: DiagnosticCode::ErrUndensifiedGroup,
                            field_id: None,
                            message: "group_neutralize grouping variable must be wrapped in densify(...)".to_string(),
                            suggestion: Some("Wrap grouping parameter in densify(group), e.g., densify(subindustry)".to_string()),
                        });
                    }

                    return ValueMeta {
                        expr_type: FastExprType::Matrix,
                        is_backfilled: sig_meta.is_backfilled,
                        backfill_window: sig_meta.backfill_window,
                        low_coverage_origins: sig_meta.low_coverage_origins,
                        is_normalized_price: true,
                    };
                }

fn extract_literal_number(expr: &AstExpr) -> Option<f64> {
    match expr {
        AstExpr::Literal(n) => Some(*n),
        AstExpr::Unary { op, expr } if op == "-" => extract_literal_number(expr).map(|n| -n),
        _ => None,
    }
}

                // ts_backfill(arg, window)
                if func_lower == "ts_backfill" {
                    if args.len() < 2 {
                        errors.push(ValidationError {
                            code: DiagnosticCode::ErrSyntaxMalformed,
                            field_id: None,
                            message: "ts_backfill requires 2 arguments: (signal, window)".to_string(),
                            suggestion: Some("Usage: ts_backfill(signal, 15)".to_string()),
                        });
                        return ValueMeta {
                            expr_type: FastExprType::TypeError,
                            ..Default::default()
                        };
                    }

                    let sig_meta = self.eval_expr(&args[0], symbol_table, errors, warnings, extracted_fields);
                    let mut window_val = 0i64;
                    if let Some(w) = extract_literal_number(&args[1]) {
                        window_val = w as i64;
                        if window_val < 1 {
                            errors.push(ValidationError {
                                code: DiagnosticCode::ErrNegativeLookback,
                                field_id: None,
                                message: format!(
                                    "ts_backfill received non-positive lookback window ({})",
                                    window_val
                                ),
                                suggestion: Some("Lookback window must be an integer >= 1".to_string()),
                            });
                        }
                    }

                    return ValueMeta {
                        expr_type: sig_meta.expr_type,
                        is_backfilled: window_val >= 5,
                        backfill_window: Some(window_val),
                        low_coverage_origins: if window_val >= 5 { Vec::new() } else { sig_meta.low_coverage_origins },
                        is_normalized_price: sig_meta.is_normalized_price,
                    };
                }

                // Other time-series operators: ts_delay, ts_delta, ts_mean, ts_decay_linear, etc.
                if func_lower.starts_with("ts_") {
                    let mut sig_meta = ValueMeta::default();
                    if !args.is_empty() {
                        sig_meta = self.eval_expr(&args[0], symbol_table, errors, warnings, extracted_fields);
                    }

                    if sig_meta.expr_type == FastExprType::Vector {
                        errors.push(ValidationError {
                            code: DiagnosticCode::ErrVecTypeMismatch,
                            field_id: None,
                            message: format!("Time-series operator '{}' cannot operate on VECTOR field", func),
                            suggestion: Some("Fold vector with vec_avg(...) before applying time-series operator".to_string()),
                        });
                    }

                    // Check lag parameter if present (typically second argument)
                    if args.len() >= 2 {
                        if let Some(lag) = extract_literal_number(&args[1]) {
                            if lag <= 0.0 {
                                errors.push(ValidationError {
                                    code: DiagnosticCode::ErrNegativeLookback,
                                    field_id: None,
                                    message: format!(
                                        "Operator '{}' received non-positive lookback lag ({})",
                                        func, lag
                                    ),
                                    suggestion: Some("Lag parameter must be >= 1 to prevent look-ahead bias".to_string()),
                                });
                            }
                        }
                    }

                    return ValueMeta {
                        expr_type: FastExprType::Matrix,
                        is_backfilled: sig_meta.is_backfilled,
                        backfill_window: sig_meta.backfill_window,
                        low_coverage_origins: sig_meta.low_coverage_origins,
                        is_normalized_price: sig_meta.is_normalized_price,
                    };
                }

                // rank(...)
                if func_lower == "rank" {
                    if !args.is_empty() {
                        let inner = self.eval_expr(&args[0], symbol_table, errors, warnings, extracted_fields);
                        if inner.expr_type == FastExprType::Vector {
                            errors.push(ValidationError {
                                code: DiagnosticCode::ErrVecTypeMismatch,
                                field_id: None,
                                message: "rank(...) cannot be applied to a VECTOR field".to_string(),
                                suggestion: Some("Wrap in vec_avg(...) before ranking".to_string()),
                            });
                        }
                        if !inner.is_normalized_price {
                            warnings.push(ValidationWarning {
                                code: DiagnosticCode::WarnUnnormalizedPrice,
                                field_id: None,
                                message: "Raw price or breakeven level passed to rank() without close normalization".to_string(),
                                suggestion: Some("Normalize by close (e.g., call_breakeven_30 / close - 1) before ranking".to_string()),
                            });
                        }
                        return ValueMeta {
                            expr_type: FastExprType::Matrix,
                            is_backfilled: inner.is_backfilled,
                            backfill_window: inner.backfill_window,
                            low_coverage_origins: inner.low_coverage_origins,
                            is_normalized_price: true,
                        };
                    }
                }
==== REPLACEMENT
                // Vector folding functions: vec_avg, vec_sum, vec_max, vec_min, vec_count, vec_std
                if matches!(
                    func_lower.as_str(),
                    "vec_avg" | "vec_sum" | "vec_max" | "vec_min" | "vec_count" | "vec_std"
                ) {
                    if args.len() != 1 {
                        errors.push(ValidationError {
                            code: DiagnosticCode::ErrSyntaxMalformed,
                            field_id: None,
                            message: format!("'{}' requires exactly 1 argument, found {}", func, args.len()),
                            suggestion: Some(format!("Usage: {}(vector_field)", func)),
                        });
                        return ValueMeta {
                            expr_type: FastExprType::TypeError,
                            ..Default::default()
                        };
                    }

                    let arg_meta = self.eval_expr(&args[0], symbol_table, errors, warnings, extracted_fields);
                    return ValueMeta {
                        expr_type: FastExprType::Matrix,
                        is_backfilled: arg_meta.is_backfilled,
                        backfill_window: arg_meta.backfill_window,
                        low_coverage_origins: arg_meta.low_coverage_origins,
                        is_normalized_price: true,
                        scalar_value: None,
                    };
                }

                // densify(group)
                if func_lower == "densify" {
                    if args.len() != 1 {
                        errors.push(ValidationError {
                            code: DiagnosticCode::ErrSyntaxMalformed,
                            field_id: None,
                            message: format!("'densify' requires exactly 1 argument (group), found {}", args.len()),
                            suggestion: Some("Usage: densify(subindustry)".to_string()),
                        });
                        return ValueMeta {
                            expr_type: FastExprType::TypeError,
                            ..Default::default()
                        };
                    }
                    let _ = self.eval_expr(&args[0], symbol_table, errors, warnings, extracted_fields);
                    return ValueMeta {
                        expr_type: FastExprType::DensifiedGroup,
                        is_backfilled: true,
                        backfill_window: None,
                        low_coverage_origins: Vec::new(),
                        is_normalized_price: true,
                        scalar_value: None,
                    };
                }

                // group_neutralize(signal, densify(group))
                if func_lower == "group_neutralize" {
                    if args.len() != 2 {
                        errors.push(ValidationError {
                            code: DiagnosticCode::ErrSyntaxMalformed,
                            field_id: None,
                            message: format!("group_neutralize requires exactly 2 arguments: (signal, densify(group)), found {}", args.len()),
                            suggestion: Some("Usage: group_neutralize(signal, densify(subindustry))".to_string()),
                        });
                        return ValueMeta {
                            expr_type: FastExprType::TypeError,
                            ..Default::default()
                        };
                    }

                    let sig_meta = self.eval_expr(&args[0], symbol_table, errors, warnings, extracted_fields);
                    let group_meta = self.eval_expr(&args[1], symbol_table, errors, warnings, extracted_fields);

                    if sig_meta.expr_type == FastExprType::Vector {
                        errors.push(ValidationError {
                            code: DiagnosticCode::ErrVecTypeMismatch,
                            field_id: None,
                            message: "group_neutralize signal argument cannot be of type VECTOR".to_string(),
                            suggestion: Some("Fold vector signal with vec_avg(...) before neutralizing".to_string()),
                        });
                    }

                    // Enforce densify on second argument
                    let is_densified = group_meta.expr_type == FastExprType::DensifiedGroup
                        || matches!(&args[1], AstExpr::Call { func, .. } if func.eq_ignore_ascii_case("densify"));

                    if !is_densified {
                        errors.push(ValidationError {
                            code: DiagnosticCode::ErrUndensifiedGroup,
                            field_id: None,
                            message: "group_neutralize grouping variable must be wrapped in densify(...)".to_string(),
                            suggestion: Some("Wrap grouping parameter in densify(group), e.g., densify(subindustry)".to_string()),
                        });
                    }

                    return ValueMeta {
                        expr_type: FastExprType::Matrix,
                        is_backfilled: sig_meta.is_backfilled,
                        backfill_window: sig_meta.backfill_window,
                        low_coverage_origins: sig_meta.low_coverage_origins,
                        is_normalized_price: true,
                        scalar_value: None,
                    };
                }

                // Helper to extract constant scalar numbers across literals, unary, binary arithmetic, and symbol table
                fn extract_constant_number(expr: &AstExpr, symbol_table: &HashMap<String, ValueMeta>) -> Option<f64> {
                    match expr {
                        AstExpr::Literal(n) => Some(*n),
                        AstExpr::Unary { op, expr } => {
                            let inner = extract_constant_number(expr, symbol_table)?;
                            match op.as_str() {
                                "-" => Some(-inner),
                                "+" => Some(inner),
                                _ => None,
                            }
                        }
                        AstExpr::Binary { op, left, right } => {
                            let l = extract_constant_number(left, symbol_table)?;
                            let r = extract_constant_number(right, symbol_table)?;
                            match op.as_str() {
                                "+" => Some(l + r),
                                "-" => Some(l - r),
                                "*" => Some(l * r),
                                "/" => if r.abs() > 1e-9 { Some(l / r) } else { None },
                                _ => None,
                            }
                        }
                        AstExpr::Variable(name) => {
                            symbol_table.get(name).and_then(|m| m.scalar_value)
                        }
                        _ => None,
                    }
                }

                // ts_backfill(arg, window)
                if func_lower == "ts_backfill" {
                    if args.len() != 2 {
                        errors.push(ValidationError {
                            code: DiagnosticCode::ErrSyntaxMalformed,
                            field_id: None,
                            message: format!("ts_backfill requires exactly 2 arguments: (signal, window), found {}", args.len()),
                            suggestion: Some("Usage: ts_backfill(signal, 15)".to_string()),
                        });
                        return ValueMeta {
                            expr_type: FastExprType::TypeError,
                            ..Default::default()
                        };
                    }

                    let sig_meta = self.eval_expr(&args[0], symbol_table, errors, warnings, extracted_fields);
                    if sig_meta.expr_type == FastExprType::Vector {
                        errors.push(ValidationError {
                            code: DiagnosticCode::ErrVecTypeMismatch,
                            field_id: None,
                            message: "ts_backfill cannot operate directly on VECTOR field; fold first".to_string(),
                            suggestion: Some("Wrap vector field in vec_avg(...) before applying ts_backfill".to_string()),
                        });
                    }

                    let mut window_val = 0i64;
                    if let Some(w) = extract_constant_number(&args[1], symbol_table) {
                        window_val = w as i64;
                        if window_val < 1 {
                            errors.push(ValidationError {
                                code: DiagnosticCode::ErrNegativeLookback,
                                field_id: None,
                                message: format!(
                                    "ts_backfill received non-positive lookback window ({})",
                                    window_val
                                ),
                                suggestion: Some("Lookback window must be an integer >= 1".to_string()),
                            });
                        }
                    } else {
                        errors.push(ValidationError {
                            code: DiagnosticCode::ErrNegativeLookback,
                            field_id: None,
                            message: "ts_backfill window parameter must be a statically resolvable positive integer".to_string(),
                            suggestion: Some("Usage: ts_backfill(signal, 15)".to_string()),
                        });
                    }

                    return ValueMeta {
                        expr_type: sig_meta.expr_type,
                        is_backfilled: window_val >= 5,
                        backfill_window: Some(window_val),
                        low_coverage_origins: if window_val >= 5 { Vec::new() } else { sig_meta.low_coverage_origins },
                        is_normalized_price: sig_meta.is_normalized_price,
                        scalar_value: None,
                    };
                }

                // Other time-series operators: ts_delay, ts_delta, ts_mean, ts_decay_linear, etc.
                if func_lower.starts_with("ts_") {
                    if args.len() < 2 {
                        errors.push(ValidationError {
                            code: DiagnosticCode::ErrSyntaxMalformed,
                            field_id: None,
                            message: format!("Time-series operator '{}' requires at least 2 arguments: (signal, lookback), found {}", func, args.len()),
                            suggestion: Some(format!("Usage: {}(signal, 10)", func)),
                        });
                        return ValueMeta {
                            expr_type: FastExprType::TypeError,
                            ..Default::default()
                        };
                    }

                    let sig_meta = self.eval_expr(&args[0], symbol_table, errors, warnings, extracted_fields);

                    if sig_meta.expr_type == FastExprType::Vector {
                        errors.push(ValidationError {
                            code: DiagnosticCode::ErrVecTypeMismatch,
                            field_id: None,
                            message: format!("Time-series operator '{}' cannot operate on VECTOR field", func),
                            suggestion: Some("Fold vector with vec_avg(...) before applying time-series operator".to_string()),
                        });
                    }

                    // Check lag parameter (typically second argument)
                    if let Some(lag) = extract_constant_number(&args[1], symbol_table) {
                        if lag <= 0.0 {
                            errors.push(ValidationError {
                                code: DiagnosticCode::ErrNegativeLookback,
                                field_id: None,
                                message: format!(
                                    "Operator '{}' received non-positive lookback lag ({})",
                                    func, lag
                                ),
                                suggestion: Some("Lag parameter must be >= 1 to prevent look-ahead bias".to_string()),
                            });
                        }
                    } else {
                        let lag_meta = self.eval_expr(&args[1], symbol_table, errors, warnings, extracted_fields);
                        if lag_meta.expr_type != FastExprType::Scalar {
                            errors.push(ValidationError {
                                code: DiagnosticCode::ErrNegativeLookback,
                                field_id: None,
                                message: format!("Operator '{}' lookback lag must be a scalar constant", func),
                                suggestion: Some("Provide a constant positive integer for the lag parameter".to_string()),
                            });
                        }
                    }

                    return ValueMeta {
                        expr_type: FastExprType::Matrix,
                        is_backfilled: sig_meta.is_backfilled,
                        backfill_window: sig_meta.backfill_window,
                        low_coverage_origins: sig_meta.low_coverage_origins,
                        is_normalized_price: sig_meta.is_normalized_price,
                        scalar_value: None,
                    };
                }

                // rank(...)
                if func_lower == "rank" {
                    if args.len() != 1 {
                        errors.push(ValidationError {
                            code: DiagnosticCode::ErrSyntaxMalformed,
                            field_id: None,
                            message: format!("'rank' requires exactly 1 argument (signal), found {}", args.len()),
                            suggestion: Some("Usage: rank(signal)".to_string()),
                        });
                        return ValueMeta {
                            expr_type: FastExprType::TypeError,
                            ..Default::default()
                        };
                    }

                    let inner = self.eval_expr(&args[0], symbol_table, errors, warnings, extracted_fields);
                    if inner.expr_type == FastExprType::Vector {
                        errors.push(ValidationError {
                            code: DiagnosticCode::ErrVecTypeMismatch,
                            field_id: None,
                            message: "rank(...) cannot be applied to a VECTOR field".to_string(),
                            suggestion: Some("Wrap in vec_avg(...) before ranking".to_string()),
                        });
                    }
                    if !inner.is_normalized_price {
                        warnings.push(ValidationWarning {
                            code: DiagnosticCode::WarnUnnormalizedPrice,
                            field_id: None,
                            message: "Raw price or breakeven level passed to rank() without close normalization".to_string(),
                            suggestion: Some("Normalize by close (e.g., call_breakeven_30 / close - 1) before ranking".to_string()),
                        });
                    }
                    return ValueMeta {
                        expr_type: FastExprType::Matrix,
                        is_backfilled: inner.is_backfilled,
                        backfill_window: inner.backfill_window,
                        low_coverage_origins: inner.low_coverage_origins,
                        is_normalized_price: true,
                        scalar_value: None,
                    };
                }
>>>>
```

---

## 4. Patch Specifications for `scratch/metadata_tool.py`

**Target File**: `scratch/metadata_tool.py`  
**Target Lines**: 260–270

Add helper function:
```python
def extract_call_args(text: str, func_name: str) -> list[list[str]]:
    """Extract argument lists for all occurrences of func_name, respecting nested parentheses."""
    results = []
    pattern = re.compile(rf"\b{func_name}\s*\(")
    for m in pattern.finditer(text):
        start = m.end()
        depth = 1
        i = start
        args = []
        cur = []
        while i < len(text) and depth > 0:
            c = text[i]
            if c == '(':
                depth += 1
                cur.append(c)
            elif c == ')':
                depth -= 1
                if depth == 0:
                    args.append("".join(cur).strip())
                    break
                else:
                    cur.append(c)
            elif c == ',' and depth == 1:
                args.append("".join(cur).strip())
                cur = []
            else:
                cur.append(c)
            i += 1
        if args:
            results.append(args)
    return results
```

In `validate_expression`:
```python
<<<< EXISTING
    # 2. Check densify in group_neutralize
    if "group_neutralize" in expr:
        # Match group_neutralize(..., ...)
        gn_matches = re.findall(r"group_neutralize\s*\([^,]+,\s*([^)]+)\)", expr)
        for g_arg in gn_matches:
            if not g_arg.strip().startswith("densify("):
                errors.append(
                    f"group_neutralize grouping argument '{g_arg.strip()}' must be wrapped in densify(...)."
                )
==== REPLACEMENT
    # 2. Check densify in group_neutralize
    if "group_neutralize" in expr:
        gn_calls = extract_call_args(expr, "group_neutralize")
        for args in gn_calls:
            if len(args) < 2:
                errors.append("group_neutralize requires 2 arguments: (signal, densify(group)).")
            else:
                g_arg = args[1].strip()
                if not g_arg.startswith("densify("):
                    errors.append(
                        f"group_neutralize grouping argument '{g_arg}' must be wrapped in densify(...)."
                    )
>>>>
```

---

## 5. Test Suite Updates for `tests/stress_schema_validator_tests.rs`

Challenger 1 initially wrote bug reproducibility assertions asserting the buggy behavior (`assert!(rep.is_valid, ...)`).
Now that the issues will be remediated, those tests must assert strict **fail-closed** behavior and correct validation:

```rust
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
    assert!(rep.errors.iter().any(|e| e.code == DiagnosticCode::ErrMissingBackfill));
}

#[test]
fn test_challenger_reproduce_literal_scalar_breaks_price_normalization() {
    let validator = SchemaValidator::new();

    // Vulnerability 2 Remediated:
    // Subtracting 1.0 from (call_breakeven_30 / close) must NOT trigger WarnUnnormalizedPrice.
    let expr = "group_neutralize(rank(call_breakeven_30 / close - 1.0), densify(subindustry))";
    let rep = validator.validate(expr);

    assert!(
        !rep.warnings.iter().any(|w| w.code == DiagnosticCode::WarnUnnormalizedPrice),
        "Remediated: Literal 1.0 is dimensionless and must not emit WarnUnnormalizedPrice"
    );
    assert!(rep.is_valid, "Valid normalized ratio minus 1.0 must pass: {:?}", rep.errors);
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
    assert!(rep.errors.iter().any(|e| e.code == DiagnosticCode::ErrNegativeLookback));

    // Also test constant variable lag
    let var_expr = "lag = -5; group_neutralize(rank(ts_delay(close, lag)), densify(subindustry))";
    let rep_var = validator.validate(var_expr);
    assert!(!rep_var.is_valid);
    assert!(rep_var.errors.iter().any(|e| e.code == DiagnosticCode::ErrNegativeLookback));
}

#[test]
fn test_challenger_reproduce_missing_arity_checks() {
    let validator = SchemaValidator::new();

    // Vulnerability 4 Remediated:
    // Function arity checks strictly enforced.
    let rep_ts_delay = validator.validate("group_neutralize(rank(ts_delay()), densify(subindustry))");
    let rep_rank = validator.validate("group_neutralize(rank(), densify(subindustry))");
    let rep_densify = validator.validate("group_neutralize(rank(close), densify())");

    assert!(!rep_ts_delay.is_valid, "Remediated: ts_delay() with 0 args must fail");
    assert!(rep_ts_delay.errors.iter().any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));

    assert!(!rep_rank.is_valid, "Remediated: rank() with 0 args must fail");
    assert!(rep_rank.errors.iter().any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));

    assert!(!rep_densify.is_valid, "Remediated: densify() with 0 args must fail");
    assert!(rep_densify.errors.iter().any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));
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
    assert!(rep.errors.iter().any(|e| e.code == DiagnosticCode::ErrUnknownField));
}

#[test]
fn test_challenger_reproduce_empty_and_assignment_only_scripts_pass() {
    let validator = SchemaValidator::new();

    // Vulnerability 6 Remediated:
    // Empty strings, comments, and assignment-only scripts must fail validation.
    let rep_empty = validator.validate("");
    let rep_comment = validator.validate("# Just a comment");
    let rep_assign_only = validator.validate("x = 1; y = 2;");

    assert!(!rep_empty.is_valid, "Remediated: empty expression must fail");
    assert!(rep_empty.errors.iter().any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));

    assert!(!rep_comment.is_valid, "Remediated: comment-only string must fail");
    assert!(rep_comment.errors.iter().any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));

    assert!(!rep_assign_only.is_valid, "Remediated: assignment-only script must fail");
    assert!(rep_assign_only.errors.iter().any(|e| e.code == DiagnosticCode::ErrSyntaxMalformed));
}
```

---

## 6. Verification Protocol for Worker

The Worker must execute the following sequence to achieve gate approval:

1. **Unit and Stress Test Execution**:
   ```bash
   cargo test --test stress_schema_validator_tests
   cargo test --test metadata_schema_tests
   cargo test
   ```
   *Pass Criteria*: All 12 stress tests pass (with inverted/fail-closed assertions); all 21 metadata schema tests pass; all crate tests pass (139+ total tests).

2. **Python Scratch Tool Verification**:
   ```bash
   python3 scratch/metadata_tool.py --validate "group_neutralize(ts_decay_linear(rank(close), 5), densify(subindustry))"
   ```
   *Pass Criteria*: Prints `Validation: PASSED` (no false positive comma splitting).

3. **CLI Integration Verification**:
   ```bash
   cargo run -- fields --validate "group_neutralize(rank(ts_backfill(fscore_momentum, 15) + fscore_growth), densify(subindustry))"
   ```
   *Pass Criteria*: Output reports `VALIDATION FAILED` with `ERR_MISSING_BACKFILL` for `fscore_growth`.

4. **Git Hygiene Audit**:
   ```bash
   bash scripts/sanitize_check.sh
   ```
   *Pass Criteria*: Pre-Push Audit PASSED with 0 confidential data leaks.
