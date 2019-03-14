#![cfg_attr(not(test), allow(dead_code))]
///! The memory specific to a process in the VM.
use std::cmp::Ordering;
use std::sync::{Arc, RwLock, Weak};

use num_bigint::BigInt;

use liblumen_arena::TypedArena;

use crate::atom::{self, Existence};
use crate::bad_argument::BadArgument;
use crate::binary::{heap, sub, Binary};
use crate::environment::Environment;
use crate::float::Float;
use crate::integer::{self, big};
use crate::list::List;
use crate::term::Term;
use crate::tuple::Tuple;

pub mod identifier;

pub struct Process {
    // parent pointer, so must be held weakly to prevent cycle with this field and
    // `Environment.process_by_pid`.
    environment: Weak<RwLock<Environment>>,
    pub pid: Term,
    big_integer_arena: TypedArena<big::Integer>,
    pub byte_arena: TypedArena<u8>,
    external_pid_arena: TypedArena<identifier::External>,
    float_arena: TypedArena<Float>,
    pub heap_binary_arena: TypedArena<heap::Binary>,
    pub subbinary_arena: TypedArena<sub::Binary>,
    pub term_arena: TypedArena<Term>,
}

impl Process {
    pub fn new(environment: Arc<RwLock<Environment>>) -> Self {
        Process {
            environment: Arc::downgrade(&Arc::clone(&environment)),
            pid: environment.write().unwrap().next_pid(),
            big_integer_arena: Default::default(),
            byte_arena: Default::default(),
            external_pid_arena: Default::default(),
            float_arena: Default::default(),
            heap_binary_arena: Default::default(),
            subbinary_arena: Default::default(),
            term_arena: Default::default(),
        }
    }

    pub fn atom_index_to_string(&self, atom_index: atom::Index) -> String {
        self.environment
            .upgrade()
            .unwrap()
            .read()
            .unwrap()
            .atom_index_to_string(atom_index)
    }

    /// Combines the two `Term`s into a list `Term`.  The list is only a proper list if the `tail`
    /// is a list `Term` (`Term.tag` is `Tag::List`) or empty list (`Term.tag` is `Tag::EmptyList`).
    pub fn cons(&mut self, head: Term, tail: Term) -> List {
        let mut term_vector = Vec::with_capacity(2);
        term_vector.push(head);
        term_vector.push(tail);

        Term::alloc_slice(term_vector.as_slice(), &mut self.term_arena)
    }

    pub fn external_pid(
        &mut self,
        node: usize,
        number: usize,
        serial: usize,
    ) -> &'static identifier::External {
        let pointer = self
            .external_pid_arena
            .alloc(identifier::External::new(node, number, serial))
            as *const identifier::External;

        unsafe { &*pointer }
    }

    pub fn f64_to_float(&self, f: f64) -> &'static Float {
        let pointer = self.float_arena.alloc(Float::new(f)) as *const Float;

        unsafe { &*pointer }
    }

    pub fn num_bigint_big_in_to_big_integer(&self, big_int: BigInt) -> &'static big::Integer {
        let pointer =
            self.big_integer_arena.alloc(big::Integer::new(big_int)) as *const big::Integer;

        unsafe { &*pointer }
    }

    pub fn subbinary(
        &self,
        original: Term,
        byte_offset: usize,
        bit_offset: u8,
        byte_count: usize,
        bit_count: u8,
    ) -> &'static sub::Binary {
        let pointer = self.subbinary_arena.alloc(sub::Binary::new(
            original,
            byte_offset,
            bit_offset,
            byte_count,
            bit_count,
        )) as *const sub::Binary;

        unsafe { &*pointer }
    }

    pub fn str_to_atom_index(
        &mut self,
        name: &str,
        existence: Existence,
    ) -> Result<atom::Index, BadArgument> {
        self.environment
            .upgrade()
            .unwrap()
            .write()
            .unwrap()
            .str_to_atom_index(name, existence)
    }

    pub fn slice_to_binary(&mut self, slice: &[u8]) -> Binary {
        Binary::from_slice(slice, self)
    }

    pub fn slice_to_tuple(&mut self, slice: &[Term]) -> &Tuple {
        Tuple::from_slice(slice, &mut self.term_arena)
    }
}

/// Like `std::fmt::Debug`, but additionally takes `&Process` in case it is needed to lookup
/// values in the process.
pub trait DebugInProcess {
    fn format_in_process(&self, process: &Process) -> String;
}

impl DebugInProcess for Result<Term, BadArgument> {
    fn format_in_process(&self, process: &Process) -> String {
        match self {
            Ok(term) => format!("Ok({})", term.format_in_process(process)),
            Err(BadArgument { file, line, column }) => format!(
                "Err(BadArgument {{ file: {:?}, line: {:?}, column: {:?} }})",
                file, line, column
            ),
        }
    }
}

