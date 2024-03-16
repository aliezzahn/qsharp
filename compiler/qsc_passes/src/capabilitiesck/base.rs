// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#![allow(clippy::needless_raw_string_hashes)]

use super::common::{
    check, CALL_CICLYC_FUNCTION_WITH_CLASSICAL_ARGUMENT,
    CALL_CICLYC_FUNCTION_WITH_DYNAMIC_ARGUMENT, MINIMAL, USE_DYNAMICALLY_SIZED_ARRAY,
    USE_DYNAMIC_BOOLEAN, USE_DYNAMIC_DOUBLE, USE_DYNAMIC_INT, USE_DYNAMIC_PAULI, USE_DYNAMIC_RANGE,
};
use expect_test::{expect, Expect};
use qsc_frontend::compile::RuntimeCapabilityFlags;

fn check_profile(source: &str, expect: &Expect) {
    check(source, expect, RuntimeCapabilityFlags::empty());
}

#[test]
fn minimal_program_yields_no_errors() {
    check_profile(
        MINIMAL,
        &expect![[r#"
            []
        "#]],
    );
}

#[test]
fn use_of_dynamic_boolean_yields_error() {
    check_profile(
        USE_DYNAMIC_BOOLEAN,
        &expect![[r#"
            [
                UseOfDynamicBool(
                    Span {
                        lo: 96,
                        hi: 117,
                    },
                ),
            ]
        "#]],
    );
}

#[test]
fn use_of_dynamic_int_yields_errors() {
    check_profile(
        USE_DYNAMIC_INT,
        &expect![[r#"
            [
                UseOfDynamicBool(
                    Span {
                        lo: 96,
                        hi: 125,
                    },
                ),
                UseOfDynamicInt(
                    Span {
                        lo: 96,
                        hi: 125,
                    },
                ),
            ]
        "#]],
    );
}

#[test]
fn use_of_dynamic_pauli_yields_errors() {
    check_profile(
        USE_DYNAMIC_PAULI,
        &expect![[r#"
            [
                UseOfDynamicBool(
                    Span {
                        lo: 96,
                        hi: 135,
                    },
                ),
                UseOfDynamicPauli(
                    Span {
                        lo: 96,
                        hi: 135,
                    },
                ),
            ]
        "#]],
    );
}

#[ignore = "work in progreess"]
#[test]
fn use_of_dynamic_range_yields_errors() {
    check_profile(
        USE_DYNAMIC_RANGE,
        &expect![[r#"
            [
                UseOfDynamicBool(
                    Span {
                        lo: 96,
                        hi: 135,
                    },
                ),
                UseOfDynamicInt(
                    Span {
                        lo: 96,
                        hi: 135,
                    },
                ),
                UseOfDynamicRange(
                    Span {
                        lo: 96,
                        hi: 135,
                    },
                ),
            ]
        "#]],
    );
}

#[test]
fn use_of_dynamic_double_yields_errors() {
    check_profile(
        USE_DYNAMIC_DOUBLE,
        &expect![[r#"
            [
                UseOfDynamicBool(
                    Span {
                        lo: 96,
                        hi: 129,
                    },
                ),
                UseOfDynamicDouble(
                    Span {
                        lo: 96,
                        hi: 129,
                    },
                ),
            ]
        "#]],
    );
}

#[test]
fn use_of_dynamically_sized_array_yields_errors() {
    check_profile(
        USE_DYNAMICALLY_SIZED_ARRAY,
        &expect![[r#"
            [
                UseOfDynamicBool(
                    Span {
                        lo: 96,
                        hi: 125,
                    },
                ),
                UseOfDynamicInt(
                    Span {
                        lo: 96,
                        hi: 125,
                    },
                ),
                UseOfDynamicBool(
                    Span {
                        lo: 138,
                        hi: 160,
                    },
                ),
                UseOfDynamicInt(
                    Span {
                        lo: 138,
                        hi: 160,
                    },
                ),
                UseOfDynamicallySizedArray(
                    Span {
                        lo: 138,
                        hi: 160,
                    },
                ),
            ]
        "#]],
    );
}

#[test]
fn call_cyclic_function_with_classical_argument_yields_no_errors() {
    check_profile(
        CALL_CICLYC_FUNCTION_WITH_CLASSICAL_ARGUMENT,
        &expect![[r#"
            []
        "#]],
    );
}

#[test]
fn call_cyclic_function_with_dynamic_argument_yields_errors() {
    check_profile(
        CALL_CICLYC_FUNCTION_WITH_DYNAMIC_ARGUMENT,
        &expect![[r#"
            [
                UseOfDynamicBool(
                    Span {
                        lo: 201,
                        hi: 232,
                    },
                ),
                UseOfDynamicInt(
                    Span {
                        lo: 201,
                        hi: 232,
                    },
                ),
                UseOfDynamicBool(
                    Span {
                        lo: 241,
                        hi: 263,
                    },
                ),
                UseOfDynamicInt(
                    Span {
                        lo: 241,
                        hi: 263,
                    },
                ),
                CyclicFunctionUsesDynamicArg(
                    Span {
                        lo: 241,
                        hi: 263,
                    },
                ),
            ]
        "#]],
    );
}
