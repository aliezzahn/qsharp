// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#[cfg(test)]
mod base;

#[cfg(test)]
mod adaptive;

#[cfg(test)]
mod adaptive_plus_integers;

#[cfg(test)]
pub mod common;

use miette::Diagnostic;
use qsc_data_structures::span::Span;
use qsc_fir::{
    fir::{Block, BlockId, Expr, ExprId, Package, PackageLookup, Pat, PatId, Stmt, StmtId},
    visit::Visitor,
};
use qsc_frontend::compile::RuntimeCapabilityFlags;
use qsc_rca::{ComputeKind, PackageComputeProperties, RuntimeFeatureFlags};
use thiserror::Error;

#[derive(Clone, Debug, Diagnostic, Error)]
pub enum Error {
    #[error("cannot use a dynamic boolean")]
    #[diagnostic(help(
        "using a dynamic boolean, a boolean that depends on a measurement result, is not supported by the target"
    ))]
    #[diagnostic(code("Qsc.CapabilitiesCk.UseOfDynamicBool"))]
    UseOfDynamicBool(#[label] Span),

    #[error("cannot use a dynamic integer")]
    #[diagnostic(help(
        "using a dynamic integer, an integer that depends on a measurement result, is not supported by the target"
    ))]
    #[diagnostic(code("Qsc.CapabilitiesCk.UseOfDynamicInt"))]
    UseOfDynamicInt(#[label] Span),

    #[error("cannot use a dynamic double")]
    #[diagnostic(help(
        "using a dynamic double, a double that depends on a measurement result, is not supported by the target"
    ))]
    #[diagnostic(code("Qsc.CapabilitiesCk.UseOfDynamicDouble"))]
    UseOfDynamicDouble(#[label] Span),

    #[error("cannot use a dynamically-sized array")]
    #[diagnostic(help(
        "using a dynamically-sized array, an array whose size depends on a measurement result, is not supported by the target"
    ))]
    #[diagnostic(code("Qsc.CapabilitiesCk.UseOfDynamicallySizedArray"))]
    UseOfDynamicallySizedArray(#[label] Span),
}

#[must_use]
pub fn check_supported_capabilities(
    package: &Package,
    compute_properties: &PackageComputeProperties,
    capabilities: RuntimeCapabilityFlags,
) -> Vec<Error> {
    let checker = Checker {
        package,
        compute_properties,
        target_capabilities: capabilities,
        errors: Vec::new(),
    };

    checker.run()
}

struct Checker<'a> {
    package: &'a Package,
    compute_properties: &'a PackageComputeProperties,
    target_capabilities: RuntimeCapabilityFlags,
    errors: Vec<Error>,
}

impl<'a> Visitor<'a> for Checker<'a> {
    fn get_block(&self, id: BlockId) -> &'a Block {
        self.package.get_block(id)
    }

    fn get_expr(&self, id: ExprId) -> &'a Expr {
        self.package.get_expr(id)
    }

    fn get_pat(&self, id: PatId) -> &'a Pat {
        self.package.get_pat(id)
    }

    fn get_stmt(&self, id: StmtId) -> &'a Stmt {
        self.package.get_stmt(id)
    }

    fn visit_stmt(&mut self, stmt_id: StmtId) {
        let compute_kind = self
            .compute_properties
            .stmts
            .get(stmt_id)
            .expect("statement should exist in package compute properties")
            .inherent;

        let ComputeKind::Quantum(quantum_properties) = compute_kind else {
            return;
        };

        let runtime_capabilities = quantum_properties.runtime_features.runtime_capabilities();
        let missing_capabilities = !self.target_capabilities & runtime_capabilities;
        let missing_features = quantum_properties
            .runtime_features
            .contributing_features(missing_capabilities);
        let stmt = self.get_stmt(stmt_id);
        let mut stmt_errors = generate_errors_from_runtime_features(missing_features, stmt.span);
        self.errors.append(&mut stmt_errors);
    }
}

impl<'a> Checker<'a> {
    fn run(mut self) -> Vec<Error> {
        self.visit_package(self.package);
        self.errors
    }
}

fn generate_errors_from_runtime_features(
    runtime_features: RuntimeFeatureFlags,
    span: Span,
) -> Vec<Error> {
    let mut errors = Vec::<Error>::new();
    if runtime_features.contains(RuntimeFeatureFlags::UseOfDynamicBool) {
        errors.push(Error::UseOfDynamicBool(span));
    }
    if runtime_features.contains(RuntimeFeatureFlags::UseOfDynamicInt) {
        errors.push(Error::UseOfDynamicInt(span));
    }
    if runtime_features.contains(RuntimeFeatureFlags::UseOfDynamicDouble) {
        errors.push(Error::UseOfDynamicDouble(span));
    }
    if runtime_features.contains(RuntimeFeatureFlags::UseOfDynamicallySizedArray) {
        errors.push(Error::UseOfDynamicallySizedArray(span));
    }
    errors
}
