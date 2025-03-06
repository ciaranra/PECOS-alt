pub mod qgate {
    pub struct Measure {
        pub qubit_id: u64,
        pub result_id: u64,
    }

    pub struct Reset {
        pub qubit_id: u64,
    }

    pub struct R1XY {
        pub qubit_id: u64,
        pub theta: f64,
        pub phi: f64,
    }

    pub struct RZZ {
        pub qubit_id_1: u64,
        pub qubit_id_2: u64,
        pub theta: f64,
    }
}

pub enum Ops {
    Measure(Vec<qgate::Measure>),
    Reset(Vec<qgate::Reset>),
    R1XY(Vec<qgate::R1XY>),
    RZZ(Vec<qgate::RZZ>),
}

pub trait CRuntime {
    fn measure(&mut self, qubit_id: u64) -> u64;
    fn reset(&mut self, qubit_id: u64);
    fn r1xy_gate(&mut self, qubit_id: u64, theta: f64, phi: f64);
    fn rzz_gate(&mut self, qubit_id_1: u64, qubit_id_2: u64, theta: f64);
    fn rz_gate(&mut self, qubit_id: u64, theta: f64);
    fn get_result(&mut self, result_id: u64) -> Option<bool>;
    fn set_result(&mut self, result_id: u64, result: bool);
    fn get_next_operaiont(&mut self) -> Option<Ops>;
    fn exit(&mut self);
    fn next_shot(&mut self);
}

struct Runner {
    pub runtime: Box<dyn CRuntime>,
}

impl Runner {
    // TODO: Build up message
    fn next_message(&mut self) {
        while let Some(op) = self.runtime.get_next_operaiont() {
            match op {
                Ops::Measure(gvec) => 0,
                Ops::Reset(gvec) => 1,
                Ops::R1XY(gvec) => 2,
                Ops::RZZ(gvec) => 3,
            }
        }
    }
}
