//! Mirrors [erlang](http://erlang::org/doc/man/erlang::html) module

#![cfg_attr(not(test), allow(dead_code))]

use std::cmp::Ordering;
use std::convert::TryInto;
use std::num::FpCategory;

use crate::atom::Existence;
use crate::binary::{heap, sub, Part};
use crate::integer::big;
use crate::list::Cons;
use crate::process::{IntoProcess, Process};
use crate::term::{BadArgument, Tag, Term};
use crate::tuple::Tuple;

pub fn abs(number: Term, mut process: &mut Process) -> Result<Term, BadArgument> {
    match number.tag() {
        Tag::SmallInteger => {
            if unsafe { number.small_integer_is_negative() } {
                // cast first so that sign bit is extended on shift
                let signed = (number.tagged as isize) >> Tag::SMALL_INTEGER_BIT_COUNT;
                let positive = -signed;
                Ok(Term {
                    tagged: ((positive << Tag::SMALL_INTEGER_BIT_COUNT) as usize)
                        | (Tag::SmallInteger as usize),
                })
            } else {
                Ok(Term {
                    tagged: number.tagged,
                })
            }
        }
        Tag::Boxed => {
            let unboxed: &Term = number.unbox_reference();

            match unboxed.tag() {
                Tag::BigInteger => {
                    let big_integer: &big::Integer = number.unbox_reference();
                    let rug_integer = &big_integer.inner;

                    match rug_integer.cmp0() {
                        Ordering::Less => {
                            let positive_rug_integer = rug_integer.clone().abs();
                            let positive_number: Term =
                                positive_rug_integer.into_process(&mut process);

                            Ok(positive_number)
                        }
                        _ => Ok(number),
                    }
                }
                _ => Err(BadArgument),
            }
        }
        _ => Err(BadArgument),
    }
}

pub fn append_element(
    tuple: Term,
    element: Term,
    process: &mut Process,
) -> Result<Term, BadArgument> {
    let internal: &Tuple = tuple.try_into()?;
    let new_tuple = internal.append_element(element, &mut process.term_arena);

    Ok(new_tuple.into())
}

pub fn atom_to_binary(
    atom: Term,
    encoding: Term,
    mut process: &mut Process,
) -> Result<Term, BadArgument> {
    if let Tag::Atom = atom.tag() {
        encoding.atom_to_encoding(&mut process)?;
        let string = atom.atom_to_string(process);
        Ok(Term::slice_to_binary(string.as_bytes(), &mut process))
    } else {
        Err(BadArgument)
    }
}

pub fn atom_to_list(
    atom: Term,
    encoding: Term,
    mut process: &mut Process,
) -> Result<Term, BadArgument> {
    if let Tag::Atom = atom.tag() {
        encoding.atom_to_encoding(&mut process)?;
        let string = atom.atom_to_string(process);
        Ok(Term::chars_to_list(string.chars(), &mut process))
    } else {
        Err(BadArgument)
    }
}

pub fn binary_part(
    binary: Term,
    start: Term,
    length: Term,
    mut process: &mut Process,
) -> Result<Term, BadArgument> {
    match binary.tag() {
        Tag::Boxed => {
            let unboxed: &Term = binary.unbox_reference();

            match unboxed.tag() {
                Tag::HeapBinary => {
                    let heap_binary: &heap::Binary = binary.unbox_reference();

                    heap_binary.part(start, length, &mut process)
                }
                Tag::Subbinary => {
                    let subbinary: &sub::Binary = binary.unbox_reference();

                    subbinary.part(start, length, &mut process)
                }
                _ => Err(BadArgument),
            }
        }
        _ => Err(BadArgument),
    }
}

pub fn binary_to_atom(
    binary: Term,
    encoding: Term,
    process: &mut Process,
) -> Result<Term, BadArgument> {
    binary_existence_to_atom(binary, encoding, Existence::DoNotCare, process)
}

pub fn binary_to_existing_atom(
    binary: Term,
    encoding: Term,
    process: &mut Process,
) -> Result<Term, BadArgument> {
    binary_existence_to_atom(binary, encoding, Existence::Exists, process)
}

pub fn binary_to_float(binary: Term, mut process: &mut Process) -> Result<Term, BadArgument> {
    let string: String = binary.try_into()?;

    match string.parse::<f64>() {
        Ok(inner) => {
            match inner.classify() {
                FpCategory::Normal | FpCategory::Subnormal =>
                // unlike Rust, Erlang requires float strings to have a decimal point
                {
                    if (inner.fract() == 0.0) & !string.chars().any(|b| b == '.') {
                        Err(BadArgument)
                    } else {
                        Ok(inner.into_process(&mut process))
                    }
                }
                // Erlang has no support for Nan, +inf or -inf
                FpCategory::Nan | FpCategory::Infinite => Err(BadArgument),
                FpCategory::Zero => {
                    // Erlang does not track the difference without +0 and -0.
                    Ok(inner.abs().into_process(&mut process))
                }
            }
        }
        Err(_) => Err(BadArgument),
    }
}

/// `binary_to_integer/1`
pub fn binary_to_integer(binary: Term, mut process: &mut Process) -> Result<Term, BadArgument> {
    let string: String = binary.try_into()?;

    match rug::Integer::parse(string) {
        Ok(incomplete) => {
            let rug_integer = rug::Integer::from(incomplete);
            let term: Term = rug_integer.into_process(&mut process);

            Ok(term)
        }
        Err(_) => Err(BadArgument),
    }
}

/// `binary_to_integer/2`
pub fn binary_in_base_to_integer(
    binary: Term,
    base: Term,
    mut process: &mut Process,
) -> Result<Term, BadArgument> {
    let string: String = binary.try_into()?;
    let radix: usize = base.try_into()?;

    if 2 <= radix && radix <= 36 {
        match rug::Integer::parse_radix(string, radix as i32) {
            Ok(incomplete) => {
                let rug_integer = rug::Integer::from(incomplete);
                let term: Term = rug_integer.into_process(&mut process);

                Ok(term)
            }
            Err(_) => Err(BadArgument),
        }
    } else {
        Err(BadArgument)
    }
}

pub fn delete_element(
    tuple: Term,
    index: Term,
    process: &mut Process,
) -> Result<Term, BadArgument> {
    let initial_inner_tuple: &Tuple = tuple.try_into()?;
    let inner_index: usize = index.try_into()?;

    initial_inner_tuple
        .delete_element(inner_index, &mut process.term_arena)
        .map(|final_inner_tuple| final_inner_tuple.into())
}

pub fn element(tuple: Term, index: Term) -> Result<Term, BadArgument> {
    let inner_tuple: &Tuple = tuple.try_into()?;
    let inner_index: usize = index.try_into()?;

    inner_tuple.element(inner_index)
}

pub fn head(list: Term) -> Result<Term, BadArgument> {
    let cons: &Cons = list.try_into()?;

    Ok(cons.head())
}

pub fn insert_element(
    tuple: Term,
    index: Term,
    element: Term,
    process: &mut Process,
) -> Result<Term, BadArgument> {
    let initial_inner_tuple: &Tuple = tuple.try_into()?;
    let inner_index: usize = index.try_into()?;

    initial_inner_tuple
        .insert_element(inner_index, element, &mut process.term_arena)
        .map(|final_inner_tuple| final_inner_tuple.into())
}

pub fn is_atom(term: Term, mut process: &mut Process) -> Term {
    (term.tag() == Tag::Atom).into_process(&mut process)
}

pub fn is_binary(term: Term, mut process: &mut Process) -> Term {
    match term.tag() {
        Tag::Boxed => {
            let unboxed: &Term = term.unbox_reference();

            match unboxed.tag() {
                Tag::HeapBinary => true,
                Tag::Subbinary => {
                    let subbinary: &sub::Binary = term.unbox_reference();

                    subbinary.is_binary()
                }
                _ => false,
            }
        }
        _ => false,
    }
    .into_process(&mut process)
}

pub fn is_integer(term: Term, mut process: &mut Process) -> Term {
    match term.tag() {
        Tag::SmallInteger => true,
        Tag::Boxed => {
            let unboxed: &Term = term.unbox_reference();

            unboxed.tag() == Tag::BigInteger
        }
        _ => false,
    }
    .into_process(&mut process)
}

pub fn is_list(term: Term, mut process: &mut Process) -> Term {
    match term.tag() {
        Tag::EmptyList | Tag::List => true,
        _ => false,
    }
    .into_process(&mut process)
}

pub fn is_tuple(term: Term, mut process: &mut Process) -> Term {
    (term.tag() == Tag::Boxed && term.unbox_reference::<Term>().tag() == Tag::Arity)
        .into_process(&mut process)
}

pub fn length(list: Term, mut process: &mut Process) -> Result<Term, BadArgument> {
    let mut length: usize = 0;
    let mut tail = list;

    loop {
        match tail.tag() {
            Tag::EmptyList => break Ok(length.into_process(&mut process)),
            Tag::List => {
                tail = crate::otp::erlang::tail(tail).unwrap();
                length += 1;
            }
            _ => break Err(BadArgument),
        }
    }
}

pub fn size(binary_or_tuple: Term, mut process: &mut Process) -> Result<Term, BadArgument> {
    match binary_or_tuple.tag() {
        Tag::Boxed => {
            let unboxed: &Term = binary_or_tuple.unbox_reference();

            match unboxed.tag() {
                Tag::Arity => {
                    let tuple: &Tuple = binary_or_tuple.unbox_reference();

                    Ok(tuple.size())
                }
                Tag::HeapBinary => {
                    let heap_binary: &heap::Binary = binary_or_tuple.unbox_reference();

                    Ok(heap_binary.size())
                }
                Tag::Subbinary => {
                    let subbinary: &sub::Binary = binary_or_tuple.unbox_reference();

                    Ok(subbinary.size())
                }
                _ => Err(BadArgument),
            }
        }
        _ => Err(BadArgument),
    }
    .map(|integer| integer.into_process(&mut process))
}

pub fn tail(list: Term) -> Result<Term, BadArgument> {
    let cons: &Cons = list.try_into()?;

    Ok(cons.tail())
}

// Private Functions

