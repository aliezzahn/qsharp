// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#![allow(clippy::needless_raw_string_hashes)]

use super::common::{
    check, MINIMAL, USE_DYNAMICALLY_SIZED_ARRAY, USE_DYNAMIC_BOOLEAN, USE_DYNAMIC_DOUBLE,
    USE_DYNAMIC_INT,
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
                        lo: 95,
                        hi: 116,
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
                        lo: 95,
                        hi: 124,
                    },
                ),
                UseOfDynamicInt(
                    Span {
                        lo: 95,
                        hi: 124,
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
                        lo: 95,
                        hi: 128,
                    },
                ),
                UseOfDynamicDouble(
                    Span {
                        lo: 95,
                        hi: 128,
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
                        lo: 95,
                        hi: 124,
                    },
                ),
                UseOfDynamicInt(
                    Span {
                        lo: 95,
                        hi: 124,
                    },
                ),
                UseOfDynamicBool(
                    Span {
                        lo: 137,
                        hi: 159,
                    },
                ),
                UseOfDynamicInt(
                    Span {
                        lo: 137,
                        hi: 159,
                    },
                ),
                UseOfDynamicallySizedArray(
                    Span {
                        lo: 137,
                        hi: 159,
                    },
                ),
            ]
        "#]],
    );
}
