// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#[cfg(test)]
mod tests;

use num_bigint::BigUint;
use num_complex::Complex;
use qsc_data_structures::index_map::IndexMap;
use qsc_eval::{backend::Backend, val::Value};
use std::fmt::{Display, Write};

pub struct BaseProfSim {
    next_meas_id: usize,
    next_qubit_id: usize,
    next_qubit_hardware_id: usize,
    qubit_map: IndexMap<usize, usize>,
    instrs: String,
    measurements: String,
}

impl Default for BaseProfSim {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseProfSim {
    #[must_use]
    pub fn new() -> Self {
        let mut sim = BaseProfSim {
            next_meas_id: 0,
            next_qubit_id: 0,
            next_qubit_hardware_id: 0,
            qubit_map: IndexMap::new(),
            instrs: String::new(),
            measurements: String::new(),
        };
        sim.instrs.push_str(include_str!("./qir_base/prefix.ll"));
        sim
    }

    #[must_use]
    pub fn finish(mut self, val: &Value) -> String {
        self.instrs.push_str(&self.measurements);
        self.write_output_recording(val)
            .expect("writing to string should succeed");

        write!(
            self.instrs,
            include_str!("./qir_base/postfix.ll"),
            self.next_qubit_hardware_id, self.next_meas_id
        )
        .expect("writing to string should succeed");

        self.instrs
    }

    #[must_use]
    fn get_meas_id(&mut self) -> usize {
        let id = self.next_meas_id;
        self.next_meas_id += 1;
        id
    }

    fn map(&mut self, qubit: usize) -> usize {
        if let Some(mapped) = self.qubit_map.get(qubit) {
            *mapped
        } else {
            let mapped = self.next_qubit_hardware_id;
            self.next_qubit_hardware_id += 1;
            self.qubit_map.insert(qubit, mapped);
            mapped
        }
    }

    fn write_output_recording(&mut self, val: &Value) -> std::fmt::Result {
        match val {
            Value::Array(arr) => {
                self.write_array_recording(arr.len())?;
                for val in arr.iter() {
                    self.write_output_recording(val)?;
                }
            }
            Value::Result(r) => {
                self.write_result_recording(r.unwrap_id());
            }
            Value::Tuple(tup) => {
                self.write_tuple_recording(tup.len())?;
                for val in tup.iter() {
                    self.write_output_recording(val)?;
                }
            }
            _ => panic!("unexpected value type: {val:?}"),
        }
        Ok(())
    }

    fn write_result_recording(&mut self, res: usize) {
        writeln!(
            self.instrs,
            "  call void @__quantum__rt__result_record_output({}, i8* null)",
            Result(res),
        )
        .expect("writing to string should succeed");
    }

    fn write_tuple_recording(&mut self, size: usize) -> std::fmt::Result {
        writeln!(
            self.instrs,
            "  call void @__quantum__rt__tuple_record_output(i64 {size}, i8* null)"
        )
    }

    fn write_array_recording(&mut self, size: usize) -> std::fmt::Result {
        writeln!(
            self.instrs,
            "  call void @__quantum__rt__array_record_output(i64 {size}, i8* null)"
        )
    }
}

impl Backend for BaseProfSim {
    type ResultType = usize;

    fn ccx(&mut self, ctl0: usize, ctl1: usize, q: usize) {
        let ctl0 = self.map(ctl0);
        let ctl1 = self.map(ctl1);
        let q = self.map(q);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__ccx__body({}, {}, {})",
            Qubit(ctl0),
            Qubit(ctl1),
            Qubit(q)
        )
        .expect("writing to string should succeed");
    }

