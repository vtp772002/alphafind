use alphafind::taxonomy::{get_curated_candidates, FactorPillar, PlaceholderRegistry};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::thread;

// ============================================================================
// Group 1: Adversarial Template & Placeholder Extraction Edge Cases
// ============================================================================

#[test]
fn test_adversarial_placeholder_malformed_brackets() {
    // Unmatched opening brackets
    assert_eq!(
        PlaceholderRegistry::extract_placeholders("<FIELD"),
        Vec::<String>::new()
    );
    assert_eq!(
        PlaceholderRegistry::extract_placeholders("prefix <FIELD"),
        Vec::<String>::new()
    );
    assert_eq!(
        PlaceholderRegistry::extract_placeholders("<"),
        Vec::<String>::new()
    );

    // Unmatched closing brackets
    assert_eq!(
        PlaceholderRegistry::extract_placeholders("FIELD>"),
        Vec::<String>::new()
    );
    assert_eq!(
        PlaceholderRegistry::extract_placeholders(">"),
        Vec::<String>::new()
    );

    // Empty brackets
    assert_eq!(
        PlaceholderRegistry::extract_placeholders("<>"),
        Vec::<String>::new()
    );
    assert_eq!(
        PlaceholderRegistry::extract_placeholders("rank(<>)"),
        Vec::<String>::new()
    );

    // Whitespace inside brackets
    assert_eq!(
        PlaceholderRegistry::extract_placeholders("< >"),
        Vec::<String>::new()
    );
    assert_eq!(
        PlaceholderRegistry::extract_placeholders("< FIELD >"),
        Vec::<String>::new()
    );
    assert_eq!(
        PlaceholderRegistry::extract_placeholders("<FIELD >"),
        Vec::<String>::new()
    );
    assert_eq!(
        PlaceholderRegistry::extract_placeholders("< FIELD>"),
        Vec::<String>::new()
    );

    // Lowercase and mixed-case placeholders (should NOT be extracted by contract)
    assert_eq!(
        PlaceholderRegistry::extract_placeholders("<field>"),
        Vec::<String>::new()
    );
    assert_eq!(
        PlaceholderRegistry::extract_placeholders("<Field>"),
        Vec::<String>::new()
    );
    assert_eq!(
        PlaceholderRegistry::extract_placeholders("<field_name>"),
        Vec::<String>::new()
    );

    // Special characters inside brackets (should NOT be extracted)
    assert_eq!(
        PlaceholderRegistry::extract_placeholders("<FIELD-NAME>"),
        Vec::<String>::new()
    );
    assert_eq!(
        PlaceholderRegistry::extract_placeholders("<FIELD.NAME>"),
        Vec::<String>::new()
    );
    assert_eq!(
        PlaceholderRegistry::extract_placeholders("<FIELD$NAME>"),
        Vec::<String>::new()
    );
    assert_eq!(
        PlaceholderRegistry::extract_placeholders("<FIELD#NAME>"),
        Vec::<String>::new()
    );

    // Digits and underscores are allowed
    let valid_num = PlaceholderRegistry::extract_placeholders("<FIELD_123_ABC>");
    assert_eq!(valid_num, vec!["<FIELD_123_ABC>"]);
}

#[test]
fn test_adversarial_placeholder_nested_and_consecutive_brackets() {
    // Consecutive placeholders
    let consecutive = "<FIELD1><FIELD2>";
    let tokens = PlaceholderRegistry::extract_placeholders(consecutive);
    assert_eq!(tokens, vec!["<FIELD1>", "<FIELD2>"]);

    // Consecutive separated by underscore
    let sep = "<FIELD1>_<FIELD2>";
    let tokens_sep = PlaceholderRegistry::extract_placeholders(sep);
    assert_eq!(tokens_sep, vec!["<FIELD1>", "<FIELD2>"]);

    // Nested double opening brackets: "<<FIELD>>"
    // Behavior check: Inner slice is "<FIELD" which has '<', so it fails char check.
    // The inner '<FIELD>' is skipped because i jumps past the first '>'.
    let nested = "<<FIELD>>";
    let nested_tokens = PlaceholderRegistry::extract_placeholders(nested);
    // Documenting empirical behavior: nested brackets do not extract tokens
    assert!(
        nested_tokens.is_empty(),
        "Nested <<FIELD>> should not extract valid token"
    );
}

