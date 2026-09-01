//! Consumer tests for structural callable aliases and same-name call macros.

use these_macros_should_be_illegal::{callable, make_fn};

#[callable(apply)]
trait Action {
    /// Applies this action to one point.
    fn apply(&self, point: usize) -> usize;
}

#[derive(Clone, Debug, PartialEq)]
struct Shift(usize);

impl Action for Shift {
    fn apply(&self, point: usize) -> usize {
        point + self.0
    }
}

impl Shift {
    fn inverse(&self) -> Self {
        Self(usize::MAX - self.0 + 1)
    }
}

#[callable(advance)]
trait Advance {
    /// Advances this counter and returns its new value.
    fn advance(&mut self, amount: usize) -> usize;
}

struct Counter(usize);

impl Advance for Counter {
    fn advance(&mut self, amount: usize) -> usize {
        self.0 += amount;
        self.0
    }
}

#[callable(evaluate)]
trait GenericAction<T, const OFFSET: usize> {
    /// Evaluates an action retaining trait-level type and const parameters.
    fn evaluate(&self, value: T) -> T;
}

struct AddOffset;

impl GenericAction<usize, 4> for AddOffset {
    fn evaluate(&self, value: usize) -> usize {
        value + 4
    }
}

#[callable(identity)]
trait GenericMethod {
    /// Returns a value of a method-level generic type unchanged.
    fn identity<T>(&self, value: T) -> T;
}

struct Identity;

impl GenericMethod for Identity {
    fn identity<T>(&self, value: T) -> T {
        value
    }
}

#[callable(evaluate_later)]
trait AsyncAction {
    /// Evaluates an action asynchronously.
    async fn evaluate_later(&self, value: usize) -> usize;
}

struct AsyncIdentity;

impl AsyncAction for AsyncIdentity {
    async fn evaluate_later(&self, value: usize) -> usize {
        value
    }
}

#[callable(read)]
trait UnsafeAction {
    /// Returns the supplied value through an unsafe calling contract.
    unsafe fn read(&self, value: usize) -> usize;
}

struct UnsafeIdentity;

impl UnsafeAction for UnsafeIdentity {
    unsafe fn read(&self, value: usize) -> usize {
        value
    }
}

#[callable(disabled)]
#[allow(dead_code)]
trait ConditionallyCallable {
    /// Exists only when its deliberately false configuration is enabled.
    #[cfg(any())]
    fn disabled(&self, value: usize) -> usize;
}

/// Function-like syntax does not erase the original object's methods or type.
#[test]
fn calls_an_object_without_erasing_its_structure() {
    make_fn!(sigma = Shift(3));

    assert_eq!(sigma!(2), 5);
    assert_eq!(sigma.inverse(), Shift(usize::MAX - 2));
}

/// Mutable receiver aliases use the mutability of the generated local binding.
#[test]
fn calls_a_mutable_object_repeatedly() {
    make_fn!(mut counter = Counter(0));

    assert_eq!(counter!(2), 2);
    assert_eq!(counter!(3), 5);
}

/// Trait generic arguments are retained by the generated qualified call.
#[test]
fn calls_an_implementation_of_a_generic_trait() {
    make_fn!(action = AddOffset);

    assert_eq!(action!(3), 7);
}

/// Method-level generic arguments are forwarded explicitly by the alias.
#[test]
fn calls_a_generic_method() {
    make_fn!(identity = Identity);

    assert_eq!(identity!(String::from("value")), "value");
}

/// Async and unsafe qualifiers remain part of the generated alias contract.
#[test]
fn preserves_async_and_unsafe_method_qualifiers() {
    make_fn!(asynchronous = AsyncIdentity);
    make_fn!(unsafe_identity = UnsafeIdentity);

    let future = asynchronous!(3);
    drop(future);
    // SAFETY: `UnsafeIdentity::read` returns its value without dereferencing or
    // otherwise relying on an additional caller invariant.
    assert_eq!(unsafe { unsafe_identity!(4) }, 4);
}

/// Attribute expansion and the generated alias also work on block-local items.
#[test]
fn expands_on_block_local_items() {
    #[callable(apply)]
    trait LocalAction {
        /// Applies the local action.
        fn apply(&self, value: usize) -> usize;
    }

    struct LocalShift;

    impl LocalAction for LocalShift {
        fn apply(&self, value: usize) -> usize {
            value + 1
        }
    }

    make_fn!(local = LocalShift);
    assert_eq!(local!(4), 5);
}
