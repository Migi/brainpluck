use crate::bf::*;

struct AsyncifiedOp {
    counter: usize,
    kind: AsyncifiedOpKind,
}

enum AsyncifiedOpKind {
    SyncBlock(Vec<BfOp>),
    In,
    AsyncLoop(Vec<AsyncifiedOp>),
}

fn asyncify(ops: Vec<BfOp>) -> Vec<AsyncifiedOp> {
    fn asyncify_rec(ops: Vec<BfOp>) -> Vec<AsyncifiedOp> {
        let mut result = Vec::new();
        let mut sync_ops = Vec::new();
        fn flush_sync_ops(result: &mut Vec<AsyncifiedOp>, sync_ops: &mut Vec<BfOp>) {
            if !sync_ops.is_empty() {
                result.push(AsyncifiedOp {
                    counter: 0,
                    kind: AsyncifiedOpKind::SyncBlock(std::mem::replace(sync_ops, Vec::new())),
                });
            }
        }
        for op in ops {
            match op {
                BfOp::In => {
                    flush_sync_ops(&mut result, &mut sync_ops);
                    result.push(AsyncifiedOp {
                        counter: 0,
                        kind: AsyncifiedOpKind::In,
                    });
                }
                BfOp::Loop(ops) => {
                    let mut rec_result = asyncify_rec(ops);
                    if rec_result.is_empty() {
                        sync_ops.push(BfOp::Loop(Vec::new()));
                    }
                    if rec_result.len() == 1 {
                        match rec_result.pop().unwrap().kind {
                            AsyncifiedOpKind::SyncBlock(rec_ops) => {
                                sync_ops.push(BfOp::Loop(rec_ops));
                            }
                            AsyncifiedOpKind::In => {
                                // loop with only an "in"
                                flush_sync_ops(&mut result, &mut sync_ops);
                                result.push(AsyncifiedOp {
                                    counter: 0,
                                    kind: AsyncifiedOpKind::AsyncLoop(vec![AsyncifiedOp {
                                        counter: 0,
                                        kind: AsyncifiedOpKind::In,
                                    }]),
                                });
                            }
                            AsyncifiedOpKind::AsyncLoop(rec_ops) => {
                                // loop with only another loop in it. Optimize this to a single loop.
                                flush_sync_ops(&mut result, &mut sync_ops);
                                result.push(AsyncifiedOp {
                                    counter: 0,
                                    kind: AsyncifiedOpKind::AsyncLoop(rec_ops),
                                });
                            }
                        }
                    } else {
                        flush_sync_ops(&mut result, &mut sync_ops);
                        result.push(AsyncifiedOp {
                            counter: 0,
                            kind: AsyncifiedOpKind::AsyncLoop(rec_result),
                        });
                    }
                }
                op => sync_ops.push(op),
            }
        }
        flush_sync_ops(&mut result, &mut sync_ops);
        result
    }
    let mut async_ops = asyncify_rec(ops);
    fn set_counter_rec(async_ops: &mut Vec<AsyncifiedOp>, global_async_block_counter: &mut usize) {
        for op in async_ops {
            match &mut op.kind {
                AsyncifiedOpKind::SyncBlock(_) => {
                    let cur_counter = *global_async_block_counter;
                    *global_async_block_counter += 1;
                    op.counter = cur_counter;
                }
                AsyncifiedOpKind::In => {
                    let cur_counter = *global_async_block_counter;
                    *global_async_block_counter += 1;
                    op.counter = cur_counter;
                }
                AsyncifiedOpKind::AsyncLoop(ops) => {
                    set_counter_rec(ops, &mut *global_async_block_counter);
                    let cur_counter = *global_async_block_counter;
                    *global_async_block_counter += 1;
                    op.counter = cur_counter;
                }
            }
        }
    }
    let mut global_async_block_counter = 1;
    set_counter_rec(&mut async_ops, &mut global_async_block_counter);
    async_ops
}

