use crate::tests::{fail_test, run_test, TestResult};

#[test]
fn concrete_variable_assignment() -> TestResult {
    run_test(
        "let x = (1..100 | each { |y| $y + 100 }); let y = ($x | length); $x | length",
        "100",
    )
}

#[test]
fn proper_shadow() -> TestResult {
    run_test("let x = 10; let x = $x + 9; $x", "19")
}

#[test]
fn config_filesize_format_with_metric_true() -> TestResult {
    // Note: this tests both the config variable and that it is properly captured into a block
    run_test(
        r#"let config = {"filesize_metric": $true "filesize_format": "kib" }; do { 40kb | into string } "#,
        "39.1 KiB",
    )
}

#[test]
fn config_filesize_format_with_metric_false_kib() -> TestResult {
    // Note: this tests both the config variable and that it is properly captured into a block
    run_test(
        r#"let config = {"filesize_metric": $false "filesize_format": "kib" }; do { 40kb | into string } "#,
        "39.1 KiB",
    )
}

#[test]
fn config_filesize_format_with_metric_false_kb() -> TestResult {
    // Note: this tests both the config variable and that it is properly captured into a block
    run_test(
        r#"let config = {"filesize_metric": $false "filesize_format": "kb" }; do { 40kb | into string } "#,
        "40.0 KB",
    )
}

#[test]
fn in_variable_1() -> TestResult {
    run_test(r#"[3] | if $in.0 > 4 { "yay!" } else { "boo" }"#, "boo")
}

#[test]
fn in_variable_2() -> TestResult {
    run_test(r#"3 | if $in > 2 { "yay!" } else { "boo" }"#, "yay!")
}

#[test]
fn in_variable_3() -> TestResult {
    run_test(r#"3 | if $in > 4 { "yay!" } else { $in }"#, "3")
}

#[test]
fn in_variable_4() -> TestResult {
    run_test(r#"3 | do { $in }"#, "3")
}

#[test]
fn in_variable_5() -> TestResult {
    run_test(r#"3 | if $in > 2 { $in - 10 } else { $in * 10 }"#, "-7")
}

#[test]
fn in_variable_6() -> TestResult {
    run_test(r#"3 | if $in > 6 { $in - 10 } else { $in * 10 }"#, "30")
}

#[test]
fn help_works_with_missing_requirements() -> TestResult {
    run_test(r#"each --help | lines | length"#, "15")
}

#[test]
fn scope_variable() -> TestResult {
    run_test(r#"let x = 3; $scope.vars.'$x'"#, "int")
}

#[test]
fn earlier_errors() -> TestResult {
    fail_test(
        r#"[1, "bob"] | each { $it + 3 } | each { $it / $it } | table"#,
        "int",
    )
}

#[test]
fn missing_flags_are_nothing() -> TestResult {
    run_test(
        r#"def foo [--aaa(-a): int, --bbb(-b): int] { (if $aaa == $nothing { 10 } else { $aaa }) + (if $bbb == $nothing { 100 } else { $bbb }) }; foo"#,
        "110",
    )
}

#[test]
fn missing_flags_are_nothing2() -> TestResult {
    run_test(
        r#"def foo [--aaa(-a): int, --bbb(-b): int] { (if $aaa == $nothing { 10 } else { $aaa }) + (if $bbb == $nothing { 100 } else { $bbb }) }; foo -a 90"#,
        "190",
    )
}

#[test]
fn missing_flags_are_nothing3() -> TestResult {
    run_test(
        r#"def foo [--aaa(-a): int, --bbb(-b): int] { (if $aaa == $nothing { 10 } else { $aaa }) + (if $bbb == $nothing { 100 } else { $bbb }) }; foo -b 45"#,
        "55",
    )
}

#[test]
fn missing_flags_are_nothing4() -> TestResult {
    run_test(
        r#"def foo [--aaa(-a): int, --bbb(-b): int] { (if $aaa == $nothing { 10 } else { $aaa }) + (if $bbb == $nothing { 100 } else { $bbb }) }; foo -a 3 -b 10000"#,
        "10003",
    )
}

#[test]
fn proper_variable_captures() -> TestResult {
    run_test(
        r#"def foo [x] { let y = 100; { $y + $x } }; do (foo 23)"#,
        "123",
    )
}

#[test]
fn proper_variable_captures_with_calls() -> TestResult {
    run_test(
        r#"def foo [] { let y = 60; def bar [] { $y }; { bar } }; do (foo)"#,
        "60",
    )
}

#[test]
fn proper_variable_captures_with_nesting() -> TestResult {
    run_test(
        r#"def foo [x] { let z = 100; def bar [y] { $y - $x + $z } ; { |z| bar $z } }; do (foo 11) 13"#,
        "102",
    )
}

#[test]
fn proper_variable_for() -> TestResult {
    run_test(r#"for x in 1..3 { if $x == 2 { "bob" } } | get 1"#, "bob")
}

#[test]
fn divide_duration() -> TestResult {
    run_test(r#"4ms / 4ms"#, "1")
}

#[test]
fn divide_filesize() -> TestResult {
    run_test(r#"4mb / 4mb"#, "1")
}