#[test]
fn test_adversarial_placeholder_unicode_and_special_strings() {
    // UTF-8 characters surrounding placeholders
    let utf8_expr =
        "alpha_β = rank(<SIGNAL>) + ts_mean(γ, 10); group_neutralize(alpha_β, densify(<GROUP>))";
    let extracted = PlaceholderRegistry::extract_placeholders(utf8_expr);
    assert_eq!(extracted.len(), 2);
    assert!(extracted.contains(&"<SIGNAL>".to_string()));
    assert!(extracted.contains(&"<GROUP>".to_string()));

    // UTF-8 inside bracket (should not extract as ASCII token)
    let non_ascii_inner = "<SIGNAL_λ>";
    assert_eq!(
        PlaceholderRegistry::extract_placeholders(non_ascii_inner),
        Vec::<String>::new()
    );

    let emoji_inner = "<SIGNAL_🚀>";
    assert_eq!(
        PlaceholderRegistry::extract_placeholders(emoji_inner),
        Vec::<String>::new()
    );

    // FastExpr comparison operators with spaces
    let comparison = "if_else(x < y && y > z, 1.0, -1.0)";
    assert_eq!(
        PlaceholderRegistry::extract_placeholders(comparison),
        Vec::<String>::new()
    );
}

#[test]
fn test_adversarial_placeholder_extreme_scale() {
    // 50,000 character string without placeholders
    let huge_clean = "a + b * c / d - e ".repeat(2500);
    assert!(!PlaceholderRegistry::has_unresolved_placeholders(
        &huge_clean
    ));
    assert_eq!(
        PlaceholderRegistry::extract_placeholders(&huge_clean),
        Vec::<String>::new()
    );

    // Expression with 100 unique placeholders
    let mut template = String::new();
    for i in 0..100 {
        template.push_str(&format!("+ rank(<TOKEN_{}>) ", i));
    }
    let tokens = PlaceholderRegistry::extract_placeholders(&template);
    assert_eq!(tokens.len(), 100);
}

// ============================================================================
// Group 2: Adversarial Template Rendering Edge Cases
// ============================================================================

#[test]
fn test_render_template_empty_substitutions() {
    let template = "rank(<FIELD>) - rank(<SIGNAL>)";
    let mut bindings = HashMap::new();
    // Empty substitution
    bindings.insert("<FIELD>", "");
    bindings.insert("<SIGNAL>", "");

    let rendered = PlaceholderRegistry::render_template(template, &bindings);
    assert!(
        rendered.is_ok(),
        "Rendering with empty replacement string should succeed"
    );
    assert_eq!(rendered.unwrap(), "rank() - rank()");
}

#[test]
fn test_render_template_missing_and_superfluous_bindings() {
    let template = "sig = <CORE_FIELD> * <WEIGHT>; densify(<GROUP>)";

    // Incomplete bindings: <WEIGHT> is missing
    let mut incomplete = HashMap::new();
    incomplete.insert("<CORE_FIELD>", "close");
    let err = PlaceholderRegistry::render_template(template, &incomplete);
    assert!(err.is_err());
    let missing = err.unwrap_err();
    assert_eq!(missing.len(), 2);
    assert!(missing.contains(&"<WEIGHT>".to_string()));
    assert!(missing.contains(&"<GROUP>".to_string()));

    // Complete bindings with extra unused keys
    let mut complete = HashMap::new();
    complete.insert("<CORE_FIELD>", "close");
    complete.insert("<WEIGHT>", "0.5");
    complete.insert("<GROUP>", "sector");
    complete.insert("<EXTRA_KEY>", "unused_value");

    let ok = PlaceholderRegistry::render_template(template, &complete);
    assert!(ok.is_ok());
    assert_eq!(ok.unwrap(), "sig = close * 0.5; densify(sector)");
}

#[test]
fn test_render_template_nested_placeholder_injection() {
    // Binding value itself introduces an unresolved placeholder
    let template = "sig = rank(<A>)";
    let mut bindings = HashMap::new();
    bindings.insert("<A>", "<B>");

    let res = PlaceholderRegistry::render_template(template, &bindings);
    assert!(res.is_err(), "Replacing <A> with <B> leaves unresolved <B>");
    let missing = res.unwrap_err();
    assert_eq!(missing, vec!["<B>".to_string()]);
}

// ============================================================================
// Group 3: Adversarial FastExpr Syntax Validation Edge Cases
// ============================================================================