/// Like `std::cmp::Ord`, but additionally takes `&Process` in case it is needed to lookup
/// values in the process.
pub trait OrderInProcess<Rhs: ?Sized = Self> {
    /// This method returns an ordering between `self` and `other` values.
    #[must_use]
    fn cmp_in_process(&self, other: &Rhs, process: &Process) -> Ordering;
}

impl OrderInProcess for Result<Term, BadArgument> {
    fn cmp_in_process(&self, other: &Self, process: &Process) -> Ordering {
        match (self, other) {
            (Ok(self_ok), Ok(other_ok)) => self_ok.cmp_in_process(&other_ok, process),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(BadArgument { .. }), Err(BadArgument { .. })) => Ordering::Equal,
        }
    }
}

#[macro_export]
macro_rules! assert_cmp_in_process {
    ($left:expr, $ordering:expr, $right:expr, $process:expr) => ({
        use std::cmp::Ordering;

        use crate::process::{DebugInProcess, OrderInProcess};

        match (&$left, &$ordering, &$right, &$process) {
            (left_val, ordering_val, right_val, process_val) => {
                if !((*left_val).cmp_in_process(right_val, process_val) == *ordering_val) {
                     let ordering_str = match *ordering_val {
                         Ordering::Less => "<",
                         Ordering::Equal => "==",
                         Ordering::Greater => ">"
                     };
                     panic!(r#"assertion failed: `(left {} right)`
  left: `{}`,
 right: `{}`"#,
                       ordering_str,
                       left_val.format_in_process(process_val),
                       right_val.format_in_process(process_val)
                     )
                }
            }
        }
    });
    ($left:expr, $ordering:expr, $right:expr, $process:expr,) => ({
        assert_cmp_in_process!($left, $ordering, $right, $process)
    });
    ($left:expr, $ordering:expr, $right:expr, $process:expr, $($arg:tt)+) => ({
        use std::cmp::Ordering;

        use crate::process::{DebugInProcess, OrderInProcess};

        match (&$left, &$ordering, &$right, &$process) {
            (left_val, ordering_val, right_val, process_val) => {
                if !((*left_val).cmp_in_process(right_val, process_val) == *ordering_val) {
                     let ordering_str = match *ordering_val {
                         Ordering::Less => "<",
                         Ordering::Equal => "==",
                         Ordering::Greater => ">"
                     };
                     panic!(r#"assertion failed: `(left {} right)`
  left: `{}`,
 right: `{}`: {}"#,
                       ordering_str,
                       left_val.format_in_process(process_val),
                       right_val.format_in_process(process_val),
                       format_args!($($arg)+)
                     )
                }
            }
        }
    });
}

#[macro_export]
macro_rules! refute_cmp_in_process {
    ($left:expr, $ordering:expr, $right:expr, $process:expr) => ({
        use std::cmp::Ordering;

        use crate::process::{DebugInProcess, OrderInProcess};

        match (&$left, &$ordering, &$right, &$process) {
            (left_val, ordering_val, right_val, process_val) => {
                if (*left_val).cmp_in_process(right_val, process_val) == *ordering_val {
                     let ordering_str = match *ordering_val {
                         Ordering::Less => ">=",
                         Ordering::Equal => "!=",
                         Ordering::Greater => "<="
                     };
                     panic!(r#"assertion failed: `(left {} right)`
  left: `{}`,
 right: `{}`"#,
                       ordering_str,
                       left_val.format_in_process(process_val),
                       right_val.format_in_process(process_val)
                     )
                }
            }
        }
    });
    ($left:expr, $ordering:expr, $right:expr, $process:expr,) => ({
        assert_cmp_in_process!($left, $ordering, $right, $process)
    });
    ($left:expr, $ordering:expr, $right:expr, $process:expr, $($arg:tt)+) => ({
        use std::cmp::Ordering;

        use crate::process::{DebugInProcess, OrderInProcess};

        match (&$left, &$ordering, &$right, &$process) {
            (left_val, ordering_val, right_val, process_val) => {
                if (*left_val).cmp_in_process(right_val, process_val) == *ordering_val {
                     let ordering_str = match *ordering_val {
                         Ordering::Less => ">=",
                         Ordering::Equal => "!=",
                         Ordering::Greater => "<="
                     };
                     panic!(r#"assertion failed: `(left {} right)`
  left: `{}`,
 right: `{}`: {}"#,
                       ordering_str,
                       left_val.format_in_process(process_val),
                       right_val.format_in_process(process_val),
                       format_args!($($arg)+)
                     )
                }
            }
        }
    });
}

