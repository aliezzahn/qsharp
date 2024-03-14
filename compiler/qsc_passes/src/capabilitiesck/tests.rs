// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#![allow(clippy::needless_raw_string_hashes)]

use expect_test::{expect, Expect};

use crate::capabilitiesck::check_supported_capabilities;
use qsc::{incremental::Compiler, PackageType};
use qsc_data_structures::language_features::LanguageFeatures;
use qsc_eval::{debug::map_hir_package_to_fir, lower::Lowerer};
use qsc_fir::fir::{Package, PackageId, PackageStore};
use qsc_frontend::compile::{PackageStore as HirPackageStore, RuntimeCapabilityFlags, SourceMap};
use qsc_rca::{Analyzer, PackageComputeProperties, PackageStoreComputeProperties};

struct CompilationContext {
    fir_store: PackageStore,
    compute_properties: PackageStoreComputeProperties,
    package_id: PackageId,
}

impl CompilationContext {
    fn new(source: &str) -> Self {
        let mut compiler = Compiler::new(
            true,
            SourceMap::default(),
            PackageType::Lib,
            RuntimeCapabilityFlags::all(),
            LanguageFeatures::default(),
        )
        .expect("should be able to create a new compiler");
        let package_id = map_hir_package_to_fir(compiler.package_id());
        let increment = compiler
            .compile_fragments_fail_fast("test", source)
            .expect("code should compile");
        compiler.update(increment);
        let mut lowerer = Lowerer::new();
        let fir_store = lower_hir_package_store(&mut lowerer, compiler.package_store());
        let analyzer = Analyzer::init(&fir_store);
        let compute_properties = analyzer.analyze_all();
        Self {
            fir_store,
            compute_properties,
            package_id,
        }
    }

    fn get_package_compute_properties_tuple(&self) -> (&Package, &PackageComputeProperties) {
        (
            self.fir_store.get(self.package_id),
            self.compute_properties.get(self.package_id),
        )
    }
}

fn lower_hir_package_store(
    lowerer: &mut Lowerer,
    hir_package_store: &HirPackageStore,
) -> PackageStore {
    let mut fir_store = PackageStore::new();
    for (id, unit) in hir_package_store {
        fir_store.insert(
            map_hir_package_to_fir(id),
            lowerer.lower_package(&unit.package),
        );
    }
    fir_store
}

fn check(source: &str, expect: &Expect) {
    let compilation_context = CompilationContext::new(source);
    let (package, compute_properties) = compilation_context.get_package_compute_properties_tuple();
    let capabilities =
        RuntimeCapabilityFlags::ForwardBranching | RuntimeCapabilityFlags::IntegerComputations;
    let errors = check_supported_capabilities(package, compute_properties, capabilities);
    expect.assert_debug_eq(&errors);
}

#[test]
fn use_of_dynamic_double_yields_error() {
    check(
        r#"
        namespace Foo {
            operation Bar() : Unit {
                use q = Qubit();
                let r = M(q);
                let d = r == Zero ? 0.0 | 1.0;
            }
        }"#,
        &expect![[r#"
            [
                UseOfDynamicDouble(
                    Span {
                        lo: 141,
                        hi: 171,
                    },
                ),
            ]
        "#]],
    );
}

#[test]
fn use_of_dynamically_sized_array_yields_error() {
    check(
        r#"
        namespace Foo {
            operation Bar() : Unit {
                use q = Qubit();
                let s = M(q) == Zero ? 1 | 2;
                let a = [1, size = s];
            }
        }"#,
        &expect![[r#"
            [
                UseOfDynamicallySizedArray(
                    Span {
                        lo: 157,
                        hi: 179,
                    },
                ),
            ]
        "#]],
    );
}