#[test]
fn test_validate_fastexpr_balanced_parentheses() {
    // Valid balanced expressions
    assert!(PlaceholderRegistry::validate_fastexpr_syntax("rank(close)").is_ok());
    assert!(PlaceholderRegistry::validate_fastexpr_syntax("((((close))))").is_ok());
    assert!(
        PlaceholderRegistry::validate_fastexpr_syntax("").is_ok(),
        "Empty expression structurally balanced"
    );

    // Premature closing parenthesis
    let err_premature = PlaceholderRegistry::validate_fastexpr_syntax(")(");
    assert!(err_premature.is_err());
    assert!(
        err_premature.unwrap_err()[0].contains("Unmatched closing parenthesis ')' at char index 0")
    );

    // Deep premature close
    let err_deep = PlaceholderRegistry::validate_fastexpr_syntax("rank(close) ) + 1");
    assert!(err_deep.is_err());
    assert!(err_deep.unwrap_err()[0].contains("Unmatched closing parenthesis"));

    // Unclosed open parenthesis
    let err_unclosed = PlaceholderRegistry::validate_fastexpr_syntax("rank(close");
    assert!(err_unclosed.is_err());
    assert!(err_unclosed.unwrap_err()[0].contains("Unclosed open parentheses: 1 unclosed '('"));

    let err_multi_unclosed = PlaceholderRegistry::validate_fastexpr_syntax("((((close)");
    assert!(err_multi_unclosed.is_err());
    assert!(err_multi_unclosed.unwrap_err()[0].contains("3 unclosed '('"));
}

#[test]
fn test_validate_fastexpr_group_neutralize_densify() {
    // Correct group_neutralize with densify
    let valid1 = "group_neutralize(rank(close), densify(subindustry))";
    assert!(PlaceholderRegistry::validate_fastexpr_syntax(valid1).is_ok());

    // Case-insensitivity for DENSIFY
    let valid_caps = "group_neutralize(rank(close), DENSIFY(subindustry))";
    assert!(PlaceholderRegistry::validate_fastexpr_syntax(valid_caps).is_ok());

    // Missing densify wrapper on group_neutralize
    let invalid = "group_neutralize(rank(close), subindustry)";
    let err = PlaceholderRegistry::validate_fastexpr_syntax(invalid);
    assert!(err.is_err());
    assert!(err.unwrap_err()[0].contains("must wrap grouping variable in densify"));
}

#[test]
fn test_validate_fastexpr_multiple_simultaneous_errors() {
    // Unresolved placeholder + unmatched parenthesis + missing densify
    let compound_bad = "sig = rank(<UNKNOWN_TOKEN; group_neutralize(sig, subindustry)";
    let err = PlaceholderRegistry::validate_fastexpr_syntax(compound_bad);
    assert!(err.is_err());
    let errs = err.unwrap_err();
    // Should catch unbalanced parens and missing densify
    assert!(errs.iter().any(|e| e.contains("parenthes")));
    assert!(errs.iter().any(|e| e.contains("densify")));
}

// ============================================================================
// Group 4: Adversarial Candidate JSON Loading Edge Cases
// ============================================================================

