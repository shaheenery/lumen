use super::*;

use num_traits::Num;

use std::sync::{Arc, RwLock};

use crate::environment::{self, Environment};

#[test]
fn with_atom_errors_badarg() {
    let environment_rw_lock: Arc<RwLock<Environment>> = Default::default();
    let process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
    let mut process = process_rw_lock.write().unwrap();
    let tuple = Term::str_to_atom("atom", DoNotCare, &mut process).unwrap();
    let index = 1.into_process(&mut process);
    let value = 4.into_process(&mut process);

    assert_bad_argument!(
        erlang::setelement_3(index, tuple, value, &mut process),
        &mut process
    );
}

#[test]
fn with_local_reference_errors_badarg() {
    let environment_rw_lock: Arc<RwLock<Environment>> = Default::default();
    let process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
    let mut process = process_rw_lock.write().unwrap();
    let tuple = Term::local_reference(&mut process);
    let index = 1.into_process(&mut process);
    let value = 4.into_process(&mut process);

    assert_bad_argument!(
        erlang::setelement_3(index, tuple, value, &mut process),
        &mut process
    );
}

#[test]
fn with_empty_list_errors_badarg() {
    let environment_rw_lock: Arc<RwLock<Environment>> = Default::default();
    let process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
    let mut process = process_rw_lock.write().unwrap();
    let tuple = Term::EMPTY_LIST;
    let index = 1.into_process(&mut process);
    let value = 4.into_process(&mut process);

    assert_bad_argument!(
        erlang::setelement_3(index, tuple, value, &mut process),
        &mut process
    );
}

#[test]
fn with_list_errors_badarg() {
    let environment_rw_lock: Arc<RwLock<Environment>> = Default::default();
    let process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
    let mut process = process_rw_lock.write().unwrap();
    let tuple = Term::cons(1.into_process(&mut process), Term::EMPTY_LIST, &mut process);
    let index = 1.into_process(&mut process);
    let value = 4.into_process(&mut process);

    assert_bad_argument!(
        erlang::setelement_3(index, tuple, value, &mut process),
        &mut process
    );
}

#[test]
fn with_small_integer_errors_badarg() {
    let environment_rw_lock: Arc<RwLock<Environment>> = Default::default();
    let process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
    let mut process = process_rw_lock.write().unwrap();
    let tuple = 0.into_process(&mut process);
    let index = 1.into_process(&mut process);
    let value = 4.into_process(&mut process);

    assert_bad_argument!(
        erlang::setelement_3(index, tuple, value, &mut process),
        &mut process
    );
}

#[test]
fn with_big_integer_errors_badarg() {
    let environment_rw_lock: Arc<RwLock<Environment>> = Default::default();
    let process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
    let mut process = process_rw_lock.write().unwrap();
    let tuple = <BigInt as Num>::from_str_radix("576460752303423489", 10)
        .unwrap()
        .into_process(&mut process);
    let index = 1.into_process(&mut process);
    let value = 4.into_process(&mut process);

    assert_bad_argument!(
        erlang::setelement_3(index, tuple, value, &mut process),
        &mut process
    );
}

#[test]
fn with_float_errors_badarg() {
    let environment_rw_lock: Arc<RwLock<Environment>> = Default::default();
    let process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
    let mut process = process_rw_lock.write().unwrap();
    let tuple = 1.0.into_process(&mut process);
    let index = 1.into_process(&mut process);
    let value = 4.into_process(&mut process);

    assert_bad_argument!(
        erlang::setelement_3(index, tuple, value, &mut process),
        &mut process
    );
}

#[test]
fn with_local_pid_errors_badarg() {
    let environment_rw_lock: Arc<RwLock<Environment>> = Default::default();
    let process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
    let mut process = process_rw_lock.write().unwrap();
    let tuple = Term::local_pid(0, 0, &mut process).unwrap();
    let index = 1.into_process(&mut process);
    let value = 4.into_process(&mut process);

    assert_bad_argument!(
        erlang::setelement_3(index, tuple, value, &mut process),
        &mut process
    );
}

