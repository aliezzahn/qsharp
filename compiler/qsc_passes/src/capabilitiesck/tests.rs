// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use expect_test::{expect, Expect};

use crate::capabilitiesck::check_supported_capabilities;
use indoc::indoc;
use qsc::{incremental::Compiler, PackageType};
use qsc_data_structures::language_features::LanguageFeatures;
use qsc_eval::{debug::map_hir_package_to_fir, lower::Lowerer};
use qsc_fir::fir::{PackageId, PackageStore};
use qsc_frontend::compile::{
    PackageStore as HirPackageStore, RuntimeCapabilityFlags, SourceContents, SourceMap, SourceName,
};
use qsc_rca::{Analyzer, PackageStoreComputeProperties};

struct CompilationContext {
    fir_store: PackageStore,
    compute_properties: PackageStoreComputeProperties,
    package_id: PackageId,
}

impl CompilationContext {
    pub fn new(sources: impl IntoIterator<Item = (SourceName, SourceContents)>) -> Self {
        let compiler = Compiler::new(
            true,
            SourceMap::new(sources, None),
            PackageType::Lib,
            RuntimeCapabilityFlags::all(),
            LanguageFeatures::default(),
        )
        .expect("should be able to create a new compiler");
        let package_id = map_hir_package_to_fir(compiler.package_id());
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

fn check(sources: impl IntoIterator<Item = (SourceName, SourceContents)>, expect: &Expect) {
    let compilation_context = CompilationContext::new(sources);
    let errors = check_supported_capabilities(
        compilation_context.package_id,
        &compilation_context.fir_store,
        &compilation_context.compute_properties,
    );
    expect.assert_debug_eq(&errors);
}

#[test]
fn simple_program_is_valid() {
    let sources: [(SourceName, SourceContents); 1] = [(
        "test".into(),
        indoc! {"
            namespace Foo {
                function A() : Unit {}
            }
        "}
        .into(),
    )];

    check(
        sources,
        &expect![[r#"
            []
        "#]],
    );
}
