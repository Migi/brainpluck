use crate::cpu::*;
use crate::linker::CompiledSamProgram;
use crate::sam::*;
use num::BigUint;
use std::collections::{HashMap, HashSet};
use std::result;

pub fn sam2lir(prog: CompiledSamProgram) -> (Vec<Lir>, CpuConfig) {
    fn goto_ptr_register(
        cpu: &mut Cpu,
        scratch_track: ScratchTrack,
        ptr: Register,
        cur_ptr: Register,
    ) {
        let ([keep_going, cmp_result], scratch_track) = scratch_track.split_2();

        let mut ptr = ptr;
        let mut cur_ptr = cur_ptr;

        // elements are (log256, log2)
        //let shift_logs = [(1, 0), (0, 4), (0, 3), (0, 2), (0, 0)];
        let shift_logs = [(1, 0)];

        for (shift_by_log256, shift_by_log2) in shift_logs {
            let shift_by = 1 << (shift_by_log256 * 8 + shift_by_log2);
            cpu.comment(format!("shift_by_{}", shift_by));
            cpu.inc_at(keep_going);
            cpu.loop_while(keep_going, |cpu| {
                if shift_by_log2 != 0 {
                    assert!(ptr.size == 1);
                    assert!(cur_ptr.size == 1);
                    let ([a, b, rem], scratch_track) = scratch_track.split_3();
                    cpu.div_u8_by_const(ptr.at(0), 1 << shift_by_log2, a, rem, scratch_track);
                    cpu.div_u8_by_const(cur_ptr.at(0), 1 << shift_by_log2, b, rem, scratch_track);
                    cpu.cmp_2_u8s(a, b, cmp_result, scratch_track);
                    cpu.clr_at(a);
                    cpu.clr_at(b);
                    cpu.clr_at(rem);
                } else {
                    cpu.cmp_2_uint_registers(
                        ptr.subview(0, ptr.size - shift_by_log256),
                        cur_ptr.subview(0, cur_ptr.size - shift_by_log256),
                        cmp_result,
                        scratch_track,
                    );
                }

                cpu.move_match_cmp_result(
                    cmp_result,
                    scratch_track,
                    |cpu, scratch_track| {
                        if shift_by_log2 != 0 {
                            assert!(cur_ptr.size == 1);
                            cpu.sub_const_from_byte(cur_ptr.at(0), 1 << shift_by_log2);
                        } else {
                            cpu.dec_register(
                                cur_ptr.subview(0, cur_ptr.size - shift_by_log256),
                                scratch_track,
                            );
                        }
                        let scratch_track_size =
                            scratch_track.offset + scratch_track.dont_go_left_of.unwrap_or(0);
                        if shift_by >= 2 {
                            let counter = Pos {
                                track: scratch_track.track.track_num,
                                frame: -1,
                            };
                            cpu.add_const_to_byte(counter, scratch_track_size as u8);
                            cpu.loop_while(counter, |cpu| {
                                cpu.dec();
                                cpu.moveadd_byte(
                                    counter.get_shifted(1),
                                    counter.get_shifted(1 - shift_by),
                                );
                                cpu.moveadd_byte(counter, counter.get_shifted(1));
                                cpu.goto(counter.get_shifted(1));
                                cpu.now_were_actually_at(counter);
                            });
                            cpu.shift_frame_untracked(-scratch_track_size, false);
                        } else {
                            for i in 0..scratch_track_size {
                                cpu.moveadd_byte(
                                    Pos {
                                        track: scratch_track.track.track_num,
                                        frame: i,
                                    },
                                    Pos {
                                        track: scratch_track.track.track_num,
                                        frame: i - shift_by,
                                    },
                                );
                            }
                        }
                        cpu.shift_frame_untracked(-shift_by, false);
                    },
                    |cpu, _| {
                        cpu.dec_at(keep_going);
                    },
                    |cpu, scratch_track| {
                        if shift_by_log2 != 0 {
                            assert!(cur_ptr.size == 1);
                            cpu.add_const_to_byte(cur_ptr.at(0), 1 << shift_by_log2);
                        } else {
                            cpu.inc_register(
                                cur_ptr.subview(0, cur_ptr.size - shift_by_log256),
                                scratch_track,
                            );
                        }
                        let scratch_track_size =
                            scratch_track.offset + scratch_track.dont_go_left_of.unwrap_or(0);
                        if shift_by >= 2 {
                            let counter = Pos {
                                track: scratch_track.track.track_num,
                                frame: scratch_track_size,
                            };
                            cpu.add_const_to_byte(counter, scratch_track_size as u8);
                            cpu.loop_while(counter, |cpu| {
                                cpu.dec();
                                cpu.moveadd_byte(
                                    counter.get_shifted(-1),
                                    counter.get_shifted(-1 + shift_by),
                                );
                                cpu.moveadd_byte(counter, counter.get_shifted(-1));
                                cpu.goto(counter.get_shifted(-1));
                                cpu.now_were_actually_at(counter);
                            });
                            cpu.shift_frame_untracked(scratch_track_size, false);
                        } else {
                            for i in (0..scratch_track_size).rev() {
                                cpu.moveadd_byte(
                                    Pos {
                                        track: scratch_track.track.track_num,
                                        frame: i,
                                    },
                                    Pos {
                                        track: scratch_track.track.track_num,
                                        frame: i + shift_by,
                                    },
                                );
                            }
                        }
                        cpu.shift_frame_untracked(shift_by, false);
                    },
                );
            });
            cpu.clr_at(cmp_result);

            if shift_by_log256 > 0 {
                ptr = ptr.subview(ptr.size - shift_by_log256, shift_by_log256);
                cur_ptr = cur_ptr.subview(cur_ptr.size - shift_by_log256, shift_by_log256);
            }
        }

        assert_eq!(ptr.size, 1);
        assert_eq!(cur_ptr.size, 1);
        let (ptr_unpacked, scratch_track) = scratch_track.split_binregister(8);
        cpu.unpack_register(ptr, ptr_unpacked, scratch_track, false);
        let (cur_ptr_unpacked, scratch_track) = scratch_track.split_binregister(8);
        cpu.unpack_register(cur_ptr, cur_ptr_unpacked, scratch_track, false);
        goto_ptr_binregister(cpu, scratch_track, ptr_unpacked, cur_ptr_unpacked);
        cpu.pack_binregister(cur_ptr_unpacked, cur_ptr, scratch_track, true);
        cpu.clr_binregister(cur_ptr_unpacked, scratch_track);
        cpu.clr_binregister(ptr_unpacked, scratch_track);
    }

    fn goto_ptr_binregister(
        cpu: &mut Cpu,
        scratch_track: ScratchTrack,
        ptr: BinRegister,
        cur_ptr: BinRegister,
    ) {
        let ([keep_going, cmp_result], scratch_track) = scratch_track.split_2();

        let mut ptr = ptr;
        let mut cur_ptr = cur_ptr;

        let shift_log2s = [7, 6, 5, 4, 3, 2, 1, 0];

        for shift_by_log2 in shift_log2s {
            let shift_by = 1 << shift_by_log2;
            cpu.comment(format!("shift_by_{}", shift_by));
            cpu.inc_at(keep_going);
            cpu.loop_while(keep_going, |cpu| {
                cpu.cmp_2_uint_binregisters(
                    ptr.subview(0, ptr.size - shift_by_log2),
                    cur_ptr.subview(0, cur_ptr.size - shift_by_log2),
                    cmp_result,
                    scratch_track,
                );

                cpu.move_match_cmp_result(
                    cmp_result,
                    scratch_track,
                    |cpu, scratch_track| {
                        cpu.dec_binregister(
                            cur_ptr.subview(0, cur_ptr.size - shift_by_log2),
                            scratch_track,
                        );
                        let scratch_track_size =
                            scratch_track.offset + scratch_track.dont_go_left_of.unwrap_or(0);
                        if shift_by >= 2 {
                            let counter = Pos {
                                track: scratch_track.track.track_num,
                                frame: -1,
                            };
                            cpu.add_const_to_byte(counter, scratch_track_size as u8);
                            cpu.loop_while(counter, |cpu| {
                                cpu.dec();
                                cpu.moveadd_byte(
                                    counter.get_shifted(1),
                                    counter.get_shifted(1 - shift_by),
                                );
                                cpu.moveadd_byte(counter, counter.get_shifted(1));
                                cpu.goto(counter.get_shifted(1));
                                cpu.now_were_actually_at(counter);
                            });
                            cpu.shift_frame_untracked(-scratch_track_size, false);
                        } else {
                            for i in 0..scratch_track_size {
                                cpu.moveadd_byte(
                                    Pos {
                                        track: scratch_track.track.track_num,
                                        frame: i,
                                    },
                                    Pos {
                                        track: scratch_track.track.track_num,
                                        frame: i - shift_by,
                                    },
                                );
                            }
                        }
                        cpu.shift_frame_untracked(-shift_by, false);
                    },
                    |cpu, _| {
                        cpu.dec_at(keep_going);
                    },
                    |cpu, scratch_track| {
                        cpu.inc_binregister(
                            cur_ptr.subview(0, cur_ptr.size - shift_by_log2),
                            scratch_track,
                        );
                        let scratch_track_size =
                            scratch_track.offset + scratch_track.dont_go_left_of.unwrap_or(0);
                        if shift_by >= 2 {
                            let counter = Pos {
                                track: scratch_track.track.track_num,
                                frame: scratch_track_size,
                            };
                            cpu.add_const_to_byte(counter, scratch_track_size as u8);
                            cpu.loop_while(counter, |cpu| {
                                cpu.dec();
                                cpu.moveadd_byte(
                                    counter.get_shifted(-1),
                                    counter.get_shifted(-1 + shift_by),
                                );
                                cpu.moveadd_byte(counter, counter.get_shifted(-1));
                                cpu.goto(counter.get_shifted(-1));
                                cpu.now_were_actually_at(counter);
                            });
                            cpu.shift_frame_untracked(scratch_track_size, false);
                        } else {
                            for i in (0..scratch_track_size).rev() {
                                cpu.moveadd_byte(
                                    Pos {
                                        track: scratch_track.track.track_num,
                                        frame: i,
                                    },
                                    Pos {
                                        track: scratch_track.track.track_num,
                                        frame: i + shift_by,
                                    },
                                );
                            }
                        }
                        cpu.shift_frame_untracked(shift_by, false);
                    },
                );
            });
            cpu.clr_at(cmp_result);

            if shift_by_log2 > 0 {
                ptr = ptr.subview(ptr.size - shift_by_log2, shift_by_log2);
                cur_ptr = cur_ptr.subview(cur_ptr.size - shift_by_log2, shift_by_log2);
            }
        }
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
    /*let mut register_builder = cfg.build_register_track(TrackId::Register1);
    let a = register_builder.add_register(4);
    let b = register_builder.add_register(4);
    let c = register_builder.add_register(4);
    let x = register_builder.add_register(1);
    let iptr = register_builder.add_register(4);
    let cur_ptr = register_builder.add_register(4);*/
    let scratch_track = cfg.add_scratch_track(TrackId::Scratch1);
    let (a, scratch_track) = scratch_track.split_register(4);
    let (b, scratch_track) = scratch_track.split_register(4);
    let (c, scratch_track) = scratch_track.split_register(4);
    let (x, scratch_track) = scratch_track.split_register(1);
    let (iptr, scratch_track) = scratch_track.split_register(4);
    let (cur_ptr, scratch_track) = scratch_track.split_register(4);
    let data_track = cfg.add_data_track(TrackId::Stack);

    /*match cfg.tracks.get_mut(&TrackId::Scratch1).unwrap() {
        TrackKind::MultipleRegisters(_, ref mut register_map, ref mut _binregister_map) => {
            register_map.insert("a".to_owned(), a);
            register_map.insert("b".to_owned(), b);
            register_map.insert("c".to_owned(), c);
            register_map.insert("x".to_owned(), x);
            register_map.insert("iptr".to_owned(), iptr);
            register_map.insert("cur_ptr".to_owned(), cur_ptr);
        }
        _ => unreachable!(),
    }*/

    let print_debug_messages = false;
    let print_comments = true;

    let mut cpu = Cpu::new(&cfg);

    let all_registers = Register {
        track: a.track,
        size: a.size + b.size + c.size + x.size + iptr.size + cur_ptr.size,
        offset: a.offset,
    };

    cpu.shift_frame_untracked(5, false);

    for (i, val) in cells.into_iter().enumerate() {
        cpu.add_const_to_byte(data_track.at(i as isize), val);
    }

    cpu.set_register(iptr, initial_instr_ptr);
    cpu.set_register(b, initial_b);

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
    should_goto_b_instr_set.insert(OPCODE_CMP_U8_AT_B_WITH_X);
    should_goto_b_instr_set.insert(OPCODE_CMP_U32_AT_B_WITH_A);
    should_goto_b_instr_set.insert(OPCODE_SET_X_TO_U8_AT_B_DIV_BY_X);
    should_goto_b_instr_set.insert(OPCODE_SET_A_TO_U32_AT_B_DIV_BY_A);
    should_goto_b_instr_set.insert(OPCODE_SET_X_TO_U8_AT_B_MOD_X);
    should_goto_b_instr_set.insert(OPCODE_SET_A_TO_U32_AT_B_MOD_A);

    cpu.comment("Main loop");

    cpu.loop_while(not_halted, |cpu| {
        goto_ptr_register(cpu, scratch_track, iptr, cur_ptr);
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
                    cpu.if_zero(deccing_instr_cpy, scratch_track, |cpu, _| {
                        cpu.inc_at(should_goto_b);
                    });
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

            cpu.comment("Calculate if we should go to b");
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

        cpu.comment("Copy instruction data");
        let (instr_cpy, scratch_track) = scratch_track.split_1();
        cpu.copy_byte_autoscratch(data_track.at(0), instr_cpy, scratch_track);
        let (instr_data, scratch_track) = scratch_track.split_register(4);
        cpu.copy_register(
            data_track.view_register_at(1, 4),
            instr_data,
            scratch_track,
            false,
        );

        cpu.comment("Go to b (if needed)");
        cpu.if_nonzero(should_goto_b, scratch_track, |cpu, scratch_track| {
            cpu.dec_at(should_goto_b);
            goto_ptr_register(cpu, scratch_track, b, cur_ptr);
        });

        let atb_1 = data_track.view_register_at(0, 1);
        let atb_4 = data_track.view_register_at(0, 4);

        let (inc_iptr_by, scratch_track) = scratch_track.split_1();

        let mut cur_instr_num = 0;

        cpu.if_zero(instr_cpy, scratch_track, |cpu, _| {
            assert_eq!(cur_instr_num, OPCODE_HALT);
            if print_debug_messages {
                cpu.debug_message("Instruction: Halt");
            }
            if print_comments {
                cpu.comment("Halt");
            }
            cpu.clr_at(not_halted);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_SET_X);
            if print_debug_messages {
                cpu.debug_message("Instruction: SetX");
            }
            if print_comments {
                cpu.comment("SetX");
            }
            cpu.add_const_to_byte(inc_iptr_by, 2);

            cpu.copy_register(instr_data.subview(0, 1), x, scratch_track, true);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_SET_A);
            if print_debug_messages {
                cpu.debug_message("Instruction: SetA");
            }
            if print_comments {
                cpu.comment("SetA");
            }
            cpu.add_const_to_byte(inc_iptr_by, 5);

            cpu.copy_register(instr_data, a, scratch_track, true);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_READ_A_AT_B);
            if print_debug_messages {
                cpu.debug_message("Instruction: ReadAAtB");
            }
            if print_comments {
                cpu.comment("ReadAAtB");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.copy_register(atb_4, a, scratch_track, true);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_READ_X_AT_B);
            if print_debug_messages {
                cpu.debug_message("Instruction: ReadXAtB");
            }
            if print_comments {
                cpu.comment("ReadXAtB");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.copy_register(atb_1, x, scratch_track, true);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_WRITE_A_AT_B);
            if print_debug_messages {
                cpu.debug_message("Instruction: WriteAAtB");
            }
            if print_comments {
                cpu.comment("WriteAAtB");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.copy_register(a, atb_4, scratch_track, true);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_WRITE_X_AT_B);
            if print_debug_messages {
                cpu.debug_message("Instruction: WriteXAtB");
            }
            if print_comments {
                cpu.comment("WriteXAtB");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.copy_register(x, atb_1, scratch_track, true);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, _| {
            assert_eq!(cur_instr_num, OPCODE_PRINT_CHAR_X);
            if print_debug_messages {
                cpu.debug_message("Instruction: PrintCharX");
            }
            if print_comments {
                cpu.comment("PrintCharX");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.goto(x.at(0));
            cpu.out();
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, _| {
            assert_eq!(cur_instr_num, OPCODE_STDIN_X);
            if print_debug_messages {
                cpu.debug_message("Instruction: StdinX");
            }
            if print_comments {
                cpu.comment("StdinX");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.goto(x.at(0));
            cpu.read_stdin();
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_ADD_CONST_TO_B);
            if print_debug_messages {
                cpu.debug_message("Instruction: AddConstToB");
            }
            if print_comments {
                cpu.comment("AddConstToB");
            }
            cpu.add_const_to_byte(inc_iptr_by, 5);

            let (val_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(instr_data, val_unpacked, scratch_track, false);
            let (b_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(b, b_unpacked, scratch_track, false);
            cpu.add_binregister_to_binregister(val_unpacked, b_unpacked, scratch_track);
            cpu.pack_binregister(b_unpacked, b, scratch_track, true);
            cpu.clr_binregister(b_unpacked, scratch_track);
            cpu.clr_binregister(val_unpacked, scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_SUB_CONST_FROM_B);
            if print_debug_messages {
                cpu.debug_message("Instruction: SubConstFromB");
            }
            if print_comments {
                cpu.comment("SubConstFromB");
            }
            cpu.add_const_to_byte(inc_iptr_by, 5);

            let (val_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(instr_data, val_unpacked, scratch_track, false);
            let (b_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(b, b_unpacked, scratch_track, false);
            cpu.sub_binregister_from_binregister(val_unpacked, b_unpacked, scratch_track);
            cpu.pack_binregister(b_unpacked, b, scratch_track, true);
            cpu.clr_binregister(b_unpacked, scratch_track);
            cpu.clr_binregister(val_unpacked, scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_PRINT_A);
            if print_debug_messages {
                cpu.debug_message("Instruction: PrintA");
            }
            if print_comments {
                cpu.comment("PrintA");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            let (a_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(a, a_unpacked, scratch_track, false);

            cpu.print_binregister_in_decimal(a_unpacked, scratch_track);

            cpu.clr_binregister(a_unpacked, scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_CALL);
            if print_debug_messages {
                cpu.debug_message("Instruction: Call");
            }
            if print_comments {
                cpu.comment("Call");
            }

            // inc instr_ptr by 5
            {
                let (counter, scratch_track) = scratch_track.split_1();
                cpu.add_const_to_byte(counter, 5);
                cpu.loop_while(counter, |cpu| {
                    cpu.dec();
                    cpu.inc_register(iptr, scratch_track);
                });
            }

            cpu.copy_register(iptr, atb_4, scratch_track, true);
            cpu.copy_register(instr_data, iptr, scratch_track, true);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_RET);
            if print_debug_messages {
                cpu.debug_message("Instruction: Ret");
            }
            if print_comments {
                cpu.comment("Ret");
            }

            cpu.copy_register(atb_4, iptr, scratch_track, true);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_JUMP);
            if print_debug_messages {
                cpu.debug_message("Instruction: Jump");
            }
            if print_comments {
                cpu.comment("Jump");
            }

            cpu.add_register_to_register(instr_data, iptr, scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_JUMP_IF_X);
            if print_debug_messages {
                cpu.debug_message("Instruction: JumpIfX");
            }
            if print_comments {
                cpu.comment("JumpIfX");
            }

            cpu.if_nonzero_else(
                x.at(0),
                scratch_track,
                |cpu, scratch_track| {
                    cpu.add_register_to_register(instr_data, iptr, scratch_track);
                },
                |cpu, _| {
                    cpu.add_const_to_byte(inc_iptr_by, 5);
                },
            );
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_ADD_U8_AT_B_TO_X);
            if print_debug_messages {
                cpu.debug_message("Instruction: AddU8AtBToX");
            }
            if print_comments {
                cpu.comment("AddU8AtBToX");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.copy_byte_autoscratch(data_track.at(0), x.at(0), scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_MUL_U8_AT_B_TO_X);
            if print_debug_messages {
                cpu.debug_message("Instruction: MulU8AtBToX");
            }
            if print_comments {
                cpu.comment("MulU8AtBToX");
            }
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
            if print_debug_messages {
                cpu.debug_message("Instruction: AddU32AtBToA");
            }
            if print_comments {
                cpu.comment("AddU32AtBToA");
            }
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
            if print_debug_messages {
                cpu.debug_message("Instruction: MulU32AtBToA");
            }
            if print_comments {
                cpu.comment("MulU32AtBToA");
            }
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
            if print_debug_messages {
                cpu.debug_message("Instruction: NegA");
            }
            if print_comments {
                cpu.comment("NegA");
            }
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
            if print_debug_messages {
                cpu.debug_message("Instruction: NegX");
            }
            if print_comments {
                cpu.comment("NegX");
            }
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
            if print_debug_messages {
                cpu.debug_message("Instruction: MoveXToA");
            }
            if print_comments {
                cpu.comment("MoveXToA");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.clr_register(a, scratch_track);
            cpu.copy_byte_autoscratch(x.at(0), a.at(a.size - 1), scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_NOT_X);
            if print_debug_messages {
                cpu.debug_message("Instruction: NotX");
            }
            if print_comments {
                cpu.comment("NotX");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.not(x.at(0), scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_ADD_CONST_TO_X);
            if print_debug_messages {
                cpu.debug_message("Instruction: AddConstToX");
            }
            if print_comments {
                cpu.comment("AddConstToX");
            }
            cpu.add_const_to_byte(inc_iptr_by, 2);

            cpu.copy_register(instr_data.subview(0, 1), x, scratch_track, false);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_CMP_U8_AT_B_WITH_X);
            if print_debug_messages {
                cpu.debug_message("Instruction: CmpU8AtBWithX");
            }
            if print_comments {
                cpu.comment("CmpU8AtBWithX");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            let (cmp_result, scratch_track) = scratch_track.split_1();

            cpu.cmp_2_u8s(atb_1.at(0), x.at(0), cmp_result, scratch_track);

            cpu.clr_at(x.at(0));
            cpu.moveadd_byte(cmp_result, x.at(0));
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_CMP_U32_AT_B_WITH_A);
            if print_debug_messages {
                cpu.debug_message("Instruction: CmpU32AtBWithA");
            }
            if print_comments {
                cpu.comment("CmpU32AtBWithA");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            let (a_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(a, a_unpacked, scratch_track, false);
            let (atb_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(atb_4, atb_unpacked, scratch_track, false);

            let (cmp_result, scratch_track) = scratch_track.split_1();
            cpu.cmp_2_uint_binregisters(atb_unpacked, a_unpacked, cmp_result, scratch_track);
            cpu.clr_at(x.at(0));
            cpu.moveadd_byte(cmp_result, x.at(0));

            cpu.clr_binregister(a_unpacked, scratch_track);
            cpu.clr_binregister(atb_unpacked, scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_SET_X_TO_U8_AT_B_DIV_BY_X);
            if print_debug_messages {
                cpu.debug_message("Instruction: SetXToU8AtBDivByX");
            }
            if print_comments {
                cpu.comment("SetXToU8AtBDivByX");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            let ([div, rem], scratch_track) = scratch_track.split_2();
            cpu.div_u8s(atb_1.at(0), x.at(0), div, rem, scratch_track);
            cpu.clr_at(x.at(0));
            cpu.moveadd_byte(div, x.at(0));
            cpu.clr_at(rem);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_SET_A_TO_U32_AT_B_DIV_BY_A);
            if print_debug_messages {
                cpu.debug_message("Instruction: SetAToU32AtBDivByA");
            }
            if print_comments {
                cpu.comment("SetAToU32AtBDivByA");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            let (a_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(a, a_unpacked, scratch_track, false);
            cpu.clr_register(a, scratch_track);
            let (atb_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(atb_4, atb_unpacked, scratch_track, false);
            let (div_unpacked, scratch_track) = scratch_track.split_binregister(32);
            let (rem_unpacked, scratch_track) = scratch_track.split_binregister(32);

            cpu.div_binregisters(
                atb_unpacked,
                a_unpacked,
                div_unpacked,
                rem_unpacked,
                scratch_track,
            );
            cpu.pack_binregister(div_unpacked, a, scratch_track, true);

            cpu.clr_binregister(a_unpacked, scratch_track);
            cpu.clr_binregister(atb_unpacked, scratch_track);
            cpu.clr_binregister(div_unpacked, scratch_track);
            cpu.clr_binregister(rem_unpacked, scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_SET_X_TO_U8_AT_B_MOD_X);
            if print_debug_messages {
                cpu.debug_message("Instruction: SetXToU8AtBModX");
            }
            if print_comments {
                cpu.comment("SetXToU8AtBModX");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            let ([div, rem], scratch_track) = scratch_track.split_2();
            cpu.div_u8s(atb_1.at(0), x.at(0), div, rem, scratch_track);
            cpu.clr_at(x.at(0));
            cpu.moveadd_byte(rem, x.at(0));
            cpu.clr_at(div);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_SET_A_TO_U32_AT_B_MOD_A);
            if print_debug_messages {
                cpu.debug_message("Instruction: SetAToU32AtBModA");
            }
            if print_comments {
                cpu.comment("SetAToU32AtBModA");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            let (a_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(a, a_unpacked, scratch_track, false);
            cpu.clr_register(a, scratch_track);
            let (atb_unpacked, scratch_track) = scratch_track.split_binregister(32);
            cpu.unpack_register(atb_4, atb_unpacked, scratch_track, false);
            let (div_unpacked, scratch_track) = scratch_track.split_binregister(32);
            let (rem_unpacked, scratch_track) = scratch_track.split_binregister(32);

            cpu.div_binregisters(
                atb_unpacked,
                a_unpacked,
                div_unpacked,
                rem_unpacked,
                scratch_track,
            );
            cpu.pack_binregister(rem_unpacked, a, scratch_track, true);

            cpu.clr_binregister(a_unpacked, scratch_track);
            cpu.clr_binregister(atb_unpacked, scratch_track);
            cpu.clr_binregister(div_unpacked, scratch_track);
            cpu.clr_binregister(rem_unpacked, scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_COPY_A_TO_B);
            if print_debug_messages {
                cpu.debug_message("Instruction: CopyAToB");
            }
            if print_comments {
                cpu.comment("CopyAToB");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.copy_register(a, b, scratch_track, true);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_COPY_B_TO_A);
            if print_debug_messages {
                cpu.debug_message("Instruction: CopyBToA");
            }
            if print_comments {
                cpu.comment("CopyBToA");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            cpu.copy_register(b, a, scratch_track, true);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        cpu.if_zero(instr_cpy, scratch_track, |cpu, scratch_track| {
            assert_eq!(cur_instr_num, OPCODE_SWAP_B_AND_C);
            if print_debug_messages {
                cpu.debug_message("Instruction: SwapBAndC");
            }
            if print_comments {
                cpu.comment("SwapBAndC");
            }
            cpu.add_const_to_byte(inc_iptr_by, 1);

            let (c_cpy, scratch_track) = scratch_track.split_register(c.size);
            cpu.copy_register(c, c_cpy, scratch_track, false);
            cpu.copy_register(b, c, scratch_track, true);
            cpu.copy_register(c_cpy, b, scratch_track, true);
            cpu.clr_register(c_cpy, scratch_track);
        });
        cur_instr_num += 1;
        cpu.dec_at(instr_cpy);

        assert_eq!(cur_instr_num, NUM_OPCODES);

        //cpu.check_scratch(scratch_track, "At finish of instruction");

        //cpu.debug_message("Finished instruction");
        //cpu.breakpoint();
        if print_debug_messages {
            cpu.print_registers(all_registers.track);
        }

        cpu.loop_while(inc_iptr_by, |cpu| {
            cpu.dec();
            cpu.inc_register(iptr, scratch_track);
        });

        cpu.clr_at(instr_cpy);
        cpu.clr_register(instr_data, scratch_track);

        //cpu.check_scratch(scratch_track, "At end of instruction");
    });

    cpu.clr_at(not_halted);

    (cpu.into_ops(), cfg)
}