#[test]
fn with_external_pid_errors_badarg() {
    let environment_rw_lock: Arc<RwLock<Environment>> = Default::default();
    let process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
    let mut process = process_rw_lock.write().unwrap();
    let tuple = Term::external_pid(1, 0, 0, &mut process).unwrap();
    let index = 1.into_process(&mut process);
    let value = 4.into_process(&mut process);

    assert_bad_argument!(
        erlang::setelement_3(index, tuple, value, &mut process),
        &mut process
    );
}

#[test]
fn with_tuple_without_valid_index_errors_badarg() {
    let environment_rw_lock: Arc<RwLock<Environment>> = Default::default();
    let process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
    let mut process = process_rw_lock.write().unwrap();
    let tuple = Term::slice_to_tuple(&[], &mut process);
    let index = 1.into_process(&mut process);
    let value = 4.into_process(&mut process);

    assert_bad_argument!(
        erlang::setelement_3(index, tuple, value, &mut process),
        &mut process
    );
}

#[test]
fn with_tuple_with_valid_index_returns_tuple_with_index_replaced() {
    let environment_rw_lock: Arc<RwLock<Environment>> = Default::default();
    let process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
    let mut process = process_rw_lock.write().unwrap();
    let first_element = 1.into_process(&mut process);
    let second_element = 2.into_process(&mut process);
    let third_element = 3.into_process(&mut process);
    let tuple = Term::slice_to_tuple(
        &[first_element, second_element, third_element],
        &mut process,
    );
    let value = 4.into_process(&mut process);

    assert_eq_in_process!(
        erlang::setelement_3(1.into_process(&mut process), tuple, value, &mut process),
        Ok(Term::slice_to_tuple(
            &[value, second_element, third_element],
            &mut process
        )),
        &mut process
    );
    assert_eq_in_process!(
        erlang::setelement_3(2.into_process(&mut process), tuple, value, &mut process),
        Ok(Term::slice_to_tuple(
            &[first_element, value, third_element],
            &mut process
        )),
        &mut process
    );
    assert_eq_in_process!(
        erlang::setelement_3(3.into_process(&mut process), tuple, value, &mut process),
        Ok(Term::slice_to_tuple(
            &[first_element, second_element, value],
            &mut process
        )),
        &mut process
    );
}

#[test]
fn with_map_errors_badarg() {
    let environment_rw_lock: Arc<RwLock<Environment>> = Default::default();
    let process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
    let mut process = process_rw_lock.write().unwrap();
    let tuple = Term::slice_to_map(&[], &mut process);
    let index = 1.into_process(&mut process);
    let value = 4.into_process(&mut process);

    assert_bad_argument!(
        erlang::setelement_3(index, tuple, value, &mut process),
        &mut process
    );
}

#[test]
fn with_heap_binary_is_false() {
    let environment_rw_lock: Arc<RwLock<Environment>> = Default::default();
    let process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
    let mut process = process_rw_lock.write().unwrap();
    let tuple = Term::slice_to_binary(&[], &mut process);
    let index = 1.into_process(&mut process);
    let value = 4.into_process(&mut process);

    assert_bad_argument!(
        erlang::setelement_3(index, tuple, value, &mut process),
        &mut process
    );
}

#[test]
fn with_subbinary_is_false() {
    let environment_rw_lock: Arc<RwLock<Environment>> = Default::default();
    let process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
    let mut process = process_rw_lock.write().unwrap();
    let original = Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
    let tuple = Term::subbinary(original, 0, 7, 2, 1, &mut process);
    let index = 1.into_process(&mut process);
    let value = 4.into_process(&mut process);

    assert_bad_argument!(
        erlang::setelement_3(index, tuple, value, &mut process),
        &mut process
    );
}