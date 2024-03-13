// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#[cfg(test)]
mod tests;

use miette::Diagnostic;
use qsc_data_structures::span::Span;
use qsc_fir::fir::{PackageId, PackageStore};
use qsc_rca::PackageStoreComputeProperties;
use thiserror::Error;

#[derive(Clone, Debug, Diagnostic, Error)]
pub enum Error {
    #[error("cannot branch based on a dynamic condition")]
    #[diagnostic(help(
        "branching based on a dynamic condition, a condition that depends on a measurement result, is not supported by the target"
    ))]
    #[diagnostic(code("Qsc.CapabilitiesCk.BranchingOnDynamicCondition"))]
    BranchingOnDynamicCondition(#[label] Span),
}

#[must_use]
pub fn check_supported_capabilities(
    _package_id: PackageId,
    _package_store: &PackageStore,
    _compute_properties: &PackageStoreComputeProperties,
) -> Vec<Error> {
    Vec::new()
}