pub fn bf2wasm(bf_ops: Vec<BfOp>, optimize_first: bool) -> wat::Result<Vec<u8>> {
    /*let mut _opt_bf_ops = None;
    let bf_ops = if optimize_first {
        _opt_bf_ops = Some(get_optimized_bf_ops(bf_ops));
        _opt_bf_ops.as_ref().unwrap()
    } else {
        bf_ops
    };*/
    let async_ops = if optimize_first {
        asyncify(get_optimized_bf_ops(&bf_ops))
    } else {
        asyncify(bf_ops)
    };
    fn process_sync_ops_rec(
        bf_ops: &Vec<BfOp>,
        bf_wat: &mut String,
        global_loop_counter: &mut usize,
    ) {
        let mut cur_shift = 0;
        let assure_nonnegative_offsets = |bf_wat: &mut String, cur_shift: &mut i16, added_shifts: &[i16]| {
            let min_shift = added_shifts.iter().cloned().min().unwrap();
            if *cur_shift + min_shift < 0 {
                *bf_wat += &format!("(local.set $cell_ptr (i32.add (local.get $cell_ptr) (i32.const {})))", *cur_shift + min_shift);
                *cur_shift = -min_shift;
            }
        };
        for op in bf_ops {
            match op {
                BfOp::Inc => {
                    assure_nonnegative_offsets(bf_wat, &mut cur_shift, &[0]);
                    *bf_wat += &format!("(i32.store8 offset={} (local.get $cell_ptr) (i32.add (i32.load8_u offset={} (local.get $cell_ptr)) (i32.const 1)))\n", cur_shift, cur_shift);
                }
                BfOp::Dec => {
                    assure_nonnegative_offsets(bf_wat, &mut cur_shift, &[0]);
                    *bf_wat += &format!("(i32.store8 offset={} (local.get $cell_ptr) (i32.add (i32.load8_u offset={} (local.get $cell_ptr)) (i32.const -1)))\n", cur_shift, cur_shift);
                }
                BfOp::Right => {
                    cur_shift += 1;
                }
                BfOp::Left => {
                    cur_shift -= 1;
                }
                BfOp::In => {
                    //*bf_wat += "(i32.store8 (local.get $cell_ptr) (call $read_input_byte))\n";
                    panic!("Encountered In in sync ops!")
                }
                BfOp::Out => {
                    if cur_shift != 0 {
                        *bf_wat += &format!("(local.set $cell_ptr (i32.add (local.get $cell_ptr) (i32.const {})))", cur_shift);
                        cur_shift = 0;
                    }
                    *bf_wat += "(global.set $cell_ptr_global (local.get $cell_ptr))";
                    *bf_wat += "(call $write_output_byte (i32.load8_u (local.get $cell_ptr)))\n";
                }
                BfOp::Loop(ops) => {
                    if cur_shift != 0 {
                        *bf_wat += &format!("(local.set $cell_ptr (i32.add (local.get $cell_ptr) (i32.const {})))", cur_shift);
                        cur_shift = 0;
                    }
                    let cur_loop_id = format!("bf_loop_{}", global_loop_counter);
                    let cur_block_id = format!("bf_loop_block_{}", global_loop_counter);
                    *global_loop_counter += 1;
                    *bf_wat += &format!("(loop ${}\n", cur_loop_id);
                    *bf_wat += &format!("(block ${}\n", cur_block_id);
                    *bf_wat += &format!(
                        "(br_if ${} (i32.eqz (i32.load8_u (local.get $cell_ptr))))\n",
                        cur_block_id
                    );
                    process_sync_ops_rec(ops, &mut *bf_wat, &mut *global_loop_counter);
                    *bf_wat += &format!("(br ${})\n", cur_loop_id);
                    *bf_wat += ")\n";
                    *bf_wat += ")\n";
                }
                BfOp::Clr => {
                    assure_nonnegative_offsets(bf_wat, &mut cur_shift, &[0]);
                    *bf_wat += &format!("(i32.store8 offset={} (local.get $cell_ptr) (i32.const 0))\n", cur_shift);
                }
                BfOp::Shift(shift) => {
                    cur_shift += shift;
                }
                BfOp::Add(val) => {
                    assure_nonnegative_offsets(bf_wat, &mut cur_shift, &[0]);
                    *bf_wat += &format!("(i32.store8 offset={} (local.get $cell_ptr) (i32.add (i32.load8_u offset={} (local.get $cell_ptr)) (i32.const {})))\n", cur_shift, cur_shift, val);
                }
                BfOp::MoveAdd(shift) => {
                    assert_ne!(*shift, 0);
                    // add to new cell:
                    assure_nonnegative_offsets(bf_wat, &mut cur_shift, &[0, *shift]);
                    *bf_wat += &format!("(i32.store8 offset={} (local.get $cell_ptr) (i32.add (i32.load8_u offset={} (local.get $cell_ptr)) (i32.load8_u offset={} (local.get $cell_ptr))))\n", cur_shift+shift, cur_shift+shift, cur_shift);
                    // set cell to 0:
                    *bf_wat += &format!("(i32.store8 offset={} (local.get $cell_ptr) (i32.const 0))\n", cur_shift);
                }
                BfOp::MoveAdd2(shift1, shift2) => {
                    assert_ne!(*shift1, 0);
                    assert_ne!(*shift2, 0);
                    assure_nonnegative_offsets(bf_wat, &mut cur_shift, &[0, *shift1, *shift2]);
                    // read cell:
                    *bf_wat += &format!("(local.set $tmp1 (i32.load8_u offset={} (local.get $cell_ptr)))\n", cur_shift);
                    // add to cell 1:
                    *bf_wat += &format!("(i32.store8 offset={} (local.get $cell_ptr) (i32.add (i32.load8_u offset={} (local.get $cell_ptr)) (local.get $tmp1)))\n", cur_shift+shift1, cur_shift+shift1);
                    // add to cell 2:
                    *bf_wat += &format!("(i32.store8 offset={} (local.get $cell_ptr) (i32.add (i32.load8_u offset={} (local.get $cell_ptr)) (local.get $tmp1)))\n", cur_shift+shift2, cur_shift+shift2);
                    // set cell to 0:
                    *bf_wat += &format!("(i32.store8 offset={} (local.get $cell_ptr) (i32.const 0))\n", cur_shift);
                }
                BfOp::MoveAddMul(shift_adds) => {
                    let mut all_shifts_vec = vec![0];
                    all_shifts_vec.extend(shift_adds.iter().map(|sa| sa.shift));
                    assure_nonnegative_offsets(bf_wat, &mut cur_shift, &all_shifts_vec);
                    // read cell:
                    *bf_wat += &format!("(local.set $tmp1 (i32.load8_u offset={} (local.get $cell_ptr)))\n", cur_shift);
                    for shift_add in shift_adds {
                        assert_ne!(shift_add.shift, 0);
                        assert_ne!(shift_add.add, 0);
                        let mul_expr = if shift_add.add == 1 {
                            format!("(i32.add (i32.load8_u offset={} (local.get $cell_ptr)) (local.get $tmp1))", cur_shift + shift_add.shift)
                        } else if shift_add.add == 255 {
                            format!("(i32.sub (i32.load8_u offset={} (local.get $cell_ptr)) (local.get $tmp1))", cur_shift + shift_add.shift)
                        } else {
                            format!("(i32.add (i32.load8_u offset={} (local.get $cell_ptr)) (i32.mul (local.get $tmp1) (i32.const {})))", cur_shift + shift_add.shift, shift_add.add)
                        };
                        *bf_wat += &format!("(i32.store8 offset={} (local.get $cell_ptr) {})\n", cur_shift + shift_add.shift, mul_expr);
                    }
                    // set cell to 0:
                    *bf_wat += &format!("(i32.store8 offset={} (local.get $cell_ptr) (i32.const 0))\n", cur_shift);
                }
                BfOp::Comment(_) => {}
                BfOp::DebugMessage(_) => {}
                BfOp::Crash(_) => {}
                BfOp::Breakpoint => {}
                BfOp::PrintRegisters => {}
                BfOp::CheckScratchIsEmptyFromHere(_) => {}
            }
        }
        if cur_shift != 0 {
            *bf_wat += &format!("(local.set $cell_ptr (i32.add (local.get $cell_ptr) (i32.const {})))", cur_shift);
        }
    }
    fn process_async_ops_rec(
        ops: &Vec<AsyncifiedOp>,
        bf_wat: &mut String,
        global_loop_counter: &mut usize,
    ) {
        for op in ops {
            let cur_async_block_counter = op.counter;
            let cur_async_block_id = format!("async_block_{}", global_loop_counter);
            *global_loop_counter += 1;
            *bf_wat += &format!("(block ${}\n", cur_async_block_id);
            *bf_wat += &format!(
                "(br_if ${} (i32.gt_u (local.get $async_start_block) (i32.const {})))\n",
                cur_async_block_id, cur_async_block_counter
            );
            match &op.kind {
                AsyncifiedOpKind::SyncBlock(ops) => {
                    process_sync_ops_rec(ops, &mut *bf_wat, &mut *global_loop_counter);
                }
                AsyncifiedOpKind::In => {
                    let inner_block_id = format!("{}_inner", cur_async_block_id);
                    *bf_wat += &format!("(block ${}\n", inner_block_id);
                    *bf_wat += "(global.set $cell_ptr_global (local.get $cell_ptr))\n";
                    *bf_wat += "(local.set $tmp1 (call $read_input_byte))\n";
                    *bf_wat += &format!(
                        "(br_if ${} (i32.ne (i32.const 0) (local.get $tmp1)))\n",
                        inner_block_id
                    );
                    *bf_wat += "(global.set $cell_ptr_global (local.get $cell_ptr))\n";
                    *bf_wat += &format!(
                        "(global.set $async_start_block_global (i32.const {}))\n",
                        cur_async_block_counter
                    );
                    // restore the cell pointer if we are rewinding but still have no input
                    // (see also the big comment block below).
                    *bf_wat += "(block $restore_cell\n";
                    *bf_wat += "(br_if $restore_cell (i32.eqz (local.get $async_start_block)))";
                    *bf_wat += "(i32.store8 (local.get $cell_ptr) (local.get $tmp2))";
                    *bf_wat += ")\n";

                    // return 1: need more input
                    *bf_wat += "(return (i32.const 1))\n";
                    *bf_wat += ")\n";

                    // We have input, set $async_start_block to 0 so we just execute everthing from now on
                    *bf_wat += "(i32.store8 (local.get $cell_ptr) (local.get $tmp1))\n";
                    *bf_wat += "(local.set $async_start_block (i32.const 0))\n";
                }
                AsyncifiedOpKind::AsyncLoop(ops) => {
                    let cur_loop_id = format!("async_bf_loop_{}", global_loop_counter);
                    let cur_block_id = format!("async_bf_loop_block_{}", global_loop_counter);
                    *global_loop_counter += 1;
                    *bf_wat += &format!("(loop ${}\n", cur_loop_id);
                    *bf_wat += &format!("(block ${}\n", cur_block_id);
                    *bf_wat += &format!(
                        "(br_if ${} (i32.eqz (i32.load8_u (local.get $cell_ptr))))\n",
                        cur_block_id
                    );
                    process_async_ops_rec(ops, &mut *bf_wat, &mut *global_loop_counter);
                    *bf_wat += &format!("(br ${})\n", cur_loop_id);
                    *bf_wat += ")\n";
                    *bf_wat += ")\n";
                }
            }
            *bf_wat += ")\n";
        }
    }
    let mut bf_wat = String::new();
    let mut global_loop_counter = 0;
    process_async_ops_rec(&async_ops, &mut bf_wat, &mut global_loop_counter);

    let mut wat = r#"
        (module
            (import "imports" "read_input_byte" (func $read_input_byte (result i32)))
            (import "imports" "write_output_byte" (func $write_output_byte (param i32)))
            (import "imports" "tape" (memory 1))
            (global $cell_ptr_global (mut i32) (i32.const 0))
            (global $async_start_block_global (mut i32) (i32.const 0))
            (export "cell_ptr" (global $cell_ptr_global))
            (func $run_bf (result i32) (local $cell_ptr i32) (local $async_start_block i32)  (local $tmp1 i32) (local $tmp2 i32)
                (local.set $cell_ptr (global.get $cell_ptr_global))
                (local.set $async_start_block (global.get $async_start_block_global))"#.to_owned();
    // This is a bit cursed, but if we're rewinding from a request for more input,
    // then we set the current cell to 1 so that all the loops are executed
    // until we encounter the "," instruction that caused the interruption.
    // At that point, if input is available, the value of the cell will get overridden by the input anyway,
    // and if not, we restore the cell (using $tmp2 to store what the cell was).
    wat += r#"
                (block $if_rewinding
                    (br_if $if_rewinding (i32.eqz (local.get $async_start_block)))
                    (br_if $if_rewinding (i32.eq (local.get $async_start_block) (i32.const 2147483647)))
                    (local.set $tmp2 (i32.load8_u (local.get $cell_ptr)))
                    (i32.store8 (local.get $cell_ptr) (i32.const 1))
                )"#;
    wat += &bf_wat;
    wat += r#"
                (global.set $cell_ptr_global (local.get $cell_ptr))
                (global.set $async_start_block_global (i32.const 2147483647))
                (return (i32.const 0)))
            (export "run_bf" (func $run_bf))
        )"#;
    wat::parse_str(wat)
}
