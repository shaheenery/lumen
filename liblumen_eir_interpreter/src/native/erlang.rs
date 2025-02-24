use liblumen_alloc::erts::term::{atom_unchecked, Atom, TypedTerm};
use liblumen_alloc::erts::ModuleFunctionArity;
use lumen_runtime::otp::erlang;

use crate::module::NativeModule;

pub fn make_erlang() -> NativeModule {
    let mut native = NativeModule::new(Atom::try_from_str("erlang").unwrap());

    native.add_simple(Atom::try_from_str("*").unwrap(), 2, |proc, args| {
        erlang::multiply_2(args[0], args[1], proc)
    });

    native.add_simple(Atom::try_from_str("/").unwrap(), 2, |proc, args| {
        erlang::div_2(args[0], args[1], proc)
    });

    native.add_simple(Atom::try_from_str("<").unwrap(), 2, |_proc, args| {
        Ok(erlang::is_less_than_2(args[0], args[1]))
    });
    native.add_simple(Atom::try_from_str(">").unwrap(), 2, |_proc, args| {
        Ok(erlang::is_greater_than_2(args[0], args[1]))
    });
    native.add_simple(Atom::try_from_str("=<").unwrap(), 2, |_proc, args| {
        Ok(erlang::is_equal_or_less_than_2(args[0], args[1]))
    });
    native.add_simple(Atom::try_from_str(">=").unwrap(), 2, |_proc, args| {
        Ok(erlang::is_greater_than_or_equal_2(args[0], args[1]))
    });
    native.add_simple(Atom::try_from_str("==").unwrap(), 2, |_proc, args| {
        Ok(erlang::are_equal_after_conversion_2(args[0], args[1]))
    });
    native.add_simple(Atom::try_from_str("=:=").unwrap(), 2, |_proc, args| {
        Ok(erlang::are_exactly_equal_2(args[0], args[1]))
    });

    native.add_simple(Atom::try_from_str("spawn_opt").unwrap(), 4, |proc, args| {
        match args[3].to_typed_term().unwrap() {
            TypedTerm::List(cons) => {
                let mut iter = cons.into_iter();
                assert!(iter.next() == Some(Ok(atom_unchecked("link").into())));
                assert!(iter.next() == None);
            }
            t => panic!("{:?}", t),
        }

        let ret = {
            let mfa = ModuleFunctionArity {
                module: Atom::try_from_str("lumen_eir_interpreter_intrinsics").unwrap(),
                function: Atom::try_from_str("return_clean").unwrap(),
                arity: 1,
            };
            proc.closure_with_env_from_slice(
                mfa.into(),
                crate::code::return_clean,
                proc.pid_term(),
                &[],
            )?
        };

        let inner_args = proc.cons(ret, proc.cons(ret, args[2])?)?;

        let res = erlang::spawn_link_3::native(proc, args[0], args[1], inner_args)?;
        Ok(res)
    });

    native.add_simple(Atom::try_from_str("spawn").unwrap(), 3, |proc, args| {
        let ret = {
            let mfa = ModuleFunctionArity {
                module: Atom::try_from_str("lumen_eir_interpreter_intrinsics").unwrap(),
                function: Atom::try_from_str("return_clean").unwrap(),
                arity: 1,
            };
            proc.closure_with_env_from_slice(
                mfa.into(),
                crate::code::return_clean,
                proc.pid_term(),
                &[],
            )?
        };

        let inner_args = proc.cons(ret, proc.cons(ret, args[2])?)?;
        erlang::spawn_3::native(proc, args[0], args[1], inner_args)
    });

    native.add_simple(
        Atom::try_from_str("spawn_link").unwrap(),
        3,
        |proc, args| {
            let ret = {
                let mfa = ModuleFunctionArity {
                    module: Atom::try_from_str("lumen_eir_interpreter_intrinsics").unwrap(),
                    function: Atom::try_from_str("return_clean").unwrap(),
                    arity: 1,
                };
                proc.closure_with_env_from_slice(
                    mfa.into(),
                    crate::code::return_clean,
                    proc.pid_term(),
                    &[],
                )?
            };

            let inner_args = proc.cons(ret, proc.cons(ret, args[2])?)?;
            erlang::spawn_link_3::native(proc, args[0], args[1], inner_args)
        },
    );

    native.add_simple(Atom::try_from_str("exit").unwrap(), 1, |_proc, args| {
        panic!("{:?}", args[0]);
        //Ok(erlang::exit_1::native(args[0]).unwrap())
    });

    native.add_simple(Atom::try_from_str("monitor").unwrap(), 2, |proc, args| {
        erlang::monitor_2::native(proc, args[0], args[1])
    });
    native.add_simple(Atom::try_from_str("demonitor").unwrap(), 2, |proc, args| {
        erlang::demonitor_2::native(proc, args[0], args[1])
    });

    native.add_simple(Atom::try_from_str("register").unwrap(), 2, |proc, args| {
        erlang::register_2(args[0], args[1], proc.clone())
    });
    native.add_simple(
        Atom::try_from_str("process_flag").unwrap(),
        2,
        |proc, args| erlang::process_flag_2::native(proc, args[0], args[1]),
    );

    native.add_simple(Atom::try_from_str("send").unwrap(), 2, |proc, args| {
        erlang::send_2(args[0], args[1], proc)
    });
    native.add_simple(Atom::try_from_str("send").unwrap(), 3, |proc, args| {
        erlang::send_2(args[0], args[1], proc)
    });
    native.add_simple(Atom::try_from_str("!").unwrap(), 2, |proc, args| {
        erlang::send_2(args[0], args[1], proc)
    });

    native.add_simple(Atom::try_from_str("-").unwrap(), 2, |proc, args| {
        erlang::subtract_2::native(proc, args[0], args[1])
    });

    native.add_simple(Atom::try_from_str("+").unwrap(), 2, |proc, args| {
        erlang::add_2::native(proc, args[0], args[1])
    });

    native.add_simple(Atom::try_from_str("self").unwrap(), 0, |proc, _args| {
        Ok(proc.pid_term())
    });

    native.add_simple(
        Atom::try_from_str("is_integer").unwrap(),
        1,
        |_proc, args| {
            assert!(args.len() == 1);
            Ok(erlang::is_integer_1(args[0]))
        },
    );
    native.add_simple(Atom::try_from_str("is_list").unwrap(), 1, |_proc, args| {
        Ok(erlang::is_list_1(args[0]))
    });
    native.add_simple(
        Atom::try_from_str("is_binary").unwrap(),
        1,
        |_proc, args| Ok(erlang::is_binary_1(args[0])),
    );
    native.add_simple(Atom::try_from_str("is_atom").unwrap(), 1, |_proc, args| {
        Ok(erlang::is_atom_1(args[0]))
    });
    native.add_simple(Atom::try_from_str("is_pid").unwrap(), 1, |_proc, args| {
        Ok(erlang::is_pid_1(args[0]))
    });
    native.add_simple(
        Atom::try_from_str("is_function").unwrap(),
        1,
        |_proc, args| Ok(erlang::is_function_1(args[0])),
    );
    native.add_simple(
        Atom::try_from_str("is_function").unwrap(),
        2,
        |_proc, args| Ok(erlang::is_function_1(args[0])),
    );
    native.add_simple(Atom::try_from_str("is_tuple").unwrap(), 1, |_proc, args| {
        Ok(erlang::is_tuple_1(args[0]))
    });
    native.add_simple(Atom::try_from_str("is_map").unwrap(), 1, |_proc, args| {
        Ok(erlang::is_map_1(args[0]))
    });
    native.add_simple(
        Atom::try_from_str("is_bitstring").unwrap(),
        1,
        |_proc, args| Ok(erlang::is_bitstring_1(args[0])),
    );
    native.add_simple(Atom::try_from_str("is_float").unwrap(), 1, |_proc, args| {
        Ok(erlang::is_bitstring_1(args[0]))
    });

    native.add_simple(
        Atom::try_from_str("monotonic_time").unwrap(),
        0,
        |proc, _args| erlang::monotonic_time_0::native(proc),
    );

    native.add_yielding(Atom::try_from_str("apply").unwrap(), 3, |proc, args| {
        let inner_args = proc.cons(args[0], proc.cons(args[1], args[4])?)?;
        proc.stack_push(inner_args)?;

        proc.stack_push(args[3])?;
        proc.stack_push(args[2])?;

        crate::code::apply(proc)
    });

    native.add_simple(Atom::try_from_str("node").unwrap(), 0, |_proc, _args| {
        Ok(erlang::node_0())
    });
    native.add_simple(Atom::try_from_str("node").unwrap(), 1, |_proc, _args| {
        Ok(atom_unchecked("nonode@nohost"))
    });
    native.add_simple(Atom::try_from_str("whereis").unwrap(), 1, |_proc, args| {
        erlang::whereis_1(args[0])
    });

    native.add_simple(
        Atom::try_from_str("process_info").unwrap(),
        2,
        |proc, args| erlang::process_info_2::native(proc, args[0], args[1]),
    );

    native.add_simple(Atom::try_from_str("get").unwrap(), 1, |proc, args| {
        Ok(proc.get(args[0]))
    });
    native.add_simple(Atom::try_from_str("put").unwrap(), 2, |proc, args| {
        Ok(proc.put(args[0], args[1])?)
    });

    native.add_simple(
        Atom::try_from_str("convert_time_unit").unwrap(),
        3,
        |proc, args| erlang::convert_time_unit_3::native(proc, args[0], args[1], args[2]),
    );

    native.add_simple(Atom::try_from_str("element").unwrap(), 2, |_proc, args| {
        erlang::element_2(args[0], args[1])
    });

    native
}