    fn cx(&mut self, ctl: usize, q: usize) {
        let ctl = self.map(ctl);
        let q = self.map(q);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__cx__body({}, {})",
            Qubit(ctl),
            Qubit(q),
        )
        .expect("writing to string should succeed");
    }

    fn cy(&mut self, ctl: usize, q: usize) {
        let ctl = self.map(ctl);
        let q = self.map(q);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__cy__body({}, {})",
            Qubit(ctl),
            Qubit(q),
        )
        .expect("writing to string should succeed");
    }

    fn cz(&mut self, ctl: usize, q: usize) {
        let ctl = self.map(ctl);
        let q = self.map(q);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__cz__body({}, {})",
            Qubit(ctl),
            Qubit(q),
        )
        .expect("writing to string should succeed");
    }

    fn h(&mut self, q: usize) {
        let q = self.map(q);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__h__body({})",
            Qubit(q),
        )
        .expect("writing to string should succeed");
    }

    fn m(&mut self, q: usize) -> Self::ResultType {
        let q = self.map(q);
        let id = self.get_meas_id();
        // Measurements are tracked separately from instructions, so that they can be
        // deferred until the end of the program.
        writeln!(
            self.measurements,
            "  call void @__quantum__qis__mz__body({}, {}) #1",
            Qubit(q),
            Result(id),
        )
        .expect("writing to string should succeed");
        self.reset(q);
        id
    }

    fn mresetz(&mut self, q: usize) -> Self::ResultType {
        self.m(q)
    }

    fn reset(&mut self, q: usize) {
        // Reset is a no-op in Base Profile, but does force qubit remapping so that future
        // operations on the given qubit id are performed on a fresh qubit. Clear the entry in the map
        // so it is known to require remapping on next use.
        self.qubit_map.remove(q);
    }

    fn rx(&mut self, theta: f64, q: usize) {
        let q = self.map(q);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__rx__body({}, {})",
            Double(theta),
            Qubit(q),
        )
        .expect("writing to string should succeed");
    }

    fn rxx(&mut self, theta: f64, q0: usize, q1: usize) {
        let q0 = self.map(q0);
        let q1 = self.map(q1);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__rxx__body({}, {}, {})",
            Double(theta),
            Qubit(q0),
            Qubit(q1),
        )
        .expect("writing to string should succeed");
    }

    fn ry(&mut self, theta: f64, q: usize) {
        let q = self.map(q);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__ry__body({}, {})",
            Double(theta),
            Qubit(q),
        )
        .expect("writing to string should succeed");
    }

    fn ryy(&mut self, theta: f64, q0: usize, q1: usize) {
        let q0 = self.map(q0);
        let q1 = self.map(q1);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__ryy__body({}, {}, {})",
            Double(theta),
            Qubit(q0),
            Qubit(q1),
        )
        .expect("writing to string should succeed");
    }

    fn rz(&mut self, theta: f64, q: usize) {
        let q = self.map(q);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__rz__body({}, {})",
            Double(theta),
            Qubit(q),
        )
        .expect("writing to string should succeed");
    }

    fn rzz(&mut self, theta: f64, q0: usize, q1: usize) {
        let q0 = self.map(q0);
        let q1 = self.map(q1);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__rzz__body({}, {}, {})",
            Double(theta),
            Qubit(q0),
            Qubit(q1),
        )
        .expect("writing to string should succeed");
    }

    fn sadj(&mut self, q: usize) {
        let q = self.map(q);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__s__adj({})",
            Qubit(q),
        )
        .expect("writing to string should succeed");
    }

    fn s(&mut self, q: usize) {
        let q = self.map(q);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__s__body({})",
            Qubit(q),
        )
        .expect("writing to string should succeed");
    }

    fn swap(&mut self, q0: usize, q1: usize) {
        let q0 = self.map(q0);
        let q1 = self.map(q1);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__swap__body({}, {})",
            Qubit(q0),
            Qubit(q1),
        )
        .expect("writing to string should succeed");
    }

    fn tadj(&mut self, q: usize) {
        let q = self.map(q);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__t__adj({})",
            Qubit(q),
        )
        .expect("writing to string should succeed");
    }

    fn t(&mut self, q: usize) {
        let q = self.map(q);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__t__body({})",
            Qubit(q),
        )
        .expect("writing to string should succeed");
    }

    fn x(&mut self, q: usize) {
        let q = self.map(q);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__x__body({})",
            Qubit(q),
        )
        .expect("writing to string should succeed");
    }

    fn y(&mut self, q: usize) {
        let q = self.map(q);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__y__body({})",
            Qubit(q),
        )
        .expect("writing to string should succeed");
    }

    fn z(&mut self, q: usize) {
        let q = self.map(q);
        writeln!(
            self.instrs,
            "  call void @__quantum__qis__z__body({})",
            Qubit(q),
        )
        .expect("writing to string should succeed");
    }

    fn qubit_allocate(&mut self) -> usize {
        let id = self.next_qubit_id;
        self.next_qubit_id += 1;
        let _ = self.map(id);
        id
    }

    fn qubit_release(&mut self, _q: usize) {
        self.next_qubit_id -= 1;
    }

    fn capture_quantum_state(&mut self) -> (Vec<(BigUint, Complex<f64>)>, usize) {
        (Vec::new(), 0)
    }

    fn qubit_is_zero(&mut self, _q: usize) -> bool {
        // Because `qubit_is_zero` is called on every qubit release, this must return
        // true to avoid a panic.
        true
    }

    fn reinit(&mut self) {
        *self = Self::default();
    }
}

struct Qubit(usize);

impl Display for Qubit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%Qubit* inttoptr (i64 {} to %Qubit*)", self.0)
    }
}

struct Result(usize);

impl Display for Result {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%Result* inttoptr (i64 {} to %Result*)", self.0)
    }
}

struct Double(f64);

impl Display for Double {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = self.0;
        if (v.floor() - v.ceil()).abs() < f64::EPSILON {
            // The value is a whole number, which requires at least one decimal point
            // to differentiate it from an integer value.
            write!(f, "double {v:.1}")
        } else {
            write!(f, "double {v}")
        }
    }
}