#[test]
fn test_candidate_loading_from_corrupted_and_nonstandard_json() {
    let tmp_dir = std::env::temp_dir();

    // 1. Completely empty file
    let path_empty = tmp_dir.join("test_candidates_empty.json");
    {
        let mut f = File::create(&path_empty).unwrap();
        write!(f, "").unwrap();
    }
    std::env::set_var("ALPHAFIND_CANDIDATES_PATH", &path_empty);
    let candidates_empty = get_curated_candidates(None, Some("TOP1000"));
    assert!(
        !candidates_empty.is_empty(),
        "Empty file must cleanly fall back to curated presets"
    );

    // 2. Malformed JSON syntax (broken braces)
    let path_corrupt = tmp_dir.join("test_candidates_corrupt.json");
    {
        let mut f = File::create(&path_corrupt).unwrap();
        write!(f, "{{ \"broken\": [").unwrap();
    }
    std::env::set_var("ALPHAFIND_CANDIDATES_PATH", &path_corrupt);
    let candidates_corrupt = get_curated_candidates(None, Some("TOP1000"));
    assert!(
        !candidates_corrupt.is_empty(),
        "Corrupt JSON must cleanly fall back to curated presets"
    );

    // 3. JSON is an object instead of array
    let path_obj = tmp_dir.join("test_candidates_obj.json");
    {
        let mut f = File::create(&path_obj).unwrap();
        write!(f, "{{\"candidates\": []}}").unwrap();
    }
    std::env::set_var("ALPHAFIND_CANDIDATES_PATH", &path_obj);
    let candidates_obj = get_curated_candidates(None, Some("TOP1000"));
    assert!(
        !candidates_obj.is_empty(),
        "JSON object must cleanly fall back to curated presets"
    );

    // 4. JSON array of primitives
    let path_prim = tmp_dir.join("test_candidates_prim.json");
    {
        let mut f = File::create(&path_prim).unwrap();
        write!(f, "[1, 2, \"string\", true]").unwrap();
    }
    std::env::set_var("ALPHAFIND_CANDIDATES_PATH", &path_prim);
    let candidates_prim = get_curated_candidates(None, Some("TOP1000"));
    assert!(
        !candidates_prim.is_empty(),
        "JSON primitives array must cleanly fall back to curated presets"
    );

    // 5. Mixed JSON array (valid candidate alongside malformed items)
    let path_mixed = tmp_dir.join("test_candidates_mixed.json");
    {
        let mut f = File::create(&path_mixed).unwrap();
        write!(
            f,
            r#"[
                {{"broken": true}},
                {{
                    "name": "US_D1_TOP1000_ValidCandidate_Dec10",
                    "expression": "group_neutralize(rank(close), densify(subindustry))",
                    "universe": "TOP1000",
                    "decay": 10,
                    "neutralization": "SUBINDUSTRY",
                    "truncation": 0.05,
                    "pillar": "MODEL51"
                }},
                {{"missing_fields": 1}}
            ]"#
        )
        .unwrap();
    }
    std::env::set_var("ALPHAFIND_CANDIDATES_PATH", &path_mixed);
    let candidates_mixed =
        get_curated_candidates(Some(FactorPillar::SystematicRisk), Some("TOP1000"));
    assert_eq!(
        candidates_mixed.len(),
        1,
        "Should filter_map valid candidate and skip invalid items"
    );
    assert_eq!(
        candidates_mixed[0].name,
        "US_D1_TOP1000_ValidCandidate_Dec10"
    );
    assert_eq!(
        candidates_mixed[0].pillar, "SYSTEMATIC_RISK",
        "MODEL51 alias should be canonicalized"
    );

    // Cleanup env var
    std::env::remove_var("ALPHAFIND_CANDIDATES_PATH");
}

// ============================================================================
// Group 5: Exhaustive Compatibility & Behavioral Audit Across All Factor Pillars
// ============================================================================

#[test]
fn test_all_factor_pillar_variants_exhaustive() {
    let all = FactorPillar::all();
    assert_eq!(
        all.len(),
        12,
        "FactorPillar::all() must yield exactly 12 variants"
    );

    for &pillar in all {
        // 1. as_str() contract
        let s = pillar.as_str();
        assert!(!s.is_empty(), "Pillar as_str() must not be empty");
        assert!(
            s.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
            "Pillar as_str() must be UPPER_SNAKE_CASE"
        );

        // 2. from_str() round-trip contract
        let parsed = FactorPillar::from_str(s);
        assert_eq!(
            parsed,
            Some(pillar),
            "Round-trip from_str(as_str()) failed for {:?}",
            pillar
        );

        // 3. description() contract
        assert!(
            !pillar.description().is_empty(),
            "Description must not be empty for {:?}",
            pillar
        );

        // 4. primary_dataset_id() contract
        let ds = pillar.primary_dataset_id();
        assert!(
            !ds.is_empty(),
            "Primary dataset ID must not be empty for {:?}",
            pillar
        );

        // 5. canonical_legacy_pillar() contract
        let legacy = pillar.canonical_legacy_pillar();
        // A legacy pillar must map to itself
        assert_eq!(
            legacy.canonical_legacy_pillar(),
            legacy,
            "Legacy pillar mapping must be idempotent"
        );

        // 6. Curated candidates behavior for each pillar under clean state (no private store)
        let tmp_empty = std::env::temp_dir().join("clean_empty.json");
        let _ = File::create(&tmp_empty);
        std::env::set_var("ALPHAFIND_CANDIDATES_PATH", &tmp_empty);

        let candidates = get_curated_candidates(Some(pillar), Some("TOP1000"));
        assert!(
            !candidates.is_empty(),
            "Curated candidates must return at least one hypothesis for pillar {:?}",
            pillar
        );

        for c in &candidates {
            assert!(!c.name.is_empty());
            assert!(!c.expression.is_empty());
            assert!(c.decay > 0);
            assert!(c.truncation > 0.0);
            // Verify to_settings conversion does not panic
            let settings = c.to_settings();
            assert_eq!(settings.universe, c.universe);
            assert_eq!(settings.decay, c.decay);
        }

        std::env::remove_var("ALPHAFIND_CANDIDATES_PATH");
    }
}

