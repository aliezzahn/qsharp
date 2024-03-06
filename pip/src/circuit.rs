// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use pyo3::{
    prelude::*,
    types::{PyDict, PyList},
};
use qsc::circuit::{Circuit, Operation};

#[pymethods]
/// An output returned from the Q# interpreter.
/// Outputs can be a state dumps or messages. These are normally printed to the console.
impl PyCircuit {
    fn __repr__(&self) -> String {
        self.0.to_string()
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn get_dict(&self, py: Python) -> Py<PyDict> {
        let dict = PyDict::new(py);
        let _ = dict.set_item(
            "operations",
            PyList::new(
                py,
                self.0
                    .operations
                    .iter()
                    .map(|o| PyOperation(o.clone()).into_py(py)),
            ),
        );
        let _ = dict.set_item(
            "qubits",
            PyList::new(
                py,
                self.0.qubits.iter().map(|q| {
                    let qubit = PyDict::new(py);
                    let _ = qubit.set_item("id", q.id);
                    let _ = qubit.set_item("numChildren", q.num_children);
                    qubit
                }),
            ),
        );
        dict.into_py(py)
    }
}

#[pyclass(unsendable)]
pub(crate) struct PyCircuit(pub Circuit);

struct PyOperation(Operation);

impl IntoPy<PyObject> for PyOperation {
    fn into_py(self, py: Python<'_>) -> PyObject {
        let gate = PyDict::new(py);
        let _ = gate.set_item("gate", self.0.gate);
        let _ = gate.set_item("displayArgs", self.0.display_args);
        let _ = gate.set_item("isControlled", self.0.is_controlled);
        let _ = gate.set_item("isAdjoint", self.0.is_adjoint);
        let _ = gate.set_item("isMeasurement", self.0.is_measurement);
        let _ = gate.set_item(
            "controls",
            PyList::new(
                py,
                self.0.controls.into_iter().map(|r| {
                    let register = PyDict::new(py);
                    let _ = register.set_item("qId", r.q_id);
                    let _ = register.set_item("type", r.r#type);
                    if let Some(c_id) = r.c_id {
                        let _ = register.set_item("cId", c_id);
                    }
                    register
                }),
            ),
        );
        let _ = gate.set_item(
            "targets",
            PyList::new(
                py,
                self.0.targets.into_iter().map(|r| {
                    let register = PyDict::new(py);
                    let _ = register.set_item("qId", r.q_id);
                    let _ = register.set_item("type", r.r#type);
                    if let Some(c_id) = r.c_id {
                        let _ = register.set_item("cId", c_id);
                    }
                    register
                }),
            ),
        );
        let _ = gate.set_item(
            "children",
            PyList::new(
                py,
                self.0
                    .children
                    .into_iter()
                    .map(|g| PyOperation(g).into_py(py)),
            ),
        );
        gate.into_py(py)
    }
}