#[macro_export]
macro_rules! assert_eq_in_process {
    ($left:expr, $right:expr, $process:expr) => ({
        assert_cmp_in_process!($left, std::cmp::Ordering::Equal, $right, $process)
    });
    ($left:expr, $right:expr, $process:expr,) => ({
        assert_cmp_in_process!($left, std::cmp::Ordering::Equal, $right, $process)
    });
    ($left:expr, $ordering:expr, $right:expr, $process:expr, $($arg:tt)+) => ({
        assert_cmp_in_process!($left, std::cmp::Ordering::Equal, $right, $process, $($arg)+)
    });
}

#[macro_export]
macro_rules! assert_ne_in_process {
    ($left:expr, $right:expr, $process:expr) => ({
        refute_cmp_in_process!($left, std::cmp::Ordering::Equal, $right, $process)
    });
    ($left:expr, $right:expr, $process:expr,) => ({
        refute_cmp_in_process!($left, std::cmp::Ordering::Equal, $right, $process)
    });
    ($left:expr, $ordering:expr, $right:expr, $process:expr, $($arg:tt)+) => ({
        refute_cmp_in_process!($left, std::cmp::Ordering::Equal, $right, $process, $($arg)+)
    });
}

#[macro_export]
macro_rules! assert_bad_argument {
    ($left:expr, $process:expr) => {{
        assert_eq_in_process!($left, Err(bad_argument!()), $process)
    }};
    ($left:expr, $process:expr,) => {{
        assert_eq_in_process!($left, Err(bad_argument!()), $process)
    }};
}

/// Like `std::convert::Into`, but additionally takes `&mut Process` in case it is needed to
/// lookup or create new values in the `Process`.
pub trait IntoProcess<T> {
    /// Performs the conversion.
    fn into_process(self, process: &mut Process) -> T;
}

impl IntoProcess<Term> for bool {
    fn into_process(self, mut process: &mut Process) -> Term {
        Term::str_to_atom(&self.to_string(), Existence::DoNotCare, &mut process)
            .unwrap()
            .into()
    }
}

impl IntoProcess<Term> for BigInt {
    fn into_process(self, mut process: &mut Process) -> Term {
        let integer: integer::Integer = self.into();

        integer.into_process(&mut process)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod pid {
        use super::*;

        use crate::environment;
        use crate::otp::erlang;

        #[test]
        fn different_processes_in_same_environment_have_different_pids() {
            let environment_rw_lock: Arc<RwLock<Environment>> = Default::default();

            let first_process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
            let first_process = first_process_rw_lock.read().unwrap();

            let second_process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
            let second_process = second_process_rw_lock.read().unwrap();

            assert_ne_in_process!(
                erlang::self_pid(&first_process),
                erlang::self_pid(&second_process),
                first_process
            );
            assert_eq_in_process!(
                erlang::self_pid(&first_process),
                Term::local_pid(0, 0).unwrap(),
                first_process
            );
            assert_eq_in_process!(
                erlang::self_pid(&second_process),
                Term::local_pid(1, 0).unwrap(),
                second_process
            );
        }

        #[test]
        fn number_rolling_over_increments_serial() {
            let environment_rw_lock: Arc<RwLock<Environment>> = Default::default();

            let first_process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
            let first_process = first_process_rw_lock.read().unwrap();

            let mut final_pid = None;

            for _ in 0..identifier::NUMBER_MAX + 1 {
                let process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
                let process = process_rw_lock.read().unwrap();
                final_pid = Some(erlang::self_pid(&process))
            }

            assert_eq_in_process!(
                final_pid.unwrap(),
                Term::local_pid(0, 1).unwrap(),
                first_process
            );
        }
    }

    mod str_to_atom_index {
        use super::*;

        use crate::environment;

        #[test]
        fn without_same_string_have_different_index() {
            let environment_rw_lock: Arc<RwLock<Environment>> = Default::default();
            let process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
            let mut process = process_rw_lock.write().unwrap();

            assert_ne!(
                process
                    .str_to_atom_index("true", Existence::DoNotCare)
                    .unwrap()
                    .0,
                process
                    .str_to_atom_index("false", Existence::DoNotCare)
                    .unwrap()
                    .0
            )
        }

        #[test]
        fn with_same_string_have_same_index() {
            let environment_rw_lock: Arc<RwLock<Environment>> = Default::default();
            let process_rw_lock = environment::process(Arc::clone(&environment_rw_lock));
            let mut process = process_rw_lock.write().unwrap();

            assert_eq!(
                process
                    .str_to_atom_index("atom", Existence::DoNotCare)
                    .unwrap()
                    .0,
                process
                    .str_to_atom_index("atom", Existence::DoNotCare)
                    .unwrap()
                    .0
            )
        }
    }
}