#[test]
fn test_financial_health_quality_pillar_fallback_behavior() {
    // Specifically stress-testing FactorPillar::FinancialHealthQuality
    // under isolated candidate store conditions
    let tmp_empty = std::env::temp_dir().join("fhq_empty.json");
    let _ = File::create(&tmp_empty);
    std::env::set_var("ALPHAFIND_CANDIDATES_PATH", &tmp_empty);

    let candidates =
        get_curated_candidates(Some(FactorPillar::FinancialHealthQuality), Some("TOP1000"));
    assert!(
        !candidates.is_empty(),
        "FinancialHealthQuality must return candidates via default curated batch"
    );

    // Check if candidates returned for FinancialHealthQuality actually match FinancialHealthQuality
    let matching_count = candidates
        .iter()
        .filter(|c| c.pillar == "FINANCIAL_HEALTH_QUALITY")
        .count();
    // Empirical observation: FinancialHealthQuality does not have its own dedicated arm in match pillar,
    // so it falls back to get_default_curated_batch(u) which contains candidates from other pillars.
    println!(
        "FinancialHealthQuality candidates count: {}, matching pillar count: {}",
        candidates.len(),
        matching_count
    );

    std::env::remove_var("ALPHAFIND_CANDIDATES_PATH");
}

// ============================================================================
// Group 6: Advanced Stress, Evasion, and Concurrency Tests
// ============================================================================

#[test]
fn test_placeholder_deduplication_and_multiple_occurrences() {
    let expr = "<FIELD> + <FIELD> - <FIELD>";
    let tokens = PlaceholderRegistry::extract_placeholders(expr);
    assert_eq!(
        tokens.len(),
        1,
        "extract_placeholders should deduplicate tokens"
    );
    assert_eq!(tokens[0], "<FIELD>");

    let mut bindings = HashMap::new();
    bindings.insert("<FIELD>", "mock_volume");
    let rendered = PlaceholderRegistry::render_template(expr, &bindings).unwrap();
    assert_eq!(rendered, "mock_volume + mock_volume - mock_volume");
}

#[test]
fn test_placeholder_lowercase_and_syntax_evasion() {
    // Empirical check: lowercase <field> is NOT detected as placeholder
    let lowercase_expr = "rank(<field>)";
    assert!(!PlaceholderRegistry::has_unresolved_placeholders(
        lowercase_expr
    ));
    // And validate_fastexpr_syntax passes because <field> is ignored by extract_placeholders
    let syntax_res = PlaceholderRegistry::validate_fastexpr_syntax(lowercase_expr);
    assert!(
        syntax_res.is_ok(),
        "Lowercase placeholders bypass uppercase placeholder extraction"
    );
}

#[test]
fn test_fast_expr_validation_densify_false_positive_evasion() {
    // group_neutralize without densify around grouping variable, but densify appears elsewhere in comments or expressions
    let tricked_expr = "group_neutralize(rank(close), subindustry) // densify(dummy)";
    // The current implementation uses `lower.contains(\"densify(\")`
    let res = PlaceholderRegistry::validate_fastexpr_syntax(tricked_expr);
    // Empirical check: because \"densify(\" is in the comment, validate_fastexpr_syntax considers it valid
    assert!(
        res.is_ok(),
        "Global densify substring search can be satisfied by comment or unrelated token"
    );
}

#[test]
fn test_multithreaded_concurrency_stress() {
    let mut handles = Vec::new();
    for thread_id in 0..8 {
        let handle = thread::spawn(move || {
            let expr = format!("sig_{} = rank(<CORE_FIELD>) + <BETA_FIELD>; group_neutralize(sig_{}, densify(<GROUP>))", thread_id, thread_id);
            for _ in 0..100 {
                let extracted = PlaceholderRegistry::extract_placeholders(&expr);
                assert_eq!(extracted.len(), 3);

                let mut bindings = HashMap::new();
                bindings.insert("<CORE_FIELD>", "mock_close");
                bindings.insert("<BETA_FIELD>", "mock_beta");
                bindings.insert("<GROUP>", "subindustry");

                let rendered = PlaceholderRegistry::render_template(&expr, &bindings).unwrap();
                let valid = PlaceholderRegistry::validate_fastexpr_syntax(&rendered);
                assert!(valid.is_ok());
            }
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }
}