fn binary_existence_to_atom(
    binary: Term,
    encoding: Term,
    existence: Existence,
    mut process: &mut Process,
) -> Result<Term, BadArgument> {
    encoding.atom_to_encoding(&mut process)?;

    match binary.tag() {
        Tag::Boxed => {
            let unboxed: &Term = binary.unbox_reference();

            match unboxed.tag() {
                Tag::HeapBinary => {
                    let heap_binary: &heap::Binary = binary.unbox_reference();

                    heap_binary.to_atom_index(existence, &mut process)
                }
                Tag::Subbinary => {
                    let subbinary: &sub::Binary = binary.unbox_reference();

                    subbinary.to_atom_index(existence, &mut process)
                }
                _ => Err(BadArgument),
            }
        }
        _ => Err(BadArgument),
    }
    .map(|atom_index| atom_index.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::cmp::Ordering;

    use crate::otp::erlang;

    mod abs {
        use super::*;

        use crate::process::IntoProcess;

        #[test]
        fn with_atom_is_bad_argument() {
            let mut process: Process = Default::default();
            let atom_term = Term::str_to_atom("atom", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::abs(atom_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_is_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[0], &mut process);

            assert_eq_in_process!(
                erlang::abs(heap_binary_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_subbinary_is_bad_argument() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);

            assert_eq_in_process!(
                erlang::abs(subbinary_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_empty_list_is_bad_argument() {
            let mut process: Process = Default::default();

            assert_eq_in_process!(
                erlang::abs(Term::EMPTY_LIST, &mut process),
                Err(BadArgument),
                Default::default()
            );
        }

        #[test]
        fn with_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let list_term = list_term(&mut process);

            assert_eq_in_process!(
                erlang::abs(list_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_small_integer_that_is_negative_returns_positive() {
            let mut process: Process = Default::default();

            let negative: isize = -1;
            let negative_term = negative.into_process(&mut process);

            let positive = -negative;
            let positive_term = positive.into_process(&mut process);

            assert_eq_in_process!(
                erlang::abs(negative_term, &mut process),
                Ok(positive_term),
                process
            );
        }

        #[test]
        fn with_small_integer_that_is_positive_returns_self() {
            let mut process: Process = Default::default();
            let positive_term = 1usize.into_process(&mut process);

            assert_eq_in_process!(
                erlang::abs(positive_term, &mut process),
                Ok(positive_term),
                process
            );
        }

        #[test]
        fn with_big_integer_that_is_negative_returns_positive() {
            let mut process: Process = Default::default();

            let negative: isize = -576460752303423489;
            let negative_term = negative.into_process(&mut process);

            assert_eq!(negative_term.tag(), Tag::Boxed);

            let unboxed_negative_term: &Term = negative_term.unbox_reference();

            assert_eq!(unboxed_negative_term.tag(), Tag::BigInteger);

            let positive = -negative;
            let positive_term = positive.into_process(&mut process);

            assert_eq_in_process!(
                erlang::abs(negative_term, &mut process),
                Ok(positive_term),
                process
            );
        }

        #[test]
        fn with_big_integer_that_is_positive_return_self() {
            let mut process: Process = Default::default();
            let positive_term: Term = 576460752303423489_isize.into_process(&mut process);

            assert_eq!(positive_term.tag(), Tag::Boxed);

            let unboxed_positive_term: &Term = positive_term.unbox_reference();

            assert_eq!(unboxed_positive_term.tag(), Tag::BigInteger);

            assert_eq_in_process!(
                erlang::abs(positive_term, &mut process),
                Ok(positive_term),
                process
            );
        }

        #[test]
        fn with_tuple_is_bad_argument() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(&[], &mut process);

            assert_eq_in_process!(
                erlang::abs(tuple_term, &mut process),
                Err(BadArgument),
                process
            );
        }
    }

    mod append_element {
        use super::*;

        #[test]
        fn with_atom_is_bad_argument() {
            let mut process: Process = Default::default();
            let atom_term = Term::str_to_atom("atom", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::append_element(atom_term, 0.into_process(&mut process), &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_empty_list_is_bad_argument() {
            let mut process: Process = Default::default();

            assert_eq_in_process!(
                erlang::append_element(
                    Term::EMPTY_LIST,
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let list_term = list_term(&mut process);

            assert_eq_in_process!(
                erlang::append_element(list_term, 0.into_process(&mut process), &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_small_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let small_integer_term: Term = 0.into_process(&mut process);

            assert_eq_in_process!(
                erlang::append_element(
                    small_integer_term,
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_big_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let big_integer_term: Term = 576460752303423489_isize.into_process(&mut process);

            assert_eq_in_process!(
                erlang::append_element(
                    big_integer_term,
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_tuple_returns_tuple_with_new_element_at_end() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(
                &[0.into_process(&mut process), 1.into_process(&mut process)],
                &mut process,
            );

            assert_eq_in_process!(
                erlang::append_element(tuple_term, 2.into_process(&mut process), &mut process),
                Ok(Term::slice_to_tuple(
                    &[
                        0.into_process(&mut process),
                        1.into_process(&mut process),
                        2.into_process(&mut process)
                    ],
                    &mut process
                )),
                process
            );
        }

        #[test]
        fn with_tuple_with_index_at_size_return_tuples_with_new_element_at_end() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(&[0.into_process(&mut process)], &mut process);

            assert_eq_in_process!(
                erlang::append_element(tuple_term, 1.into_process(&mut process), &mut process),
                Ok(Term::slice_to_tuple(
                    &[0.into_process(&mut process), 1.into_process(&mut process)],
                    &mut process
                )),
                process
            )
        }

        #[test]
        fn with_heap_binary_is_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);

            assert_eq_in_process!(
                erlang::append_element(
                    heap_binary_term,
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_subbinary_is_bad_argument() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);

            assert_eq_in_process!(
                erlang::append_element(subbinary_term, 0.into_process(&mut process), &mut process),
                Err(BadArgument),
                process
            )
        }
    }

    mod atom_to_binary {
        use super::*;

        use crate::process::IntoProcess;

        #[test]
        fn with_atom_without_encoding_atom_returns_bad_argument() {
            let mut process: Process = Default::default();
            let atom_name = "😈";
            let atom_term =
                Term::str_to_atom(atom_name, Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_binary(atom_term, 0.into_process(&mut process), &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_atom_with_invalid_encoding_atom_returns_bad_argument() {
            let mut process: Process = Default::default();
            let atom_name = "😈";
            let atom_term =
                Term::str_to_atom(atom_name, Existence::DoNotCare, &mut process).unwrap();
            let invalid_encoding_atom_term =
                Term::str_to_atom("invalid_encoding", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_binary(atom_term, invalid_encoding_atom_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_atom_with_encoding_atom_returns_name_in_binary() {
            let mut process: Process = Default::default();
            let atom_name = "😈";
            let atom_term =
                Term::str_to_atom(atom_name, Existence::DoNotCare, &mut process).unwrap();
            let latin1_atom_term =
                Term::str_to_atom("latin1", Existence::DoNotCare, &mut process).unwrap();
            let unicode_atom_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();
            let utf8_atom_term =
                Term::str_to_atom("utf8", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_binary(atom_term, latin1_atom_term, &mut process),
                Ok(Term::slice_to_binary(atom_name.as_bytes(), &mut process)),
                process
            );
            assert_eq_in_process!(
                erlang::atom_to_binary(atom_term, unicode_atom_term, &mut process),
                Ok(Term::slice_to_binary(atom_name.as_bytes(), &mut process)),
                process
            );
            assert_eq_in_process!(
                erlang::atom_to_binary(atom_term, utf8_atom_term, &mut process),
                Ok(Term::slice_to_binary(atom_name.as_bytes(), &mut process)),
                process
            );
        }

        #[test]
        fn with_empty_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_binary(Term::EMPTY_LIST, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let list_term = list_term(&mut process);
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_binary(list_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_small_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let small_integer_term = 0usize.into_process(&mut process);
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_binary(small_integer_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_big_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let big_integer_term = 576460752303423489_isize.into_process(&mut process);
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_binary(big_integer_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_tuple_returns_is_bad_argument() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(
                &[0.into_process(&mut process), 1.into_process(&mut process)],
                &mut process,
            );
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_binary(tuple_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_is_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_binary(heap_binary_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_subbinary_is_bad_argument() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_binary(subbinary_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }
    }

    mod atom_to_list {
        use super::*;

        use crate::process::IntoProcess;

        #[test]
        fn with_atom_without_encoding_atom_returns_bad_argument() {
            let mut process: Process = Default::default();
            let atom_name = "😈🤘";
            let atom_term =
                Term::str_to_atom(atom_name, Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_list(atom_term, 0.into_process(&mut process), &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_atom_with_invalid_encoding_atom_returns_bad_argument() {
            let mut process: Process = Default::default();
            let atom_name = "😈🤘";
            let atom_term =
                Term::str_to_atom(atom_name, Existence::DoNotCare, &mut process).unwrap();
            let invalid_encoding_atom_term =
                Term::str_to_atom("invalid_encoding", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_list(atom_term, invalid_encoding_atom_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_atom_with_encoding_atom_returns_chars_in_list() {
            let mut process: Process = Default::default();
            let atom_name = "😈🤘";
            let atom_term =
                Term::str_to_atom(atom_name, Existence::DoNotCare, &mut process).unwrap();
            let latin1_atom_term =
                Term::str_to_atom("latin1", Existence::DoNotCare, &mut process).unwrap();
            let unicode_atom_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();
            let utf8_atom_term =
                Term::str_to_atom("utf8", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_list(atom_term, latin1_atom_term, &mut process),
                Ok(Term::cons(
                    128520.into_process(&mut process),
                    Term::cons(
                        129304.into_process(&mut process),
                        Term::EMPTY_LIST,
                        &mut process
                    ),
                    &mut process
                )),
                process
            );
            assert_eq_in_process!(
                erlang::atom_to_list(atom_term, unicode_atom_term, &mut process),
                Ok(Term::cons(
                    128520.into_process(&mut process),
                    Term::cons(
                        129304.into_process(&mut process),
                        Term::EMPTY_LIST,
                        &mut process
                    ),
                    &mut process
                )),
                process
            );
            assert_eq_in_process!(
                erlang::atom_to_list(atom_term, utf8_atom_term, &mut process),
                Ok(Term::cons(
                    128520.into_process(&mut process),
                    Term::cons(
                        129304.into_process(&mut process),
                        Term::EMPTY_LIST,
                        &mut process
                    ),
                    &mut process
                )),
                process
            );
        }

        #[test]
        fn with_empty_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_list(Term::EMPTY_LIST, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let list_term = list_term(&mut process);
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_list(list_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_small_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let small_integer_term = 0usize.into_process(&mut process);
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_list(small_integer_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_big_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let big_integer_term = 576460752303423489_isize.into_process(&mut process);
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_list(big_integer_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_tuple_returns_is_bad_argument() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(
                &[0.into_process(&mut process), 1.into_process(&mut process)],
                &mut process,
            );
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_list(tuple_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_is_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_list(heap_binary_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_subbinary_is_bad_argument() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::atom_to_list(subbinary_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }
    }

    // binary_part/3
    mod binary_part {
        use super::*;

        use crate::process::IntoProcess;

        #[test]
        fn with_atom_is_bad_argument() {
            let mut process: Process = Default::default();
            let atom_term = Term::str_to_atom("atom", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_part(
                    atom_term,
                    0.into_process(&mut process),
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_empty_list_is_bad_argument() {
            let mut process: Process = Default::default();

            assert_eq_in_process!(
                erlang::binary_part(
                    Term::EMPTY_LIST,
                    0.into_process(&mut process),
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let list_term = list_term(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(
                    list_term,
                    0.into_process(&mut process),
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_small_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let small_integer_term: Term = 0.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(
                    small_integer_term,
                    0.into_process(&mut process),
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_big_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let big_integer_term: Term = 576460752303423489_isize.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(
                    big_integer_term,
                    0.into_process(&mut process),
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_tuple_returns_is_bad_argument() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(
                &[0.into_process(&mut process), 1.into_process(&mut process)],
                &mut process,
            );

            assert_eq_in_process!(
                erlang::binary_part(
                    tuple_term,
                    0.into_process(&mut process),
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_without_integer_start_without_integer_length_returns_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);
            let start_term = Term::slice_to_tuple(
                &[0.into_process(&mut process), 0.into_process(&mut process)],
                &mut process,
            );
            let length_term = Term::str_to_atom("all", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_part(heap_binary_term, start_term, length_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_without_integer_start_with_integer_length_returns_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);
            let start_term = 0.into_process(&mut process);
            let length_term = Term::str_to_atom("all", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_part(heap_binary_term, start_term, length_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_with_integer_start_without_integer_length_returns_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);
            let start_term = 0.into_process(&mut process);
            let length_term = Term::str_to_atom("all", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_part(heap_binary_term, start_term, length_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_with_negative_start_with_valid_length_returns_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);
            let start_term = (-1isize).into_process(&mut process);
            let length_term = 0.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(heap_binary_term, start_term, length_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_with_start_greater_than_size_with_non_negative_length_returns_bad_argument(
        ) {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);
            let start_term = 1.into_process(&mut process);
            let length_term = 0.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(heap_binary_term, start_term, length_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_with_start_less_than_size_with_negative_length_past_start_returns_bad_argument(
        ) {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[0], &mut process);
            let start_term = 0.into_process(&mut process);
            let length_term = (-1isize).into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(heap_binary_term, start_term, length_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_with_start_less_than_size_with_positive_length_past_end_returns_bad_argument(
        ) {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[0], &mut process);
            let start_term = 0.into_process(&mut process);
            let length_term = 2.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(heap_binary_term, start_term, length_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_with_zero_start_and_size_length_returns_binary() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[0], &mut process);
            let start_term = 0.into_process(&mut process);
            let length_term = 1.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(heap_binary_term, start_term, length_term, &mut process),
                Ok(heap_binary_term),
                process
            );

            let returned_binary =
                erlang::binary_part(heap_binary_term, start_term, length_term, &mut process)
                    .unwrap();

            assert_eq!(returned_binary.tagged, heap_binary_term.tagged);
        }

        #[test]
        fn with_heap_binary_with_size_start_and_negative_size_length_returns_binary() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[0], &mut process);
            let start_term = 1.into_process(&mut process);
            let length_term = (-1isize).into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(heap_binary_term, start_term, length_term, &mut process),
                Ok(heap_binary_term),
                process
            );

            let returned_binary =
                erlang::binary_part(heap_binary_term, start_term, length_term, &mut process)
                    .unwrap();

            assert_eq!(returned_binary.tagged, heap_binary_term.tagged);
        }

        #[test]
        fn with_heap_binary_with_positive_start_and_negative_length_returns_subbinary() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[0, 1], &mut process);
            let start_term = 1.into_process(&mut process);
            let length_term = (-1isize).into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(heap_binary_term, start_term, length_term, &mut process),
                Ok(Term::slice_to_binary(&[0], &mut process)),
                process
            );

            let returned_boxed =
                erlang::binary_part(heap_binary_term, start_term, length_term, &mut process)
                    .unwrap();

            assert_eq!(returned_boxed.tag(), Tag::Boxed);

            let returned_unboxed: &Term = returned_boxed.unbox_reference();

            assert_eq!(returned_unboxed.tag(), Tag::Subbinary);
        }

        #[test]
        fn with_heap_binary_with_positive_start_and_positive_length_returns_subbinary() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[0, 1], &mut process);
            let start_term = 1.into_process(&mut process);
            let length_term = 1.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(heap_binary_term, start_term, length_term, &mut process),
                Ok(Term::slice_to_binary(&[1], &mut process)),
                process
            );

            let returned_boxed =
                erlang::binary_part(heap_binary_term, start_term, length_term, &mut process)
                    .unwrap();

            assert_eq!(returned_boxed.tag(), Tag::Boxed);

            let returned_unboxed: &Term = returned_boxed.unbox_reference();

            assert_eq!(returned_unboxed.tag(), Tag::Subbinary);
        }

        #[test]
        fn with_subbinary_without_integer_start_without_integer_length_returns_bad_argument() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);
            let start_term = Term::slice_to_tuple(
                &[0.into_process(&mut process), 0.into_process(&mut process)],
                &mut process,
            );
            let length_term = Term::str_to_atom("all", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_part(subbinary_term, start_term, length_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_subbinary_without_integer_start_with_integer_length_returns_bad_argument() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);
            let start_term = 0.into_process(&mut process);
            let length_term = Term::str_to_atom("all", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_part(subbinary_term, start_term, length_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_subbinary_with_integer_start_without_integer_length_returns_bad_argument() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);
            let start_term = 0.into_process(&mut process);
            let length_term = Term::str_to_atom("all", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_part(subbinary_term, start_term, length_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_subbinary_with_negative_start_with_valid_length_returns_bad_argument() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);
            let start_term = (-1isize).into_process(&mut process);
            let length_term = 0.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(subbinary_term, start_term, length_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_subbinary_with_start_greater_than_size_with_non_negative_length_returns_bad_argument(
        ) {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 0, 1, &mut process);
            let start_term = 1.into_process(&mut process);
            let length_term = 0.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(subbinary_term, start_term, length_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_subbinary_with_start_less_than_size_with_negative_length_past_start_returns_bad_argument(
        ) {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);
            let start_term = 0.into_process(&mut process);
            let length_term = (-1isize).into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(subbinary_term, start_term, length_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_subbinary_with_start_less_than_size_with_positive_length_past_end_returns_bad_argument(
        ) {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 1, 1, &mut process);
            let start_term = 0.into_process(&mut process);
            let length_term = 2.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(subbinary_term, start_term, length_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_subbinary_with_zero_start_and_byte_count_length_returns_new_subbinary_with_bytes() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);
            let start_term = 0.into_process(&mut process);
            let length_term = 2.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(subbinary_term, start_term, length_term, &mut process),
                Ok(Term::subbinary(binary_term, 0, 7, 2, 0, &mut process)),
                process
            );
        }

        #[test]
        fn with_subbinary_with_byte_count_start_and_negative_byte_count_length_returns_new_subbinary_with_bytes(
        ) {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);
            let start_term = 2.into_process(&mut process);
            let length_term = (-2isize).into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(subbinary_term, start_term, length_term, &mut process),
                Ok(Term::subbinary(binary_term, 0, 7, 2, 0, &mut process)),
                process
            );
        }

        #[test]
        fn with_subbinary_with_positive_start_and_negative_length_returns_subbinary() {
            let mut process: Process = Default::default();
            let binary_term = Term::slice_to_binary(&[0b0000_00001, 0b1111_1110], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 1, 0, &mut process);
            let start_term = 1.into_process(&mut process);
            let length_term = (-1isize).into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(subbinary_term, start_term, length_term, &mut process),
                Ok(Term::slice_to_binary(&[0b1111_1111], &mut process)),
                process
            );

            let returned_boxed =
                erlang::binary_part(subbinary_term, start_term, length_term, &mut process).unwrap();

            assert_eq!(returned_boxed.tag(), Tag::Boxed);

            let returned_unboxed: &Term = returned_boxed.unbox_reference();

            assert_eq!(returned_unboxed.tag(), Tag::Subbinary);
        }

        #[test]
        fn with_subbinary_with_positive_start_and_positive_length_returns_subbinary() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term: Term = process.subbinary(binary_term, 0, 7, 2, 1).into();
            let start_term = 1.into_process(&mut process);
            let length_term = 1.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_part(subbinary_term, start_term, length_term, &mut process),
                Ok(Term::slice_to_binary(&[0b0101_0101], &mut process)),
                process
            );

            let returned_boxed =
                erlang::binary_part(subbinary_term, start_term, length_term, &mut process).unwrap();

            assert_eq!(returned_boxed.tag(), Tag::Boxed);

            let returned_unboxed: &Term = returned_boxed.unbox_reference();

            assert_eq!(returned_unboxed.tag(), Tag::Subbinary);
        }
    }

    mod binary_to_atom {
        use super::*;

        use crate::process::IntoProcess;

        #[test]
        fn with_atom_is_bad_argument() {
            let mut process: Process = Default::default();
            let atom_term = Term::str_to_atom("atom", Existence::DoNotCare, &mut process).unwrap();
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_atom(atom_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_empty_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_atom(Term::EMPTY_LIST, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let list_term = list_term(&mut process);
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_atom(list_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_small_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let small_integer_term = 0usize.into_process(&mut process);
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_atom(small_integer_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_big_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let big_integer_term = 576460752303423489_isize.into_process(&mut process);
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_atom(big_integer_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_tuple_returns_is_bad_argument() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(
                &[0.into_process(&mut process), 1.into_process(&mut process)],
                &mut process,
            );
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_atom(tuple_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_without_encoding_atom_returns_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);

            assert_eq_in_process!(
                erlang::binary_to_atom(
                    heap_binary_term,
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_with_invalid_encoding_atom_returns_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);
            let invalid_encoding_term =
                Term::str_to_atom("invalid_encoding", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_atom(heap_binary_term, invalid_encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_with_valid_encoding_returns_atom() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary("😈".as_bytes(), &mut process);
            let latin1_atom_term =
                Term::str_to_atom("latin1", Existence::DoNotCare, &mut process).unwrap();
            let unicode_atom_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();
            let utf8_atom_term =
                Term::str_to_atom("utf8", Existence::DoNotCare, &mut process).unwrap();
            let atom_term = Term::str_to_atom("😈", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_atom(heap_binary_term, latin1_atom_term, &mut process),
                Ok(atom_term),
                &mut process
            );
            assert_eq_in_process!(
                erlang::binary_to_atom(heap_binary_term, unicode_atom_term, &mut process),
                Ok(atom_term),
                &mut process
            );
            assert_eq_in_process!(
                erlang::binary_to_atom(heap_binary_term, utf8_atom_term, &mut process),
                Ok(atom_term),
                &mut process
            );
        }

        #[test]
        fn with_subbinary_with_bit_count_returns_bad_argument() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);
            let unicode_atom_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_atom(subbinary_term, unicode_atom_term, &mut process),
                Err(BadArgument),
                &mut process
            )
        }

        #[test]
        fn with_subbinaty_without_bit_count_returns_atom_with_bytes() {
            let mut process: Process = Default::default();
            let binary_term = Term::slice_to_binary("😈🤘".as_bytes(), &mut process);
            let subbinary_term = Term::subbinary(binary_term, 4, 0, 4, 0, &mut process);
            let unicode_atom_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_atom(subbinary_term, unicode_atom_term, &mut process),
                Term::str_to_atom("🤘", Existence::DoNotCare, &mut process),
                &mut process
            )
        }
    }

    mod binary_to_existing_atom {
        use super::*;

        use crate::process::IntoProcess;

        #[test]
        fn with_atom_is_bad_argument() {
            let mut process: Process = Default::default();
            let atom_term = Term::str_to_atom("atom", Existence::DoNotCare, &mut process).unwrap();
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_existing_atom(atom_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_empty_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_existing_atom(Term::EMPTY_LIST, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let list_term = list_term(&mut process);
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_existing_atom(list_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_small_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let small_integer_term = 0usize.into_process(&mut process);
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_existing_atom(small_integer_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_big_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let big_integer_term = 576460752303423489_isize.into_process(&mut process);
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_existing_atom(big_integer_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_tuple_returns_is_bad_argument() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(
                &[0.into_process(&mut process), 1.into_process(&mut process)],
                &mut process,
            );
            let encoding_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_existing_atom(tuple_term, encoding_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_without_encoding_atom_returns_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);

            assert_eq_in_process!(
                erlang::binary_to_existing_atom(
                    heap_binary_term,
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_with_invalid_encoding_atom_returns_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);
            let invalid_encoding_term =
                Term::str_to_atom("invalid_encoding", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_existing_atom(
                    heap_binary_term,
                    invalid_encoding_term,
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_with_valid_encoding_without_existing_atom_returns_atom() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary("😈".as_bytes(), &mut process);
            let latin1_atom_term =
                Term::str_to_atom("latin1", Existence::DoNotCare, &mut process).unwrap();
            let unicode_atom_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();
            let utf8_atom_term =
                Term::str_to_atom("utf8", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_existing_atom(heap_binary_term, latin1_atom_term, &mut process),
                Err(BadArgument),
                &mut process
            );
            assert_eq_in_process!(
                erlang::binary_to_existing_atom(heap_binary_term, unicode_atom_term, &mut process),
                Err(BadArgument),
                &mut process
            );
            assert_eq_in_process!(
                erlang::binary_to_existing_atom(heap_binary_term, utf8_atom_term, &mut process),
                Err(BadArgument),
                &mut process
            );
        }

        #[test]
        fn with_heap_binary_with_valid_encoding_with_existing_atom_returns_atom() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary("😈".as_bytes(), &mut process);
            let latin1_atom_term =
                Term::str_to_atom("latin1", Existence::DoNotCare, &mut process).unwrap();
            let unicode_atom_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();
            let utf8_atom_term =
                Term::str_to_atom("utf8", Existence::DoNotCare, &mut process).unwrap();
            let atom_term = Term::str_to_atom("😈", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_existing_atom(heap_binary_term, latin1_atom_term, &mut process),
                Ok(atom_term),
                &mut process
            );
            assert_eq_in_process!(
                erlang::binary_to_existing_atom(heap_binary_term, unicode_atom_term, &mut process),
                Ok(atom_term),
                &mut process
            );
            assert_eq_in_process!(
                erlang::binary_to_existing_atom(heap_binary_term, utf8_atom_term, &mut process),
                Ok(atom_term),
                &mut process
            );
        }

        #[test]
        fn with_subbinary_with_bit_count_returns_bad_argument() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);
            let unicode_atom_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_existing_atom(subbinary_term, unicode_atom_term, &mut process),
                Err(BadArgument),
                &mut process
            )
        }

        #[test]
        fn with_subbinary_without_bit_count_without_existing_atom_returns_bad_argument() {
            let mut process: Process = Default::default();
            let binary_term = Term::slice_to_binary("😈🤘".as_bytes(), &mut process);
            let subbinary_term = Term::subbinary(binary_term, 4, 0, 4, 0, &mut process);
            let unicode_atom_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_existing_atom(subbinary_term, unicode_atom_term, &mut process),
                Err(BadArgument),
                &mut process
            )
        }

        #[test]
        fn with_subbinary_without_bit_count_with_existing_atom_returns_atom_with_bytes() {
            let mut process: Process = Default::default();
            let binary_term = Term::slice_to_binary("😈🤘".as_bytes(), &mut process);
            let subbinary_term = Term::subbinary(binary_term, 4, 0, 4, 0, &mut process);
            let unicode_atom_term =
                Term::str_to_atom("unicode", Existence::DoNotCare, &mut process).unwrap();
            let atom_term = Term::str_to_atom("🤘", Existence::DoNotCare, &mut process);

            assert_eq_in_process!(
                erlang::binary_to_existing_atom(subbinary_term, unicode_atom_term, &mut process),
                atom_term,
                &mut process
            )
        }
    }

    mod binary_to_float {
        use super::*;

        use crate::process::IntoProcess;

        #[test]
        fn with_atom_returns_bad_argument() {
            let mut process: Process = Default::default();
            let atom_term =
                Term::str_to_atom("😈🤘", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_float(atom_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_empty_list_returns_bad_argument() {
            let mut process: Process = Default::default();

            assert_eq_in_process!(
                erlang::binary_to_float(Term::EMPTY_LIST, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let list_term = list_term(&mut process);

            assert_eq_in_process!(
                erlang::binary_to_float(list_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_small_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let small_integer_term = 0usize.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_to_float(small_integer_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_big_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let big_integer_term: Term =
                rug::Integer::from(rug::Integer::parse("18446744073709551616").unwrap())
                    .into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_to_float(big_integer_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_tuple_returns_is_bad_argument() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(&[], &mut process);

            assert_eq_in_process!(
                erlang::binary_to_float(tuple_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_with_integer_returns_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary("1".as_bytes(), &mut process);

            assert_eq_in_process!(
                erlang::binary_to_float(heap_binary_term, &mut process),
                Err(BadArgument),
                process
            )
        }

        #[test]
        fn with_heap_binary_with_min_f64_returns_float() {
            let mut process: Process = Default::default();
            let heap_binary_term =
                Term::slice_to_binary("-179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000.0".as_bytes(), &mut process);

            assert_eq_in_process!(
                erlang::binary_to_float(heap_binary_term, &mut process),
                Ok((-179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000.0_f64).into_process(&mut process)),
                process
            );
        }

        #[test]
        fn with_heap_binary_with_max_f64_returns_float() {
            let mut process: Process = Default::default();
            let heap_binary_term =
                Term::slice_to_binary("179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000.0".as_bytes(), &mut process);

            assert_eq_in_process!(
                erlang::binary_to_float(heap_binary_term, &mut process),
                Ok(179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000.0_f64.into_process(&mut process)),
                process
            );
        }

        #[test]
        fn with_heap_binary_with_less_than_min_f64_returns_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term =
                Term::slice_to_binary("-1797693134862315700000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000.0".as_bytes(), &mut process);

            assert_eq_in_process!(
                erlang::binary_to_float(heap_binary_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_with_greater_than_max_f64_returns_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term =
                Term::slice_to_binary("17976931348623157000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000.0".as_bytes(), &mut process);

            assert_eq_in_process!(
                erlang::binary_to_float(heap_binary_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_subbinary_with_min_f64_returns_float() {
            let mut process: Process = Default::default();
            // <<1::1, "-179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000.0">>
            let heap_binary_term = Term::slice_to_binary(
                &[
                    150,
                    152,
                    155,
                    156,
                    155,
                    155,
                    28,
                    153,
                    152,
                    153,
                    154,
                    28,
                    27,
                    25,
                    25,
                    152,
                    154,
                    155,
                    152,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    23,
                    24,
                    0b0000_0000,
                ],
                &mut process,
            );
            let subbinary_term = Term::subbinary(heap_binary_term, 0, 1, 312, 0, &mut process);

            assert_eq_in_process!(
                erlang::binary_to_float(subbinary_term, &mut process),
                Ok((-179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000.0_f64).into_process(&mut process)),
                process
            );
        }

        #[test]
        fn with_subbinary_with_max_f64_returns_float() {
            let mut process: Process = Default::default();
            // <<1::1, "179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000.0">>
            let heap_binary_term = Term::slice_to_binary(
                &[
                    152,
                    155,
                    156,
                    155,
                    155,
                    28,
                    153,
                    152,
                    153,
                    154,
                    28,
                    27,
                    25,
                    25,
                    152,
                    154,
                    155,
                    152,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    23,
                    24,
                    0b0000_0000,
                ],
                &mut process,
            );
            let subbinary_term = Term::subbinary(heap_binary_term, 0, 1, 311, 0, &mut process);

            assert_eq_in_process!(
                erlang::binary_to_float(subbinary_term, &mut process),
                Ok(179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000.0_f64.into_process(&mut process)),
                process
            );
        }

        #[test]
        fn with_subbinary_with_less_than_min_f64_returns_bag_argument() {
            let mut process: Process = Default::default();
            // <<1::1, "-1797693134862315700000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000.0">>
            let heap_binary_term = Term::slice_to_binary(
                &[
                    150,
                    152,
                    155,
                    156,
                    155,
                    155,
                    28,
                    153,
                    152,
                    153,
                    154,
                    28,
                    27,
                    25,
                    25,
                    152,
                    154,
                    155,
                    152,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    23,
                    24,
                    0b0000_0000,
                ],
                &mut process,
            );
            let subbinary_term = Term::subbinary(heap_binary_term, 0, 1, 313, 0, &mut process);

            assert_eq_in_process!(
                erlang::binary_to_float(subbinary_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_subbinary_with_greater_than_max_f64_returns_bad_argument() {
            let mut process: Process = Default::default();
            // <<1::1, "576460752303423488">>
            let heap_binary_term = Term::slice_to_binary(
                &[
                    150,
                    152,
                    155,
                    156,
                    155,
                    155,
                    28,
                    153,
                    152,
                    153,
                    154,
                    28,
                    27,
                    25,
                    25,
                    152,
                    154,
                    155,
                    152,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    23,
                    24,
                    0b0000_0000,
                ],
                &mut process,
            );
            let subbinary_term = Term::subbinary(heap_binary_term, 0, 1, 313, 0, &mut process);

            assert_eq_in_process!(
                erlang::binary_to_float(subbinary_term, &mut process),
                Err(BadArgument),
                process
            );
        }
    }

    mod binary_to_integer {
        use super::*;

        use crate::process::IntoProcess;

        #[test]
        fn with_atom_returns_bad_argument() {
            let mut process: Process = Default::default();
            let atom_term =
                Term::str_to_atom("😈🤘", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::binary_to_integer(atom_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_empty_list_returns_bad_argument() {
            let mut process: Process = Default::default();

            assert_eq_in_process!(
                erlang::binary_to_integer(Term::EMPTY_LIST, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let list_term = list_term(&mut process);

            assert_eq_in_process!(
                erlang::binary_to_integer(list_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_small_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let small_integer_term = 0usize.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_to_integer(small_integer_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_big_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let big_integer_term: Term =
                rug::Integer::from(rug::Integer::parse("18446744073709551616").unwrap())
                    .into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_to_integer(big_integer_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_tuple_returns_is_bad_argument() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(&[], &mut process);

            assert_eq_in_process!(
                erlang::binary_to_integer(tuple_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_with_min_small_integer_returns_small_integer() {
            let mut process: Process = Default::default();
            let heap_binary_term =
                Term::slice_to_binary("-576460752303423488".as_bytes(), &mut process);

            let integer_result = erlang::binary_to_integer(heap_binary_term, &mut process);

            assert_eq_in_process!(
                integer_result,
                Ok((-576460752303423488_isize).into_process(&mut process)),
                process
            );
            assert_eq!(integer_result.unwrap().tag(), Tag::SmallInteger);
        }

        #[test]
        fn with_heap_binary_with_max_small_integer_returns_small_integer() {
            let mut process: Process = Default::default();
            let heap_binary_term =
                Term::slice_to_binary("576460752303423487".as_bytes(), &mut process);

            let integer_result = erlang::binary_to_integer(heap_binary_term, &mut process);

            assert_eq_in_process!(
                integer_result,
                Ok(576460752303423487usize.into_process(&mut process)),
                process
            );
            assert_eq!(integer_result.unwrap().tag(), Tag::SmallInteger);
        }

        #[test]
        fn with_heap_binary_with_less_than_min_small_integer_returns_big_integer() {
            let mut process: Process = Default::default();
            let heap_binary_term =
                Term::slice_to_binary("-576460752303423489".as_bytes(), &mut process);

            let integer_result = erlang::binary_to_integer(heap_binary_term, &mut process);

            assert_eq_in_process!(
                integer_result,
                Ok((-576460752303423489_isize).into_process(&mut process)),
                process
            );

            let integer = integer_result.unwrap();

            assert_eq!(integer.tag(), Tag::Boxed);

            let unboxed: &Term = integer.unbox_reference();

            assert_eq!(unboxed.tag(), Tag::BigInteger);
        }

        #[test]
        fn with_heap_binary_with_greater_than_max_small_integer_returns_big_integer() {
            let mut process: Process = Default::default();
            let heap_binary_term =
                Term::slice_to_binary("576460752303423488".as_bytes(), &mut process);

            let integer_result = erlang::binary_to_integer(heap_binary_term, &mut process);

            assert_eq_in_process!(
                integer_result,
                Ok(576460752303423488_usize.into_process(&mut process)),
                process
            );

            let integer = integer_result.unwrap();

            assert_eq!(integer.tag(), Tag::Boxed);

            let unboxed: &Term = integer.unbox_reference();

            assert_eq!(unboxed.tag(), Tag::BigInteger);
        }

        #[test]
        fn with_heap_binary_with_non_decimal_returns_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary("FF".as_bytes(), &mut process);

            assert_eq_in_process!(
                erlang::binary_to_integer(heap_binary_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_subbinary_with_min_small_integer_returns_small_integer() {
            let mut process: Process = Default::default();
            // <<1::1, "-576460752303423488">>
            let heap_binary_term = Term::slice_to_binary(
                &[
                    150,
                    154,
                    155,
                    155,
                    26,
                    27,
                    24,
                    27,
                    154,
                    153,
                    25,
                    152,
                    25,
                    154,
                    25,
                    25,
                    154,
                    28,
                    28,
                    0b0000_0000,
                ],
                &mut process,
            );
            let subbinary_term = Term::subbinary(heap_binary_term, 0, 1, 19, 0, &mut process);

            let integer_result = erlang::binary_to_integer(subbinary_term, &mut process);

            assert_eq_in_process!(
                integer_result,
                Ok((-576460752303423488_isize).into_process(&mut process)),
                process
            );
            assert_eq!(integer_result.unwrap().tag(), Tag::SmallInteger);
        }

        #[test]
        fn with_subbinary_with_max_small_integer_returns_small_integer() {
            let mut process: Process = Default::default();
            // <<1::1, "576460752303423487">>
            let heap_binary_term = Term::slice_to_binary(
                &[
                    154,
                    155,
                    155,
                    26,
                    27,
                    24,
                    27,
                    154,
                    153,
                    25,
                    152,
                    25,
                    154,
                    25,
                    25,
                    154,
                    28,
                    27,
                    0b1000_0000,
                ],
                &mut process,
            );
            let subbinary_term = Term::subbinary(heap_binary_term, 0, 1, 18, 0, &mut process);

            let integer_result = erlang::binary_to_integer(subbinary_term, &mut process);

            assert_eq_in_process!(
                integer_result,
                Ok(576460752303423487_isize.into_process(&mut process)),
                process
            );
            assert_eq!(integer_result.unwrap().tag(), Tag::SmallInteger);
        }

        #[test]
        fn with_subbinary_with_less_than_min_small_integer_returns_big_integer() {
            let mut process: Process = Default::default();
            // <<1::1, "-576460752303423489">>
            let heap_binary_term = Term::slice_to_binary(
                &[
                    150,
                    154,
                    155,
                    155,
                    26,
                    27,
                    24,
                    27,
                    154,
                    153,
                    25,
                    152,
                    25,
                    154,
                    25,
                    25,
                    154,
                    28,
                    28,
                    0b1000_0000,
                ],
                &mut process,
            );
            let subbinary_term = Term::subbinary(heap_binary_term, 0, 1, 19, 0, &mut process);

            let integer_result = erlang::binary_to_integer(subbinary_term, &mut process);

            assert_eq_in_process!(
                integer_result,
                Ok((-576460752303423489_isize).into_process(&mut process)),
                process
            );

            let integer = integer_result.unwrap();

            assert_eq!(integer.tag(), Tag::Boxed);

            let unboxed: &Term = integer.unbox_reference();

            assert_eq!(unboxed.tag(), Tag::BigInteger);
        }

        #[test]
        fn with_subbinary_with_greater_than_max_small_integer_returns_big_integer() {
            let mut process: Process = Default::default();
            // <<1::1, "576460752303423488">>
            let heap_binary_term = Term::slice_to_binary(
                &[
                    154,
                    155,
                    155,
                    26,
                    27,
                    24,
                    27,
                    154,
                    153,
                    25,
                    152,
                    25,
                    154,
                    25,
                    25,
                    154,
                    28,
                    28,
                    0b0000_0000,
                ],
                &mut process,
            );
            let subbinary_term = Term::subbinary(heap_binary_term, 0, 1, 18, 0, &mut process);

            let integer_result = erlang::binary_to_integer(subbinary_term, &mut process);

            assert_eq_in_process!(
                integer_result,
                Ok(576460752303423488_usize.into_process(&mut process)),
                process
            );

            let integer = integer_result.unwrap();

            assert_eq!(integer.tag(), Tag::Boxed);

            let unboxed: &Term = integer.unbox_reference();

            assert_eq!(unboxed.tag(), Tag::BigInteger);
        }

        #[test]
        fn with_subbinary_with_non_decimal_returns_bad_argument() {
            let mut process: Process = Default::default();
            // <<1:1, "FF>>
            let heap_binary_term = Term::slice_to_binary(&[163, 35, 0b000_0000], &mut process);
            let subbinary_term = Term::subbinary(heap_binary_term, 0, 1, 2, 0, &mut process);

            assert_eq_in_process!(
                erlang::binary_to_integer(subbinary_term, &mut process),
                Err(BadArgument),
                process
            );
        }
    }

    mod binary_in_base_to_integer {
        use super::*;

        use crate::process::IntoProcess;

        #[test]
        fn with_atom_returns_bad_argument() {
            let mut process: Process = Default::default();
            let atom_term =
                Term::str_to_atom("😈🤘", Existence::DoNotCare, &mut process).unwrap();
            let base_term: Term = 16.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_in_base_to_integer(atom_term, base_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_empty_list_returns_bad_argument() {
            let mut process: Process = Default::default();
            let base_term: Term = 16.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_in_base_to_integer(Term::EMPTY_LIST, base_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let list_term = list_term(&mut process);
            let base_term: Term = 16.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_in_base_to_integer(list_term, base_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_small_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let small_integer_term = 0usize.into_process(&mut process);
            let base_term: Term = 16.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_in_base_to_integer(small_integer_term, base_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_big_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let big_integer_term: Term =
                rug::Integer::from(rug::Integer::parse("18446744073709551616").unwrap())
                    .into_process(&mut process);
            let base_term: Term = 16.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_in_base_to_integer(big_integer_term, base_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_tuple_returns_is_bad_argument() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(&[], &mut process);
            let base_term: Term = 16.into_process(&mut process);

            assert_eq_in_process!(
                erlang::binary_in_base_to_integer(tuple_term, base_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_with_min_small_integer_returns_small_integer() {
            let mut process: Process = Default::default();
            let heap_binary_term =
                Term::slice_to_binary("-800000000000000".as_bytes(), &mut process);
            let base_term: Term = 16.into_process(&mut process);

            let integer_result =
                erlang::binary_in_base_to_integer(heap_binary_term, base_term, &mut process);

            assert_eq_in_process!(
                integer_result,
                Ok((-576460752303423488_isize).into_process(&mut process)),
                process
            );
            assert_eq!(integer_result.unwrap().tag(), Tag::SmallInteger);
        }

        #[test]
        fn with_heap_binary_with_max_small_integer_returns_small_integer() {
            let mut process: Process = Default::default();
            let heap_binary_term =
                Term::slice_to_binary("7FFFFFFFFFFFFFF".as_bytes(), &mut process);
            let base_term: Term = 16.into_process(&mut process);

            let integer_result =
                erlang::binary_in_base_to_integer(heap_binary_term, base_term, &mut process);

            assert_eq_in_process!(
                integer_result,
                Ok(576460752303423487usize.into_process(&mut process)),
                process
            );
            assert_eq!(integer_result.unwrap().tag(), Tag::SmallInteger);
        }

        #[test]
        fn with_heap_binary_with_less_than_min_small_integer_returns_big_integer() {
            let mut process: Process = Default::default();
            let heap_binary_term =
                Term::slice_to_binary("-800000000000001".as_bytes(), &mut process);
            let base_term: Term = 16.into_process(&mut process);

            let integer_result =
                erlang::binary_in_base_to_integer(heap_binary_term, base_term, &mut process);

            assert_eq_in_process!(
                integer_result,
                Ok((-576460752303423489_isize).into_process(&mut process)),
                process
            );

            let integer = integer_result.unwrap();

            assert_eq!(integer.tag(), Tag::Boxed);

            let unboxed: &Term = integer.unbox_reference();

            assert_eq!(unboxed.tag(), Tag::BigInteger);
        }

        #[test]
        fn with_heap_binary_with_greater_than_max_small_integer_returns_big_integer() {
            let mut process: Process = Default::default();
            let heap_binary_term =
                Term::slice_to_binary("800000000000000".as_bytes(), &mut process);
            let base_term: Term = 16.into_process(&mut process);

            let integer_result =
                erlang::binary_in_base_to_integer(heap_binary_term, base_term, &mut process);

            assert_eq_in_process!(
                integer_result,
                Ok(576460752303423488_usize.into_process(&mut process)),
                process
            );

            let integer = integer_result.unwrap();

            assert_eq!(integer.tag(), Tag::Boxed);

            let unboxed: &Term = integer.unbox_reference();

            assert_eq!(unboxed.tag(), Tag::BigInteger);
        }

        #[test]
        fn with_subbinary_with_min_small_integer_returns_small_integer() {
            let mut process: Process = Default::default();
            // <<1::1, Integer.to_string(-576460752303423488, 16) :: binary>>
            let heap_binary_term = Term::slice_to_binary(
                &[
                    150,
                    156,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    0b0000_0000,
                ],
                &mut process,
            );
            let subbinary_term = Term::subbinary(heap_binary_term, 0, 1, 16, 0, &mut process);
            let base_term: Term = 16.into_process(&mut process);

            let integer_result =
                erlang::binary_in_base_to_integer(subbinary_term, base_term, &mut process);

            assert_eq_in_process!(
                integer_result,
                Ok((-576460752303423488_isize).into_process(&mut process)),
                process
            );
            assert_eq!(integer_result.unwrap().tag(), Tag::SmallInteger);
        }

        #[test]
        fn with_subbinary_with_max_small_integer_returns_small_integer() {
            let mut process: Process = Default::default();
            // <<1::1, Integer.to_string(576460752303423487, 16) :: binary>>
            let heap_binary_term = Term::slice_to_binary(
                &[
                    155,
                    163,
                    35,
                    35,
                    35,
                    35,
                    35,
                    35,
                    35,
                    35,
                    35,
                    35,
                    35,
                    35,
                    35,
                    0b0000_0000,
                ],
                &mut process,
            );
            let subbinary_term = Term::subbinary(heap_binary_term, 0, 1, 15, 0, &mut process);
            let base_term: Term = 16.into_process(&mut process);

            let integer_result =
                erlang::binary_in_base_to_integer(subbinary_term, base_term, &mut process);

            assert_eq_in_process!(
                integer_result,
                Ok(576460752303423487_isize.into_process(&mut process)),
                process
            );
            assert_eq!(integer_result.unwrap().tag(), Tag::SmallInteger);
        }

        #[test]
        fn with_subbinary_with_less_than_min_small_integer_returns_big_integer() {
            let mut process: Process = Default::default();
            // <<1::1, Integer.to_string(-576460752303423489, 16) :: binary>>
            let heap_binary_term = Term::slice_to_binary(
                &[
                    150,
                    156,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    0b1000_0000,
                ],
                &mut process,
            );
            let subbinary_term = Term::subbinary(heap_binary_term, 0, 1, 16, 0, &mut process);
            let base_term: Term = 16.into_process(&mut process);

            let integer_result =
                erlang::binary_in_base_to_integer(subbinary_term, base_term, &mut process);

            assert_eq_in_process!(
                integer_result,
                Ok((-576460752303423489_isize).into_process(&mut process)),
                process
            );

            let integer = integer_result.unwrap();

            assert_eq!(integer.tag(), Tag::Boxed);

            let unboxed: &Term = integer.unbox_reference();

            assert_eq!(unboxed.tag(), Tag::BigInteger);
        }

        #[test]
        fn with_subbinary_with_greater_than_max_small_integer_returns_big_integer() {
            let mut process: Process = Default::default();
            // <<1::1, Integer.to_string(576460752303423488, 16) :: binary>>
            let heap_binary_term = Term::slice_to_binary(
                &[
                    156,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    24,
                    0b0000_0000,
                ],
                &mut process,
            );
            let subbinary_term = Term::subbinary(heap_binary_term, 0, 1, 15, 0, &mut process);
            let base_term: Term = 16.into_process(&mut process);

            let integer_result =
                erlang::binary_in_base_to_integer(subbinary_term, base_term, &mut process);

            assert_eq_in_process!(
                integer_result,
                Ok(576460752303423488_usize.into_process(&mut process)),
                process
            );

            let integer = integer_result.unwrap();

            assert_eq!(integer.tag(), Tag::Boxed);

            let unboxed: &Term = integer.unbox_reference();

            assert_eq!(unboxed.tag(), Tag::BigInteger);
        }
    }

    mod delete_element {
        use super::*;

        #[test]
        fn with_atom_is_bad_argument() {
            let mut process: Process = Default::default();
            let atom_term = Term::str_to_atom("atom", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::delete_element(atom_term, 0.into_process(&mut process), &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_empty_list_is_bad_argument() {
            let mut process: Process = Default::default();

            assert_eq_in_process!(
                erlang::delete_element(
                    Term::EMPTY_LIST,
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let list_term = list_term(&mut process);

            assert_eq_in_process!(
                erlang::delete_element(list_term, 0.into_process(&mut process), &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_small_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let small_integer_term: Term = 0.into_process(&mut process);

            assert_eq_in_process!(
                erlang::delete_element(
                    small_integer_term,
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_big_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let big_integer_term: Term = 576460752303423489_isize.into_process(&mut process);

            assert_eq_in_process!(
                erlang::delete_element(
                    big_integer_term,
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_tuple_without_small_integer_index_is_bad_argument() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(
                &[
                    0.into_process(&mut process),
                    1.into_process(&mut process),
                    2.into_process(&mut process),
                ],
                &mut process,
            );
            let index = 1usize;
            let invalid_index_term = Term::arity(index);

            assert_ne!(invalid_index_term.tag(), Tag::SmallInteger);
            assert_eq_in_process!(
                erlang::delete_element(tuple_term, invalid_index_term, &mut process),
                Err(BadArgument),
                process
            );

            let valid_index_term: Term = index.into_process(&mut process);

            assert_eq!(valid_index_term.tag(), Tag::SmallInteger);
            assert_eq_in_process!(
                erlang::delete_element(tuple_term, valid_index_term, &mut process),
                Ok(Term::slice_to_tuple(
                    &[0.into_process(&mut process), 2.into_process(&mut process)],
                    &mut process
                )),
                process
            );
        }

        #[test]
        fn with_tuple_without_index_in_range_is_bad_argument() {
            let mut process: Process = Default::default();
            let empty_tuple_term = Term::slice_to_tuple(&[], &mut process);

            assert_eq_in_process!(
                erlang::delete_element(
                    empty_tuple_term,
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_tuple_with_index_in_range_returns_tuple_without_element() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(
                &[
                    0.into_process(&mut process),
                    1.into_process(&mut process),
                    2.into_process(&mut process),
                ],
                &mut process,
            );

            assert_eq_in_process!(
                erlang::delete_element(tuple_term, 1.into_process(&mut process), &mut process),
                Ok(Term::slice_to_tuple(
                    &[0.into_process(&mut process), 2.into_process(&mut process)],
                    &mut process
                )),
                process
            );
        }

        #[test]
        fn with_heap_binary_is_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);

            assert_eq_in_process!(
                erlang::delete_element(
                    heap_binary_term,
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_subbinary_is_bad_argument() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);

            assert_eq_in_process!(
                erlang::delete_element(subbinary_term, 0.into_process(&mut process), &mut process),
                Err(BadArgument),
                process
            );
        }
    }

    mod element {
        use super::*;

        #[test]
        fn with_atom_is_bad_argument() {
            let mut process: Process = Default::default();
            let atom_term = Term::str_to_atom("atom", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::element(atom_term, 0.into_process(&mut process)),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_empty_list_is_bad_argument() {
            let mut process: Process = Default::default();

            assert_eq_in_process!(
                erlang::element(Term::EMPTY_LIST, 0.into_process(&mut process)),
                Err(BadArgument),
                Default::default()
            );
        }

        #[test]
        fn with_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let list_term = list_term(&mut process);

            assert_eq_in_process!(
                erlang::element(list_term, 0.into_process(&mut process)),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_small_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let small_integer_term: Term = 0.into_process(&mut process);

            assert_eq_in_process!(
                erlang::element(small_integer_term, 0.into_process(&mut process)),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_big_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let big_integer_term: Term = 576460752303423489_isize.into_process(&mut process);

            assert_eq_in_process!(
                erlang::element(big_integer_term, 0.into_process(&mut process)),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_tuple_without_small_integer_index_is_bad_argument() {
            let mut process: Process = Default::default();
            let element_term = 1.into_process(&mut process);
            let tuple_term = Term::slice_to_tuple(&[element_term], &mut process);
            let index = 0usize;
            let invalid_index_term = Term::arity(index);

            assert_ne!(invalid_index_term.tag(), Tag::SmallInteger);
            assert_eq_in_process!(
                erlang::element(tuple_term, invalid_index_term),
                Err(BadArgument),
                process
            );

            let valid_index_term: Term = index.into_process(&mut process);

            assert_eq!(valid_index_term.tag(), Tag::SmallInteger);
            assert_eq_in_process!(
                erlang::element(tuple_term, valid_index_term),
                Ok(element_term),
                process
            );
        }

        #[test]
        fn with_tuple_without_index_in_range_is_bad_argument() {
            let mut process: Process = Default::default();
            let empty_tuple_term = Term::slice_to_tuple(&[], &mut process);

            assert_eq_in_process!(
                erlang::element(empty_tuple_term, 0.into_process(&mut process)),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_tuple_with_index_in_range_is_element() {
            let mut process: Process = Default::default();
            let element_term = 1.into_process(&mut process);
            let tuple_term = Term::slice_to_tuple(&[element_term], &mut process);

            assert_eq_in_process!(
                erlang::element(tuple_term, 0.into_process(&mut process)),
                Ok(element_term),
                process
            );
        }

        #[test]
        fn with_heap_binary_is_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);

            assert_eq_in_process!(
                erlang::element(heap_binary_term, 0.into_process(&mut process)),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_subbinary_is_bad_argument() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);

            assert_eq_in_process!(
                erlang::element(subbinary_term, 0.into_process(&mut process)),
                Err(BadArgument),
                process
            );
        }
    }

    mod head {
        use super::*;

        #[test]
        fn with_atom_is_bad_argument() {
            let mut process: Process = Default::default();
            let atom_term = Term::str_to_atom("atom", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(erlang::head(atom_term), Err(BadArgument), process);
        }

        #[test]
        fn with_empty_list_is_bad_argument() {
            let empty_list_term = Term::EMPTY_LIST;

            assert_eq_in_process!(
                erlang::head(empty_list_term),
                Err(BadArgument),
                Default::default()
            );
        }

        #[test]
        fn with_list_returns_head() {
            let mut process: Process = Default::default();
            let head_term = Term::str_to_atom("head", Existence::DoNotCare, &mut process).unwrap();
            let list_term = Term::cons(head_term, Term::EMPTY_LIST, &mut process);

            assert_eq_in_process!(erlang::head(list_term), Ok(head_term), process);
        }

        #[test]
        fn with_small_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let small_integer_term = 0.into_process(&mut process);

            assert_eq_in_process!(erlang::head(small_integer_term), Err(BadArgument), process);
        }

        #[test]
        fn with_big_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let big_integer_term = 576460752303423489_isize.into_process(&mut process);

            assert_eq_in_process!(erlang::head(big_integer_term), Err(BadArgument), process);
        }

        #[test]
        fn with_tuple_is_bad_argument() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(&[], &mut process);

            assert_eq_in_process!(erlang::head(tuple_term), Err(BadArgument), process);
        }

        #[test]
        fn with_heap_binary_is_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);

            assert_eq_in_process!(erlang::head(heap_binary_term), Err(BadArgument), process);
        }

        #[test]
        fn with_subbinary_is_bad_argument() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);

            assert_eq_in_process!(erlang::head(subbinary_term), Err(BadArgument), process);
        }
    }

    mod insert_element {
        use super::*;

        #[test]
        fn with_atom_is_bad_argument() {
            let mut process: Process = Default::default();
            let atom_term = Term::str_to_atom("atom", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::insert_element(
                    atom_term,
                    0.into_process(&mut process),
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_empty_list_is_bad_argument() {
            let mut process: Process = Default::default();

            assert_eq_in_process!(
                erlang::insert_element(
                    Term::EMPTY_LIST,
                    0.into_process(&mut process),
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let list_term = list_term(&mut process);

            assert_eq_in_process!(
                erlang::insert_element(
                    list_term,
                    0.into_process(&mut process),
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_small_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let small_integer_term = 0.into_process(&mut process);

            assert_eq_in_process!(
                erlang::insert_element(
                    small_integer_term,
                    0.into_process(&mut process),
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_big_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let big_integer_term = 576460752303423489_isize.into_process(&mut process);

            assert_eq_in_process!(
                erlang::insert_element(
                    big_integer_term,
                    0.into_process(&mut process),
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_tuple_without_small_integer_index_is_bad_argument() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(
                &[0.into_process(&mut process), 2.into_process(&mut process)],
                &mut process,
            );
            let index = 1usize;
            let invalid_index_term = Term::arity(index);

            assert_ne!(invalid_index_term.tag(), Tag::SmallInteger);
            assert_eq_in_process!(
                erlang::insert_element(
                    tuple_term,
                    invalid_index_term,
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );

            let valid_index_term: Term = index.into_process(&mut process);

            assert_eq!(valid_index_term.tag(), Tag::SmallInteger);
            assert_eq_in_process!(
                erlang::insert_element(
                    tuple_term,
                    valid_index_term,
                    1.into_process(&mut process),
                    &mut process
                ),
                Ok(Term::slice_to_tuple(
                    &[
                        0.into_process(&mut process),
                        1.into_process(&mut process),
                        2.into_process(&mut process)
                    ],
                    &mut process
                )),
                process
            );
        }

        #[test]
        fn with_tuple_without_index_in_range_is_bad_argument() {
            let mut process: Process = Default::default();
            let empty_tuple_term = Term::slice_to_tuple(&[], &mut process);

            assert_eq_in_process!(
                erlang::insert_element(
                    empty_tuple_term,
                    1.into_process(&mut process),
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_tuple_with_index_in_range_returns_tuple_with_new_element_at_index() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(
                &[0.into_process(&mut process), 2.into_process(&mut process)],
                &mut process,
            );

            assert_eq_in_process!(
                erlang::insert_element(
                    tuple_term,
                    1.into_process(&mut process),
                    1.into_process(&mut process),
                    &mut process
                ),
                Ok(Term::slice_to_tuple(
                    &[
                        0.into_process(&mut process),
                        1.into_process(&mut process),
                        2.into_process(&mut process)
                    ],
                    &mut process
                )),
                process
            );
        }

        #[test]
        fn with_tuple_with_index_at_size_return_tuples_with_new_element_at_end() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(&[0.into_process(&mut process)], &mut process);

            assert_eq_in_process!(
                erlang::insert_element(
                    tuple_term,
                    1.into_process(&mut process),
                    1.into_process(&mut process),
                    &mut process
                ),
                Ok(Term::slice_to_tuple(
                    &[0.into_process(&mut process), 1.into_process(&mut process)],
                    &mut process
                )),
                process
            )
        }

        #[test]
        fn with_heap_binary_is_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);

            assert_eq_in_process!(
                erlang::insert_element(
                    heap_binary_term,
                    0.into_process(&mut process),
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_subbinary_is_bad_argument() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);

            assert_eq_in_process!(
                erlang::insert_element(
                    subbinary_term,
                    0.into_process(&mut process),
                    0.into_process(&mut process),
                    &mut process
                ),
                Err(BadArgument),
                process
            );
        }
    }

    mod is_atom {
        use super::*;

        use crate::process::IntoProcess;

        #[test]
        fn with_atom_is_true() {
            let mut process: Process = Default::default();
            let atom_term = Term::str_to_atom("atom", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::is_atom(atom_term, &mut process),
                true.into_process(&mut process),
                process
            );
        }

        #[test]
        fn with_booleans_is_true() {
            let mut process: Process = Default::default();
            let true_term = true.into_process(&mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(erlang::is_atom(true_term, &mut process), true_term, process);
            assert_eq_in_process!(
                erlang::is_atom(false_term, &mut process),
                true_term,
                process
            );
        }

        #[test]
        fn with_nil_is_true() {
            let mut process: Process = Default::default();
            let nil_term = Term::str_to_atom("nil", Existence::DoNotCare, &mut process).unwrap();
            let true_term = true.into_process(&mut process);

            assert_eq_in_process!(erlang::is_atom(nil_term, &mut process), true_term, process);
        }

        #[test]
        fn with_empty_list_is_false() {
            let mut process: Process = Default::default();
            let empty_list_term = Term::EMPTY_LIST;
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_atom(empty_list_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_list_is_false() {
            let mut process: Process = Default::default();
            let head_term = Term::str_to_atom("head", Existence::DoNotCare, &mut process).unwrap();
            let list_term = Term::cons(head_term, Term::EMPTY_LIST, &mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_atom(list_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_small_integer_is_false() {
            let mut process: Process = Default::default();
            let small_integer_term = 0.into_process(&mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_atom(small_integer_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_big_integer_is_false() {
            let mut process: Process = Default::default();
            let big_integer_term = 576460752303423489_isize.into_process(&mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_atom(big_integer_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_tuple_is_false() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(&[], &mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_atom(tuple_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_heap_binary_is_false() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_atom(heap_binary_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_subbinary_is_false() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_atom(subbinary_term, &mut process),
                false_term,
                process
            );
        }
    }

    mod is_binary {
        use super::*;

        use crate::process::IntoProcess;

        #[test]
        fn with_atom_is_false() {
            let mut process: Process = Default::default();
            let atom_term = Term::str_to_atom("atom", Existence::DoNotCare, &mut process).unwrap();
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_binary(atom_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_empty_list_is_false() {
            let mut process: Process = Default::default();
            let empty_list_term = Term::EMPTY_LIST;
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_binary(empty_list_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_list_is_false() {
            let mut process: Process = Default::default();
            let head_term = Term::str_to_atom("head", Existence::DoNotCare, &mut process).unwrap();
            let list_term = Term::cons(head_term, Term::EMPTY_LIST, &mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_binary(list_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_small_integer_is_false() {
            let mut process: Process = Default::default();
            let small_integer_term = 0.into_process(&mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_binary(small_integer_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_big_integer_is_false() {
            let mut process: Process = Default::default();
            let big_integer_term = 576460752303423489_isize.into_process(&mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_binary(big_integer_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_tuple_is_false() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(&[], &mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_binary(tuple_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_heap_binary_is_true() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);
            let true_term = true.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_binary(heap_binary_term, &mut process),
                true_term,
                process
            );
        }

        #[test]
        fn with_subbinary_with_bit_count_is_false() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_binary(subbinary_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_subbinary_without_bit_count_is_true() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 0, &mut process);
            let true_term = true.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_binary(subbinary_term, &mut process),
                true_term,
                process
            );
        }
    }

    mod is_integer {
        use super::*;

        use crate::process::IntoProcess;

        #[test]
        fn with_atom_is_false() {
            let mut process: Process = Default::default();
            let atom_term = Term::str_to_atom("atom", Existence::DoNotCare, &mut process).unwrap();
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_integer(atom_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_empty_list_is_false() {
            let mut process: Process = Default::default();
            let empty_list_term = Term::EMPTY_LIST;
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_integer(empty_list_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_list_is_false() {
            let mut process: Process = Default::default();
            let list_term = list_term(&mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_integer(list_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_small_integer_is_true() {
            let mut process: Process = Default::default();
            let zero_term = 0usize.into_process(&mut process);
            let true_term = true.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_integer(zero_term, &mut process),
                true_term,
                process
            );
        }

        #[test]
        fn with_big_integer_is_true() {
            let mut process: Process = Default::default();
            let big_integer_term = 576460752303423489_isize.into_process(&mut process);
            let true_term = true.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_integer(big_integer_term, &mut process),
                true_term,
                process
            );
        }

        #[test]
        fn with_tuple_is_false() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(&[], &mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_integer(tuple_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_heap_binary_is_false() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_integer(heap_binary_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_subbinary_is_false() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_integer(subbinary_term, &mut process),
                false_term,
                process
            );
        }
    }

    mod is_list {
        use super::*;

        use crate::process::IntoProcess;

        #[test]
        fn with_atom_is_false() {
            let mut process: Process = Default::default();
            let atom_term = Term::str_to_atom("atom", Existence::DoNotCare, &mut process).unwrap();
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_list(atom_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_empty_list_is_true() {
            let mut process: Process = Default::default();
            let empty_list_term = Term::EMPTY_LIST;
            let true_term = true.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_list(empty_list_term, &mut process),
                true_term,
                process
            );
        }

        #[test]
        fn with_list_is_true() {
            let mut process: Process = Default::default();
            let list_term = list_term(&mut process);
            let true_term = true.into_process(&mut process);

            assert_eq_in_process!(erlang::is_list(list_term, &mut process), true_term, process);
        }

        #[test]
        fn with_small_integer_is_false() {
            let mut process: Process = Default::default();
            let small_integer_term = 0.into_process(&mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_list(small_integer_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_big_integer_is_false() {
            let mut process: Process = Default::default();
            let big_integer_term = 576460752303423489_isize.into_process(&mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_list(big_integer_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_tuple_is_false() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(&[], &mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_list(tuple_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_heap_binary_is_false() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_list(heap_binary_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_subbinary_is_false() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_list(subbinary_term, &mut process),
                false_term,
                process
            );
        }
    }

    mod is_tuple {
        use super::*;

        use crate::process::IntoProcess;

        #[test]
        fn with_atom_is_false() {
            let mut process: Process = Default::default();
            let atom_term = Term::str_to_atom("atom", Existence::DoNotCare, &mut process).unwrap();
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_tuple(atom_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_empty_list_is_false() {
            let mut process: Process = Default::default();
            let empty_list_term = Term::EMPTY_LIST;
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_tuple(empty_list_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_list_is_false() {
            let mut process: Process = Default::default();
            let list_term = list_term(&mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_tuple(list_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_small_integer_is_false() {
            let mut process: Process = Default::default();
            let small_integer_term = 0.into_process(&mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_tuple(small_integer_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_big_integer_is_false() {
            let mut process: Process = Default::default();
            let big_integer_term = 576460752303423489_isize.into_process(&mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_tuple(big_integer_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_tuple_is_true() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(&[], &mut process);
            let true_term = true.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_tuple(tuple_term, &mut process),
                true_term,
                process
            );
        }

        #[test]
        fn with_heap_binary_is_false() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_tuple(heap_binary_term, &mut process),
                false_term,
                process
            );
        }

        #[test]
        fn with_subbinary_is_false() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);
            let false_term = false.into_process(&mut process);

            assert_eq_in_process!(
                erlang::is_tuple(subbinary_term, &mut process),
                false_term,
                process
            );
        }
    }

    mod length {
        use super::*;

        #[test]
        fn with_atom_is_bad_argument() {
            let mut process: Process = Default::default();
            let atom_term = Term::str_to_atom("atom", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::length(atom_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_empty_list_is_zero() {
            let mut process: Process = Default::default();
            let zero_term = 0.into_process(&mut process);

            assert_eq_in_process!(
                erlang::length(Term::EMPTY_LIST, &mut process),
                Ok(zero_term),
                process
            );
        }

        #[test]
        fn with_improper_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let head_term = Term::str_to_atom("head", Existence::DoNotCare, &mut process).unwrap();
            let tail_term = Term::str_to_atom("tail", Existence::DoNotCare, &mut process).unwrap();
            let improper_list_term = Term::cons(head_term, tail_term, &mut process);

            assert_eq_in_process!(
                erlang::length(improper_list_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_list_is_length() {
            let mut process: Process = Default::default();
            let list_term = (0..=2).rfold(Term::EMPTY_LIST, |acc, i| {
                Term::cons(i.into_process(&mut process), acc, &mut process)
            });

            assert_eq_in_process!(
                erlang::length(list_term, &mut process),
                Ok(3.into_process(&mut process)),
                process
            );
        }

        #[test]
        fn with_small_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let small_integer_term = 0.into_process(&mut process);

            assert_eq_in_process!(
                erlang::length(small_integer_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_big_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let big_integer_term = 576460752303423489_isize.into_process(&mut process);

            assert_eq_in_process!(
                erlang::length(big_integer_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_tuple_is_bad_argument() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(&[], &mut process);

            assert_eq_in_process!(
                erlang::length(tuple_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_heap_binary_is_false() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);

            assert_eq_in_process!(
                erlang::length(heap_binary_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_subbinary_is_false() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);

            assert_eq_in_process!(
                erlang::length(subbinary_term, &mut process),
                Err(BadArgument),
                process
            );
        }
    }

    mod size {
        use super::*;

        use crate::process::IntoProcess;

        #[test]
        fn with_atom_is_bad_argument() {
            let mut process: Process = Default::default();
            let atom_term = Term::str_to_atom("atom", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(
                erlang::size(atom_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_empty_list_is_bad_argument() {
            let mut process: Process = Default::default();

            assert_eq_in_process!(
                erlang::size(Term::EMPTY_LIST, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_list_is_bad_argument() {
            let mut process: Process = Default::default();
            let list_term = list_term(&mut process);

            assert_eq_in_process!(
                erlang::size(list_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_small_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let small_integer_term = 0usize.into_process(&mut process);

            assert_eq_in_process!(
                erlang::size(small_integer_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_big_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let big_integer_term = 576460752303423489_isize.into_process(&mut process);

            assert_eq_in_process!(
                erlang::size(big_integer_term, &mut process),
                Err(BadArgument),
                process
            );
        }

        #[test]
        fn with_tuple_without_elements_is_zero() {
            let mut process: Process = Default::default();
            let empty_tuple_term = Term::slice_to_tuple(&[], &mut process);
            let zero_term = 0usize.into_process(&mut process);

            assert_eq_in_process!(
                erlang::size(empty_tuple_term, &mut process),
                Ok(zero_term),
                process
            );
        }

        #[test]
        fn with_tuple_with_elements_is_element_count() {
            let mut process: Process = Default::default();
            let element_vec: Vec<Term> =
                (0..=2usize).map(|i| i.into_process(&mut process)).collect();
            let element_slice: &[Term] = element_vec.as_slice();
            let tuple_term = Term::slice_to_tuple(element_slice, &mut process);
            let arity_term = 3usize.into_process(&mut process);

            assert_eq_in_process!(
                erlang::size(tuple_term, &mut process),
                Ok(arity_term),
                process
            );
        }

        #[test]
        fn with_heap_binary_is_byte_count() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[0, 1, 2], &mut process);
            let byte_count_term = 3usize.into_process(&mut process);

            assert_eq_in_process!(
                erlang::size(heap_binary_term, &mut process),
                Ok(byte_count_term),
                process
            );
        }

        #[test]
        fn with_subbinary_with_bit_count_is_byte_count() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);
            let byte_count_term = 2usize.into_process(&mut process);

            assert_eq_in_process!(
                erlang::size(subbinary_term, &mut process),
                Ok(byte_count_term),
                process
            );
        }

        #[test]
        fn with_subbinary_without_bit_count_is_byte_count() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 0, &mut process);
            let byte_count_term = 2usize.into_process(&mut process);

            assert_eq_in_process!(
                erlang::size(subbinary_term, &mut process),
                Ok(byte_count_term),
                process
            );
        }
    }

    mod tail {
        use super::*;

        #[test]
        fn with_atom_is_bad_argument() {
            let mut process: Process = Default::default();
            let atom_term = Term::str_to_atom("atom", Existence::DoNotCare, &mut process).unwrap();

            assert_eq_in_process!(erlang::tail(atom_term), Err(BadArgument), process);
        }

        #[test]
        fn with_empty_list_is_bad_argument() {
            let empty_list_term = Term::EMPTY_LIST;

            assert_eq_in_process!(
                erlang::tail(empty_list_term),
                Err(BadArgument),
                Default::default()
            );
        }

        #[test]
        fn with_list_returns_tail() {
            let mut process: Process = Default::default();
            let head_term = Term::str_to_atom("head", Existence::DoNotCare, &mut process).unwrap();
            let list_term = Term::cons(head_term, Term::EMPTY_LIST, &mut process);

            assert_eq_in_process!(erlang::tail(list_term), Ok(Term::EMPTY_LIST), process);
        }

        #[test]
        fn with_small_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let small_integer_term = 0.into_process(&mut process);

            assert_eq_in_process!(erlang::tail(small_integer_term), Err(BadArgument), process);
        }

        #[test]
        fn with_big_integer_is_bad_argument() {
            let mut process: Process = Default::default();
            let big_integer_term = 576460752303423489_isize.into_process(&mut process);

            assert_eq_in_process!(erlang::tail(big_integer_term), Err(BadArgument), process);
        }

        #[test]
        fn with_tuple_is_bad_argument() {
            let mut process: Process = Default::default();
            let tuple_term = Term::slice_to_tuple(&[], &mut process);

            assert_eq_in_process!(erlang::tail(tuple_term), Err(BadArgument), process);
        }

        #[test]
        fn with_heap_binary_is_bad_argument() {
            let mut process: Process = Default::default();
            let heap_binary_term = Term::slice_to_binary(&[], &mut process);

            assert_eq_in_process!(erlang::tail(heap_binary_term), Err(BadArgument), process);
        }

        #[test]
        fn with_subbinary_is_bad_argument() {
            let mut process: Process = Default::default();
            let binary_term =
                Term::slice_to_binary(&[0b0000_00001, 0b1111_1110, 0b1010_1011], &mut process);
            let subbinary_term = Term::subbinary(binary_term, 0, 7, 2, 1, &mut process);

            assert_eq_in_process!(erlang::tail(subbinary_term), Err(BadArgument), process);
        }
    }

    fn list_term(mut process: &mut Process) -> Term {
        let head_term = Term::str_to_atom("head", Existence::DoNotCare, &mut process).unwrap();
        Term::cons(head_term, Term::EMPTY_LIST, process)
    }
}
