use alphafind::autotuner::{replace_last_signed_power_exponent, switch_neutralization_in_expr};

#[test]
fn test_replace_last_signed_power_exponent() {
    let expr =
        "signal = rank(close); group_neutralize(signed_power(signal, 1.5), densify(subindustry))";
    let updated = replace_last_signed_power_exponent(expr, 4.4);
    assert!(updated.is_some());
    let res = updated.unwrap();
    assert!(res.contains("signed_power(signal, 4.4)"));
}

#[test]
fn test_replace_last_signed_power_exponent_nested() {
    let expr = "sig1 = signed_power(rank(close) - 0.5, 1.2); group_neutralize(signed_power(sig1, 2.0), densify(market))";
    let updated = replace_last_signed_power_exponent(expr, 3.8);
    assert!(updated.is_some());
    let res = updated.unwrap();
    assert!(res.contains("signed_power(sig1, 3.8)"));
    assert!(res.contains("signed_power(rank(close) - 0.5, 1.2)"));
}

#[test]
fn test_switch_neutralization_subindustry() {
    let expr = "group_neutralize(signal, densify(market))";
    let switched = switch_neutralization_in_expr(expr, "SUBINDUSTRY");
    assert_eq!(switched, "group_neutralize(signal, densify(subindustry))");
}

#[test]
fn test_switch_neutralization_market() {
    let expr = "group_neutralize(signal, densify(subindustry))";
    let switched = switch_neutralization_in_expr(expr, "MARKET");
    assert_eq!(switched, "group_neutralize(signal, densify(market))");
}

#[test]
fn test_switch_neutralization_sector() {
    let expr = "group_neutralize(signal, densify(subindustry))";
    let switched = switch_neutralization_in_expr(expr, "SECTOR");
    assert_eq!(switched, "group_neutralize(signal, densify(sector))");
}
