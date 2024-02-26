// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use crate::{
    circuit::{Circuit, Operation, Register},
    Config,
};
use log::debug;
use num_bigint::BigUint;
use num_complex::Complex;
use qsc_data_structures::index_map::IndexMap;
use qsc_eval::{backend::Backend, val::Value};
use rustc_hash::FxHashSet;

#[derive(Copy, Clone, Default)]
struct HardwareId(usize);

pub struct Builder<T> {
    next_meas_id: usize,
    next_qubit_id: usize,
    next_qubit_hardware_id: HardwareId,
    qubit_map: IndexMap<usize, HardwareId>,
    circuit: Circuit,
    measurements: Vec<(Qubit, Res)>,
    real_backend: Option<T>,
    config: Config,
}

impl<T> Builder<T>
where
    T: Backend,
{
    #[must_use]
    pub fn new(config: Config) -> Self {
        Builder {
            next_meas_id: 0,
            next_qubit_id: 0,
            next_qubit_hardware_id: HardwareId::default(),
            qubit_map: IndexMap::new(),
            circuit: Circuit::default(),
            measurements: Vec::new(),
            config,
            real_backend: None,
        }
    }

    #[must_use]
    pub fn with_backend(mut config: Config, real_backend: T) -> Self {
        // TODO: I don't think it ever makes sense to
        // disallow qubit reuse when using in conjuction with the
        // real simulator
        config.no_qubit_reuse = false;
        Builder {
            next_meas_id: 0,
            next_qubit_id: 0,
            next_qubit_hardware_id: HardwareId::default(),
            qubit_map: IndexMap::new(),
            circuit: Circuit::default(),
            measurements: Vec::new(),
            config,
            real_backend: Some(real_backend),
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> Circuit {
        debug!("taking circuit snapshot");
        let mut circuit = self.circuit.clone();
        populate_wires_from_children(&mut circuit.operations);

        if self.config.no_qubit_reuse {
            let by_qubit = self.measurements.iter().fold(
                IndexMap::default(),
                |mut map: IndexMap<usize, Vec<Res>>, (q, r)| {
                    match map.get_mut(q.0 .0) {
                        Some(rs) => rs.push(*r),
                        None => {
                            map.insert(q.0 .0, vec![*r]);
                        }
                    }
                    map
                },
            );

            for (qubit, results) in &by_qubit {
                for result in results {
                    circuit.operations.push(measurement_gate(qubit, result.0));
                }
            }
        }

        // qubits

        for i in 0..self.next_qubit_hardware_id.0 {
            let num_measurements = self.measurements.iter().filter(|m| m.0 .0 .0 == i).count();
            circuit.qubits.push(crate::circuit::Qubit {
                id: i,
                num_children: num_measurements,
            });
        }

        circuit
    }

    #[must_use]
    pub fn finish(mut self, _val: &Value) -> Circuit {
        populate_wires_from_children(&mut self.circuit.operations);

        if self.config.no_qubit_reuse {
            let by_qubit = self.measurements.iter().fold(
                IndexMap::default(),
                |mut map: IndexMap<usize, Vec<Res>>, (q, r)| {
                    match map.get_mut(q.0 .0) {
                        Some(rs) => rs.push(*r),
                        None => {
                            map.insert(q.0 .0, vec![*r]);
                        }
                    }
                    map
                },
            );

            for (qubit, results) in &by_qubit {
                for result in results {
                    self.push_gate(measurement_gate(qubit, result.0));
                }
            }
        }

        // qubits

        for i in 0..self.next_qubit_hardware_id.0 {
            let num_measurements = self.measurements.iter().filter(|m| m.0 .0 .0 == i).count();
            self.circuit.qubits.push(crate::circuit::Qubit {
                id: i,
                num_children: num_measurements,
            });
        }

        self.circuit
    }

    #[must_use]
    fn get_meas_id(&mut self) -> usize {
        let id = self.next_meas_id;
        self.next_meas_id += 1;
        id
    }

    fn map(&mut self, qubit: usize) -> HardwareId {
        if let Some(mapped) = self.qubit_map.get(qubit) {
            *mapped
        } else {
            let mapped = self.next_qubit_hardware_id;
            self.next_qubit_hardware_id.0 += 1;
            self.qubit_map.insert(qubit, mapped);
            mapped
        }
    }

    fn push_gate(&mut self, gate: Operation) {
        let operations = &mut self.circuit.operations;

        operations.push(gate);
        // let operations = if self.current_boxes.is_some() {
        //     debug!("pushing gate {} into box", gate.gate);
        //     &mut self
        //         .circuit
        //         .operations
        //         .last_mut()
        //         .expect("expected an operation to be in the list")
        //         .children
        // } else {
        //     debug!("pushing gate {} at top level", gate.gate);
        //     &mut self.circuit.operations
        // };
        // operations.push(gate);
    }
}

fn populate_wires_from_children(operations: &mut [Operation]) {
    for operation in operations {
        let mut operation_targets = FxHashSet::<Register>::default();
        let mut operation_controls = FxHashSet::<Register>::default();
        if !operation.children.is_empty() {
            populate_wires_from_children(&mut operation.children);
            for child in &operation.children {
                for target in &child.targets {
                    operation_targets.insert(target.clone());
                }
                for control in &child.controls {
                    operation_controls.insert(control.clone());
                }
            }
            operation.targets = operation_targets.into_iter().collect();
            operation.controls = operation_controls.into_iter().collect();
        }
    }
}

fn gate<const N: usize>(name: &str, targets: [Qubit; N]) -> Operation {
    // {
    //     "gate": "H",
    //     "targets": [{ "qId": 0 }],
    // }
    Operation {
        gate: name.into(),
        display_args: None,
        is_controlled: false,
        is_adjoint: false,
        is_measurement: false,
        controls: vec![],
        targets: targets
            .iter()
            .map(|q| Register {
                r#type: 0,
                q_id: q.0 .0,
                c_id: None,
            })
            .collect(),
        children: vec![],
    }
}

fn adjoint_gate<const N: usize>(name: &str, targets: [Qubit; N]) -> Operation {
    Operation {
        gate: name.into(),
        display_args: None,
        is_controlled: false,
        is_adjoint: true,
        is_measurement: false,
        controls: vec![],
        targets: targets
            .iter()
            .map(|q| Register {
                r#type: 0,
                q_id: q.0 .0,
                c_id: None,
            })
            .collect(),
        children: vec![],
    }
}

fn controlled_gate<const M: usize, const N: usize>(
    name: &str,
    controls: [Qubit; M],
    targets: [Qubit; N],
) -> Operation {
    // {
    //     "gate": "X",
    //     "isControlled": "True",
    //     "controls": [{ "qId": 0 }],
    //     "targets": [{ "qId": 1 }],
    // }

    Operation {
        gate: name.into(),
        display_args: None,
        is_controlled: true,
        is_adjoint: false,
        is_measurement: false,
        controls: controls
            .iter()
            .map(|q| Register {
                r#type: 0,
                q_id: q.0 .0,
                c_id: None,
            })
            .collect(),
        targets: targets
            .iter()
            .map(|q| Register {
                r#type: 0,
                q_id: q.0 .0,
                c_id: None,
            })
            .collect(),
        children: vec![],
    }
}

fn measurement_gate(qubit: usize, _result: usize) -> Operation {
    // {
    //     "gate": "Measure",
    //     "isMeasurement": "True",
    //     "controls": [{ "qId": 1 }],
    //     "targets": [{ "type": 1, "qId": 1, "cId": 0 }],
    // }

    Operation {
        gate: "Measure".into(),
        display_args: None,
        is_controlled: false,
        is_adjoint: false,
        is_measurement: true,
        controls: vec![Register {
            r#type: 0,
            q_id: qubit,
            c_id: None,
        }],
        targets: vec![Register {
            r#type: 1,
            q_id: qubit,
            c_id: Some(0), // dunno why but quantum-viz wants this to be zero always
        }],
        children: vec![],
    }
}

fn rotation_gate<const N: usize>(name: &str, theta: f64, targets: [Qubit; N]) -> Operation {
    Operation {
        gate: name.into(),
        display_args: Some(format!("{theta:.4}")),
        is_controlled: false,
        is_adjoint: false,
        is_measurement: false,
        controls: vec![],
        targets: targets
            .iter()
            .map(|q| Register {
                r#type: 0,
                q_id: q.0 .0,
                c_id: None,
            })
            .collect(),
        children: vec![],
    }
}

impl<T> Backend for Builder<T>
where
    T: Backend,
    T::ResultType: Default + Into<bool> + Copy,
{
    type ResultType = T::ResultType;

    fn ccx(&mut self, ctl0: usize, ctl1: usize, q: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.ccx(ctl0, ctl1, q);
        }

        let ctl0 = self.map(ctl0);
        let ctl1 = self.map(ctl1);
        let q = self.map(q);

        self.push_gate(controlled_gate(
            "CX",
            [Qubit(ctl0), Qubit(ctl1)],
            [Qubit(q)],
        ));
    }

    fn cx(&mut self, ctl: usize, q: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.cx(ctl, q);
        }

        let ctl = self.map(ctl);
        let q = self.map(q);
        self.push_gate(controlled_gate("X", [Qubit(ctl)], [Qubit(q)]));
    }

    fn cy(&mut self, ctl: usize, q: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.cy(ctl, q);
        }

        let ctl = self.map(ctl);
        let q = self.map(q);
        self.push_gate(controlled_gate("Y", [Qubit(ctl)], [Qubit(q)]));
    }

    fn cz(&mut self, ctl: usize, q: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.cz(ctl, q);
        }

        let ctl = self.map(ctl);
        let q = self.map(q);
        self.push_gate(controlled_gate("Z", [Qubit(ctl)], [Qubit(q)]));
    }

    fn h(&mut self, q: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.h(q);
        }

        let q = self.map(q);
        self.push_gate(gate("H", [Qubit(q)]));
    }

    fn m(&mut self, q: usize) -> Self::ResultType {
        let result = self
            .real_backend
            .as_mut()
            .map(|s| s.m(q))
            .unwrap_or_default();

        let mapped_q = self.map(q);
        let id = self.get_meas_id();

        if self.config.no_qubit_reuse {
            // defer the measurement and reset the qubit
            self.measurements.push((Qubit(mapped_q), Res(id)));
            self.qubit_map.remove(q);
        } else {
            self.push_gate(measurement_gate(mapped_q.0, id));
        }

        result
    }

    fn mresetz(&mut self, q: usize) -> Self::ResultType {
        let result = self
            .real_backend
            .as_mut()
            .map(|s| s.mresetz(q))
            .unwrap_or_default();

        let mapped_q = self.map(q);
        let id = self.get_meas_id();

        if self.config.no_qubit_reuse {
            // defer the measurement and reset the qubit
            self.measurements.push((Qubit(mapped_q), Res(id)));
            self.qubit_map.remove(q);
        } else {
            self.push_gate(measurement_gate(mapped_q.0, id));
            if result.into() {
                self.push_gate(gate("X", [Qubit(mapped_q)]));
            }
        }

        result
    }

    fn reset(&mut self, q: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.reset(q);
        }

        if self.config.no_qubit_reuse {
            // Reset is a no-op in Base Profile, but does force qubit remapping so that future
            // operations on the given qubit id are performed on a fresh qubit. Clear the entry in the map
            // so it is known to require remapping on next use.
            self.qubit_map.remove(q);
        } else if let Some(ref mut s) = self.real_backend {
            let result = s.m(q);
            let mapped_q = self.map(q);
            let id = self.get_meas_id();

            self.push_gate(measurement_gate(mapped_q.0, id));
            if result.into() {
                self.push_gate(gate("X", [Qubit(mapped_q)]));
            }
        }
    }

    fn rx(&mut self, theta: f64, q: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.rx(theta, q);
        }
        let q = self.map(q);
        self.push_gate(rotation_gate("rx", theta, [Qubit(q)]));
    }

    fn rxx(&mut self, theta: f64, q0: usize, q1: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.rxx(theta, q0, q1);
        }
        let q0 = self.map(q0);
        let q1 = self.map(q1);
        self.push_gate(rotation_gate("rxx", theta, [Qubit(q0), Qubit(q1)]));
    }

    fn ry(&mut self, theta: f64, q: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.ry(theta, q);
        }
        let q = self.map(q);
        self.push_gate(rotation_gate("ry", theta, [Qubit(q)]));
    }

    fn ryy(&mut self, theta: f64, q0: usize, q1: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.ryy(theta, q0, q1);
        }
        let q0 = self.map(q0);
        let q1 = self.map(q1);
        self.push_gate(rotation_gate("ryy", theta, [Qubit(q0), Qubit(q1)]));
    }

    fn rz(&mut self, theta: f64, q: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.rz(theta, q);
        }
        let q = self.map(q);
        self.push_gate(rotation_gate("rz", theta, [Qubit(q)]));
    }

    fn rzz(&mut self, theta: f64, q0: usize, q1: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.rzz(theta, q0, q1);
        }
        let q0 = self.map(q0);
        let q1 = self.map(q1);
        self.push_gate(rotation_gate("rzz", theta, [Qubit(q0), Qubit(q1)]));
    }

    fn sadj(&mut self, q: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.sadj(q);
        }
        let q = self.map(q);
        self.push_gate(adjoint_gate("S", [Qubit(q)]));
    }

    fn s(&mut self, q: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.s(q);
        }
        let q = self.map(q);
        self.push_gate(gate("S", [Qubit(q)]));
    }

    fn swap(&mut self, q0: usize, q1: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.swap(q0, q1);
        }
        let q0 = self.map(q0);
        let q1 = self.map(q1);
        self.push_gate(gate("SWAP", [Qubit(q0), Qubit(q1)]));
    }

    fn tadj(&mut self, q: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.tadj(q);
        }
        let q = self.map(q);
        self.push_gate(adjoint_gate("T", [Qubit(q)]));
    }

    fn t(&mut self, q: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.t(q);
        }
        let q = self.map(q);
        self.push_gate(gate("T", [Qubit(q)]));
    }

    fn x(&mut self, q: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.x(q);
        }
        let q = self.map(q);
        self.push_gate(gate("X", [Qubit(q)]));
    }

    fn y(&mut self, q: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.y(q);
        }
        let q = self.map(q);
        self.push_gate(gate("Y", [Qubit(q)]));
    }

    fn z(&mut self, q: usize) {
        if let Some(ref mut s) = self.real_backend {
            s.z(q);
        }
        let q = self.map(q);
        self.push_gate(gate("Z", [Qubit(q)]));
    }

    fn qubit_allocate(&mut self) -> usize {
        let id = if let Some(ref mut s) = self.real_backend {
            s.qubit_allocate()
        } else {
            let id = self.next_qubit_id;
            self.next_qubit_id += 1;
            id
        };
        let _ = self.map(id);
        debug!("allocated qubit ${id}");
        id
    }

    fn qubit_release(&mut self, q: usize) {
        debug!("releasing qubit ${q}");
        if let Some(ref mut s) = self.real_backend {
            s.qubit_release(q);
        } else {
            self.next_qubit_id -= 1;
        }
    }

    fn capture_quantum_state(&mut self) -> (Vec<(BigUint, Complex<f64>)>, usize) {
        if let Some(sim) = self.real_backend.as_mut() {
            sim.capture_quantum_state()
        } else {
            (Vec::new(), 0)
        }
    }

    fn qubit_is_zero(&mut self, q: usize) -> bool {
        // Because `qubit_is_zero` is called on every qubit release, this must return
        // true to avoid a panic.
        self.real_backend
            .as_mut()
            .map_or(true, |s| s.qubit_is_zero(q))
    }

    fn custom_intrinsic(&mut self, name: &str, arg: Value) -> Option<Result<Value, String>> {
        if let Some(sim) = self.real_backend.as_mut() {
            sim.custom_intrinsic(name, arg)
        } else {
            None
        }
    }

    fn set_seed(&mut self, seed: Option<u64>) {
        if let Some(sim) = self.real_backend.as_mut() {
            sim.set_seed(seed);
        }
    }
}

#[derive(Copy, Clone)]
struct Qubit(HardwareId);

#[derive(Copy, Clone)]
struct Res(usize);
