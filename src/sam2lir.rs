use crate::cpu::*;
use crate::linker::CompiledSamProgram;
use crate::sam::*;
use num::BigUint;
use std::collections::{HashMap, HashSet};
use std::result;

pub fn sam2lir(prog: CompiledSamProgram) -> (Vec<Lir>, CpuConfig) {
    fn goto_ptr(
        cpu: &mut Cpu,
        scratch_track: ScratchTrack,
        ptr: BinRegister,
        cur_ptr: BinRegister,
        all_registers: Register,
    ) {
        let ([keep_going, cmp_result], scratch_track) = scratch_track.split_2();
        cpu.inc_at(keep_going);
        cpu.loop_while(keep_going, |cpu| {
            cpu.cmp_2_uint_binregisters(ptr, cur_ptr, cmp_result, scratch_track);

            // debug:
            /*cpu.goto(cmp_result);
            cpu.debug_message("Before going");
            cpu.breakpoint();*/

            cpu.match_cmp_result(
                cmp_result,
                scratch_track,
                |cpu, scratch_track| {
                    /*cpu.debug_message("Going left");
                    cpu.breakpoint();
                    cpu.print_registers(all_registers.track);*/
                    cpu.dec_binregister(cur_ptr, scratch_track);
                    cpu.shift_register_left(
                        all_registers.subview_unchecked(-1, all_registers.size + 1),
                        scratch_track,
                    );
                    let scratch_track_size =
                        scratch_track.offset + scratch_track.dont_go_left_of.unwrap_or(0);
                    for i in 0..scratch_track_size {
                        cpu.moveadd_byte(
                            Pos {
                                track: scratch_track.track.track_num,
                                frame: i,
                            },
                            Pos {
                                track: scratch_track.track.track_num,
                                frame: i - 1,
                            },
                        );
                    }
                    cpu.shift_frame_untracked(-1, false);
                },
                |cpu, _| {
                    /*cpu.debug_message("Done going");
                    cpu.breakpoint();
                    cpu.print_registers(all_registers.track);*/
                    cpu.dec_at(keep_going);
                },
                |cpu, scratch_track| {
                    /*cpu.debug_message("Going right");
                    cpu.breakpoint();
                    cpu.print_registers(all_registers.track);*/
                    cpu.inc_binregister(cur_ptr, scratch_track);
                    cpu.shift_register_right(
                        all_registers.subview_unchecked(0, all_registers.size + 1),
                        scratch_track,
                    );
                    let scratch_track_size =
                        scratch_track.offset + scratch_track.dont_go_left_of.unwrap_or(0);
                    for i in (0..scratch_track_size).rev() {
                        cpu.moveadd_byte(
                            Pos {
                                track: scratch_track.track.track_num,
                                frame: i,
                            },
                            Pos {
                                track: scratch_track.track.track_num,
                                frame: i + 1,
                            },
                        );
                    }
                    cpu.shift_frame_untracked(1, false);
                },
            );
            cpu.clr_at(cmp_result);
        });
    }

    let initial_instr_ptr = *prog
        .fn_start_poss
        .get("main")
        .expect("no main function found");
    let mut cells = prog.bytes;
    let hlt = cells.len() as SamVal;
    cells.extend(&[OPCODE_HALT]);
    let initial_b = cells.len() as SamVal;
    push_u32_to_vec(&mut cells, hlt);

    let mut cfg = CpuConfig::new();
    let mut register_builder = cfg.build_register_track(TrackId::Register1);
    let a = register_builder.add_register(4);
    let b = register_builder.add_binregister(32);
    let x = register_builder.add_register(1);
    let iptr = register_builder.add_binregister(32);
    let cur_ptr = register_builder.add_binregister(32);
    let scratch_track = cfg.add_scratch_track(TrackId::Scratch1);
    let data_track = cfg.add_data_track(TrackId::Stack);

    match cfg.tracks.get_mut(&TrackId::Register1).unwrap() {
        TrackKind::MultipleRegisters(_, ref mut register_map, ref mut binregister_map) => {
            register_map.insert("a".to_owned(), a);
            register_map.insert("x".to_owned(), x);
            binregister_map.insert("b".to_owned(), b);
            binregister_map.insert("iptr".to_owned(), iptr);
            binregister_map.insert("cur_ptr".to_owned(), cur_ptr);
        }
        _ => unreachable!(),
    }

    let mut cpu = Cpu::new(&cfg);

    let all_registers = Register {
        track: a.track,
        size: a.size + b.size + x.size + iptr.size + cur_ptr.size,
        offset: a.offset,
    };

    cpu.shift_frame_untracked(5, false);

    for (i, val) in cells.into_iter().enumerate() {
        cpu.add_const_to_byte(data_track.at(i as isize), val);
    }

    cpu.set_binregister(iptr, initial_instr_ptr, scratch_track);
    cpu.set_binregister(b, initial_b, scratch_track);

    let (not_halted, scratch_track) = scratch_track.split_1();
    cpu.inc_at(not_halted);

    let mut should_goto_b_instr_set = HashSet::new();
    should_goto_b_instr_set.insert(OPCODE_READ_A_AT_B);
    should_goto_b_instr_set.insert(OPCODE_READ_X_AT_B);
    should_goto_b_instr_set.insert(OPCODE_WRITE_A_AT_B);
    should_goto_b_instr_set.insert(OPCODE_WRITE_X_AT_B);
    should_goto_b_instr_set.insert(OPCODE_ADD_CONST_TO_B);
    should_goto_b_instr_set.insert(OPCODE_SUB_CONST_FROM_B);
    should_goto_b_instr_set.insert(OPCODE_CALL);
    should_goto_b_instr_set.insert(OPCODE_RET);
    should_goto_b_instr_set.insert(OPCODE_ADD_U8_AT_B_TO_X);
    should_goto_b_instr_set.insert(OPCODE_MUL_U8_AT_B_TO_X);
    should_goto_b_instr_set.insert(OPCODE_ADD_U32_AT_B_TO_A);
    should_goto_b_instr_set.insert(OPCODE_MUL_U32_AT_B_TO_A);

    cpu.loop_while(not_halted, |cpu| {
        goto_ptr(cpu, scratch_track, iptr, cur_ptr, all_registers);
        let (should_goto_b, scratch_track) = scratch_track.split_1();
        {
            let (deccing_instr_cpy, scratch_track) = scratch_track.split_1();
            cpu.copy_byte_autoscratch(data_track.at(0), deccing_instr_cpy, scratch_track);

            fn process_should_goto_b_rec(
                cpu: &mut Cpu,
                scratch_track: ScratchTrack,
                set: &HashSet<u8>,
                should_goto_b: Pos,
                deccing_instr_cpy: Pos,
                cur_instr: u8,
            ) {
                if set.contains(&cur_instr) {
                    cpu.if_nonzero_else(
                        deccing_instr_cpy,
                        scratch_track,
                        |_, _| {},
                        |cpu, _| {
                            cpu.inc_at(should_goto_b);
                        },
                    );
                }
                let max_should_goto_b_instr = *set.iter().max().unwrap();
                if cur_instr < max_should_goto_b_instr {
                    cpu.dec_at(deccing_instr_cpy);
                    process_should_goto_b_rec(
                        cpu,
                        scratch_track,
                        set,
                        should_goto_b,
                        deccing_instr_cpy,
                        cur_instr + 1,
                    );
                }
            }
            process_should_goto_b_rec(
                cpu,
                scratch_track,
                &should_goto_b_instr_set,
                should_goto_b,
                deccing_instr_cpy,
                0,
            );
            cpu.clr_at(deccing_instr_cpy);
        }
        let (instr_cpy, scratch_track) = scratch_track.split_1();
        cpu.copy_byte_autoscratch(data_track.at(0), instr_cpy, scratch_track);
        let (instr_data, scratch_track) = scratch_track.split_register(4);
        cpu.copy_register(
            data_track.view_register_at(1, 4),
            instr_data,
            scratch_track,
            false,
        );
        cpu.if_nonzero(should_goto_b, scratch_track, |cpu, scratch_track| {
            cpu.dec_at(should_goto_b);
            goto_ptr(cpu, scratch_track, b, cur_ptr, all_registers);
        });

        let atb_1 = data_track.view_register_at(0, 1);
        let atb_4 = data_track.view_register_at(0, 4);

        let (inc_iptr_by, scratch_track) = scratch_track.split_1();

        let mut cur_instr_num = 0;

        cpu.if_zero(instr_cpy, scratch_track, |cpu, _| {
            assert_eq!(cur_instr_num, OPCODE_HALT);
            cpu.debug_message("Instruction: Halt");
            cpu.clr_at(not_halted);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_SET_X);
            cpu.debug_message("Instruction: SetX");
            cpu.add_const_to_byte(inc_iptr_by, 2);

            cpu.copy_register(instr_data.subview(0, 1), x, scratch_track, true);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_SET_A);
            cpu.debug_message("Instruction: SetA");
            cpu.add_const_to_byte(inc_iptr_by, 5);

            cpu.copy_register(instr_data, a, scratch_track, true);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_READ_A_AT_B);
            cpu.debug_message("Instruction: ReadAAtB");
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.copy_register(atb_4, a, scratch_track, true);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_READ_X_AT_B);
            cpu.debug_message("Instruction: ReadXAtB");
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.copy_register(atb_1, x, scratch_track, true);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_WRITE_A_AT_B);
            cpu.debug_message("Instruction: WriteAAtB");
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.copy_register(a, atb_4, scratch_track, true);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_WRITE_X_AT_B);
            cpu.debug_message("Instruction: WriteXAtB");
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.copy_register(x, atb_1, scratch_track, true);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, _| {
            assert_eq!(cur_instr_num, OPCODE_PRINT_CHAR_X);
            cpu.debug_message("Instruction: PrintCharX");
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.goto(x.at(0));
            cpu.out();
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, _| {
            assert_eq!(cur_instr_num, OPCODE_STDIN_X);
            cpu.debug_message("Instruction: StdinX");
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.goto(x.at(0));
            cpu.read_stdin();
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_ADD_CONST_TO_B);
            cpu.debug_message("Instruction: AddConstToB");
            cpu.add_const_to_byte(inc_iptr_by, 5);

            let (val_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(instr_data, val_unpacked, scratch_track, false);
            cpu.add_binregister_to_binregister(val_unpacked, b, scratch_track);
            cpu.clr_binregister(val_unpacked, scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_SUB_CONST_FROM_B);
            cpu.debug_message("Instruction: SubConstFromB");
            cpu.add_const_to_byte(inc_iptr_by, 5);

            let (val_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(instr_data, val_unpacked, scratch_track, false);
            cpu.sub_binregister_from_binregister(val_unpacked, b, scratch_track);
            cpu.clr_binregister(val_unpacked, scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, _scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_PRINT_A);
            cpu.debug_message("Instruction: PrintA");
            cpu.add_const_to_byte(inc_iptr_by, 1);

            // not implemented yet!
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_CALL);
            cpu.debug_message("Instruction: Call");

            // inc instr_ptr by 5
            {
                let (counter, scratch_track) = scratch_track.split_1();
                cpu.add_const_to_byte(counter, 5);
                cpu.loop_while(counter, |cpu| {
                    cpu.dec();
                    cpu.inc_binregister(iptr, scratch_track);
                });
            }

            cpu.pack_binregister(iptr, atb_4, scratch_track, true);

            cpu.unpack_register(instr_data, iptr, scratch_track, true);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_RET);
            cpu.debug_message("Instruction: Ret");

            cpu.unpack_register(atb_4, iptr, scratch_track, true);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_JUMP);
            cpu.debug_message("Instruction: Jump");

            let (val_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(instr_data, val_unpacked, scratch_track, false);
            cpu.add_binregister_to_binregister(val_unpacked, iptr, scratch_track);
            cpu.clr_binregister(val_unpacked, scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_JUMP_IF_X);
            cpu.debug_message("Instruction: JumpIfX");

            cpu.if_nonzero(x.at(0), scratch_track, |cpu, scratch_track| {
                let (val_unpacked, scratch_track) = scratch_track.split_binregister(32);
                cpu.unpack_register(instr_data, val_unpacked, scratch_track, false);
                cpu.add_binregister_to_binregister(val_unpacked, iptr, scratch_track);
                cpu.clr_binregister(val_unpacked, scratch_track);
            });
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_ADD_U8_AT_B_TO_X);
            cpu.debug_message("Instruction: AddU8AtBToX");
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.copy_byte_autoscratch(data_track.at(0), x.at(0), scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_MUL_U8_AT_B_TO_X);
            cpu.debug_message("Instruction: MulU8AtBToX");
            cpu.add_const_to_byte(inc_iptr_by, 1);

            let ([x_cpy, bval_cpy], scratch_track) = scratch_track.split_2();
            cpu.copy_byte_autoscratch(x.at(0), x_cpy, scratch_track);
            cpu.copy_byte_autoscratch(data_track.at(0), bval_cpy, scratch_track);

            cpu.clr_at(x.at(0));

            cpu.loop_while(bval_cpy, |cpu| {
                cpu.dec();
                cpu.copy_byte_autoscratch(x_cpy, x.at(0), scratch_track);
            });

            cpu.clr_at(x_cpy);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_ADD_U32_AT_B_TO_A);
            cpu.debug_message("Instruction: AddU32AtBToA");
            cpu.add_const_to_byte(inc_iptr_by, 1);

            let (a_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(a, a_unpacked, scratch_track, false);
            let (atb_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(atb_4, atb_unpacked, scratch_track, false);

            cpu.add_binregister_to_binregister(atb_unpacked, a_unpacked, scratch_track);
            cpu.pack_binregister(a_unpacked, a, scratch_track, true);

            cpu.clr_binregister(a_unpacked, scratch_track);
            cpu.clr_binregister(atb_unpacked, scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_MUL_U32_AT_B_TO_A);
            cpu.debug_message("Instruction: MulU32AtBToA");
            cpu.add_const_to_byte(inc_iptr_by, 1);

            let (a_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(a, a_unpacked, scratch_track, false);
            let (atb_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(atb_4, atb_unpacked, scratch_track, false);
            let (result_unpacked, scratch_track) = scratch_track.split_binregister(32);

            cpu.mul_binregisters(a_unpacked, atb_unpacked, result_unpacked, scratch_track);
            cpu.pack_binregister(result_unpacked, a, scratch_track, true);

            cpu.clr_binregister(a_unpacked, scratch_track);
            cpu.clr_binregister(atb_unpacked, scratch_track);
            cpu.clr_binregister(result_unpacked, scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_NEG_A);
            cpu.debug_message("Instruction: NegA");
            cpu.add_const_to_byte(inc_iptr_by, 1);

            let (a_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(a, a_unpacked, scratch_track, false);
            let (result_unpacked, scratch_track) = scratch_track.split_binregister(32);

            cpu.sub_binregister_from_binregister(a_unpacked, result_unpacked, scratch_track);
            cpu.pack_binregister(result_unpacked, a, scratch_track, true);

            cpu.clr_binregister(a_unpacked, scratch_track);
            cpu.clr_binregister(result_unpacked, scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_NEG_X);
            cpu.debug_message("Instruction: NegX");
            cpu.add_const_to_byte(inc_iptr_by, 1);

            let (x_cpy, _) = scratch_track.split_1();
            cpu.moveadd_byte(x.at(0), x_cpy);
            cpu.loop_while(x_cpy, |cpu| {
                cpu.dec();
                cpu.dec_at(x.at(0));
            });
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_MOVE_X_TO_A);
            cpu.debug_message("Instruction: MoveXToA");
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.clr_register(a, scratch_track);
            cpu.copy_byte_autoscratch(x.at(0), a.at(a.size - 1), scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        assert_eq!(cur_instr_num, 23);

        cpu.check_scratch(scratch_track, "At finish of instruction");

        cpu.debug_message("Finished instruction");
        cpu.breakpoint();
        cpu.print_registers(all_registers.track);

        cpu.loop_while(inc_iptr_by, |cpu| {
            cpu.dec();
            cpu.inc_binregister(iptr, scratch_track);
        });

        cpu.clr_at(instr_cpy);
        cpu.clr_register(instr_data, scratch_track);

        cpu.check_scratch(scratch_track, "At end of instruction");
    });

    cpu.clr_at(not_halted);

    (cpu.into_ops(), cfg)
}
