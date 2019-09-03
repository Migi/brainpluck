use crate::bf::*;
use crate::cpu::*;

pub fn lir2bf(ops: Vec<Lir>) -> Vec<BfOp> {
    let mut result = Vec::new();
    for op in ops {
        match op {
            Lir::Left => {
                result.push(BfOp::Left);
            }
            Lir::Right => {
                result.push(BfOp::Right);
            }
            Lir::Inc => {
                result.push(BfOp::Inc);
            }
            Lir::Dec => {
                result.push(BfOp::Dec);
            }
            Lir::In => {
                result.push(BfOp::In);
            }
            Lir::Out => {
                result.push(BfOp::Out);
            }
            Lir::Loop(ops) => {
                result.push(BfOp::Loop(lir2bf(ops)));
            }
        }
    }
    result
}
