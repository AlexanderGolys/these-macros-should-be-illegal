//! Consumer tests for ownership and assignment overloads of an operator.

use std::ops::{Add, AddAssign, Mul, Neg, Not, Sub};
use these_macros_should_be_illegal::overload_op;

#[derive(Clone, Debug, PartialEq)]
struct Vector(f64, f64);

#[overload_op]
impl Add<&Vector> for &Vector {
    type Output = Vector;

    fn add(self, rhs: &Vector) -> Vector {
        Vector(self.0 + rhs.0, self.1 + rhs.1)
    }
}

#[overload_op]
impl Neg for &Vector {
    type Output = Vector;

    fn neg(self) -> Vector {
        Vector(-self.0, -self.1)
    }
}

#[overload_op]
impl Mul<&f64> for &Vector {
    type Output = Vector;

    fn mul(self, rhs: &f64) -> Vector {
        Vector(self.0 * rhs, self.1 * rhs)
    }
}

/// `Not` is unary and has no augmented form in the standard library, as `Neg` has none.
#[derive(Clone, Debug, PartialEq)]
struct Mask(bool);

#[overload_op]
impl Not for &Mask {
    type Output = Mask;

    fn not(self) -> Mask {
        Mask(!self.0)
    }
}

/// Subtracting points gives a different type, so no assignment applies.
#[derive(Clone, Debug, PartialEq)]
struct Point(f64);

#[overload_op]
impl Sub<&Point> for &Point {
    type Output = Vector;

    fn sub(self, rhs: &Point) -> Vector {
        Vector(self.0 - rhs.0, 0.0)
    }
}

/// An operator naming no right operand takes `Self`, which is the borrowed operand.
#[derive(Clone, Debug, PartialEq)]
struct Count(u32);

#[overload_op]
impl Add for &Count {
    type Output = Count;

    fn add(self, rhs: &Count) -> Count {
        Count(self.0 + rhs.0)
    }
}

/// A generic container forwarding the operator of its element type.
#[derive(Clone, Debug, PartialEq)]
struct Pair<T>(T, T);

#[overload_op]
impl<T> Add<&Pair<T>> for &Pair<T>
where
    for<'b> &'b T: Add<&'b T, Output = T>,
{
    type Output = Pair<T>;

    fn add(self, rhs: &Pair<T>) -> Pair<T> {
        Pair(&self.0 + &rhs.0, &self.1 + &rhs.1)
    }
}

/// An impl naming its operand lifetimes explicitly rather than eliding them.
#[derive(Clone, Debug, PartialEq)]
struct Tagged(u32);

#[overload_op]
impl<'a> Add<&'a Tagged> for &'a Tagged {
    type Output = Tagged;

    fn add(self, rhs: &'a Tagged) -> Tagged {
        Tagged(self.0 + rhs.0)
    }
}

/// An operator trait outside `std::ops` following the assignment naming convention.
trait Join<Rhs> {
    /// Result of joining both operands.
    type Output;
    /// Joins this value with the right operand.
    fn join(self, rhs: Rhs) -> Self::Output;
}

/// Augmented assignment of [`Join`], named by the convention the macro relies on.
trait JoinAssign<Rhs> {
    /// Joins the right operand into this value.
    fn join_assign(&mut self, rhs: Rhs);
}

#[derive(Clone, Debug, PartialEq)]
struct Segments(String);

#[overload_op]
impl Join<&Segments> for &Segments {
    type Output = Segments;

    fn join(self, rhs: &Segments) -> Segments {
        Segments(format!("{}/{}", self.0, rhs.0))
    }
}

#[test]
fn repeats_a_binary_operator_across_owned_operands() {
    let left = Vector(1.0, 2.0);
    let right = Vector(3.0, 4.0);
    let expected = Vector(4.0, 6.0);

    assert_eq!(&left + &right, expected);
    assert_eq!(left.clone() + &right, expected);
    assert_eq!(left + right, expected);
}

#[test]
fn derives_augmented_assignment_from_the_binary_operator() {
    let mut total = Vector(1.0, 2.0);

    total += &Vector(3.0, 4.0);
    assert_eq!(total, Vector(4.0, 6.0));

    total += Vector(1.0, 1.0);
    assert_eq!(total, Vector(5.0, 7.0));
}

#[test]
fn repeats_a_unary_operator_for_an_owned_operand() {
    let vector = Vector(1.0, 2.0);

    assert_eq!(-&vector, Vector(-1.0, -2.0));
    assert_eq!(-vector, Vector(-1.0, -2.0));
}

/// Borrowing the right operand is the overload under test, not a redundant borrow.
#[allow(clippy::op_ref)]
#[test]
fn overloads_an_operator_over_a_foreign_right_operand() {
    let vector = Vector(1.0, 2.0);

    assert_eq!(&vector * &2.0, Vector(2.0, 4.0));
    assert_eq!(vector.clone() * &2.0, Vector(2.0, 4.0));
    assert_eq!(vector.clone() * 2.0, Vector(2.0, 4.0));

    let mut scaled = vector;
    scaled *= 3.0;
    assert_eq!(scaled, Vector(3.0, 6.0));
}

#[test]
fn repeats_a_unary_operator_that_has_no_augmented_form() {
    let mask = Mask(true);

    assert_eq!(!&mask, Mask(false));
    assert_eq!(!mask, Mask(false));
}

#[test]
fn skips_assignment_when_the_operator_changes_type() {
    let left = Point(5.0);
    let right = Point(2.0);
    let expected = Vector(3.0, 0.0);

    assert_eq!(&left - &right, expected);
    assert_eq!(left.clone() - &right, expected);
    assert_eq!(left - right, expected);
}

#[test]
fn takes_the_right_operand_from_an_elided_self() {
    let mut total = Count(1);

    assert_eq!(Count(1) + Count(2), Count(3));
    assert_eq!(Count(1) + &Count(2), Count(3));

    total += Count(4);
    assert_eq!(total, Count(5));
}

#[test]
fn retains_generic_parameters_and_predicates() {
    let left = Pair(1u32, 2u32);
    let right = Pair(3u32, 4u32);
    let expected = Pair(4u32, 6u32);

    assert_eq!(&left + &right, expected);
    assert_eq!(left.clone() + &right, expected);
    assert_eq!(left + right, expected);

    let mut total = Pair(1u32, 1u32);
    total += Pair(2u32, 3u32);
    assert_eq!(total, Pair(3u32, 4u32));
}

#[test]
fn drops_lifetimes_the_generated_header_no_longer_names() {
    let mut tagged = Tagged(1);

    assert_eq!(Tagged(1) + Tagged(2), Tagged(3));
    assert_eq!(Tagged(1) + &Tagged(2), Tagged(3));

    tagged += Tagged(4);
    assert_eq!(tagged, Tagged(5));
}

#[test]
fn overloads_an_operator_trait_outside_the_standard_library() {
    let left = Segments(String::from("usr"));
    let right = Segments(String::from("local"));
    let expected = Segments(String::from("usr/local"));

    assert_eq!(left.clone().join(&right), expected);
    assert_eq!(left.clone().join(right.clone()), expected);

    let mut path = left;
    path.join_assign(right);
    assert_eq!(path, expected);
}

/// The generated assignment is the ordinary one, reachable through the trait.
#[test]
fn generates_the_standard_assignment_trait() {
    let mut total = Vector(1.0, 2.0);

    AddAssign::add_assign(&mut total, Vector(1.0, 1.0));
    assert_eq!(total, Vector(2.0, 3.0));
}

/// A type whose name is also exported by `core::ops`, which the overloads must not shadow.
#[derive(Clone, Debug, PartialEq)]
struct Range(f64);

#[overload_op]
impl Add<&Range> for &Range {
    type Output = Range;

    fn add(self, rhs: &Range) -> Range {
        Range(self.0 + rhs.0)
    }
}

/// The generated assignment impl resolves operands in the caller's scope, not `core::ops`.
#[test]
fn keeps_operand_names_that_core_ops_also_exports() {
    let mut range = Range(1.0);
    range += Range(2.0);

    assert_eq!(range, Range(3.0));
}

/// A payload carried by reference, bounded inline by the operand's own lifetime.
#[derive(Clone, Debug, PartialEq)]
struct Boxed<T>(T);

#[overload_op]
impl<'a, T: 'a + Copy + Add<T, Output = T>> Add<&'a Boxed<T>> for &'a Boxed<T> {
    type Output = Boxed<T>;

    fn add(self, rhs: &'a Boxed<T>) -> Boxed<T> {
        Boxed(self.0 + rhs.0)
    }
}

/// An inline bound naming the operand lifetime does not outlive the header that drops it.
#[test]
fn drops_inline_bounds_stated_on_the_operand_lifetime() {
    assert_eq!(Boxed(2u8) + Boxed(3u8), Boxed(5u8));
}

/// A component-wise operator whose predicate names the operand lifetime directly.
#[derive(Clone, Debug, PartialEq)]
struct Coupled<T>(T, T);

#[overload_op]
impl<'a, T> Add<&'a Coupled<T>> for &'a Coupled<T>
where
    &'a T: Add<&'a T, Output = T>,
{
    type Output = Coupled<T>;

    fn add(self, rhs: &'a Coupled<T>) -> Coupled<T> {
        Coupled(&self.0 + &rhs.0, &self.1 + &rhs.1)
    }
}

/// A predicate naming the operand lifetime is requantified rather than discarded.
#[test]
fn requantifies_predicates_stated_on_the_operand_lifetime() {
    assert_eq!(
        Coupled(1u32, 2u32) + Coupled(3u32, 4u32),
        Coupled(4u32, 6u32)
    );
}

/// A type reached through a module, so the impl can name it two different ways.
mod qualified {
    /// Result type written unqualified in one operand position and qualified in the other.
    #[derive(Clone, Debug, PartialEq)]
    pub struct Amount(pub f64);
}

use qualified::Amount;

#[overload_op]
impl Add<&Amount> for &qualified::Amount {
    type Output = Amount;

    fn add(self, rhs: &Amount) -> Amount {
        Amount(self.0 + rhs.0)
    }
}

/// Differing path qualifiers on one type still derive the assignment overloads.
#[test]
fn assigns_in_place_across_differing_path_qualifiers() {
    let mut amount = Amount(1.0);
    amount += Amount(2.0);

    assert_eq!(amount, Amount(3.0));
}

/// Operator traits written as paths rather than imported names.
mod qualified_traits {
    use these_macros_should_be_illegal::overload_op;

    /// Operator traits reached through `crate::`, beside their assignment counterparts.
    pub mod operations {
        /// Interleaving of two values.
        pub trait Weave<Rhs> {
            /// Result of interleaving both operands.
            type Output;
            /// Interleaves this value with the right operand.
            fn weave(self, rhs: Rhs) -> Self::Output;
        }

        /// Augmented assignment of [`Weave`].
        pub trait WeaveAssign<Rhs> {
            /// Interleaves the right operand into this value.
            fn weave_assign(&mut self, rhs: Rhs);
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct Meters(pub f64);

    #[overload_op]
    impl std::ops::Add<&Meters> for &Meters {
        type Output = Meters;

        fn add(self, rhs: &Meters) -> Meters {
            Meters(self.0 + rhs.0)
        }
    }

    #[overload_op]
    impl ::core::ops::Mul<&f64> for &Meters {
        type Output = Meters;

        fn mul(self, rhs: &f64) -> Meters {
            Meters(self.0 * rhs)
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct Thread(pub String);

    #[overload_op]
    impl crate::qualified_traits::operations::Weave<&Thread> for &Thread {
        type Output = Thread;

        fn weave(self, rhs: &Thread) -> Thread {
            Thread(format!("{}{}", self.0, rhs.0))
        }
    }

    #[overload_op]
    impl self::operations::Weave<&f64> for &Thread {
        type Output = Thread;

        fn weave(self, rhs: &f64) -> Thread {
            Thread(format!("{}{rhs}", self.0))
        }
    }
}

/// Operator traits given a path keep their assignment trait beside them.
#[test]
fn overloads_an_operator_trait_written_as_a_path() {
    use qualified_traits::operations::{Weave, WeaveAssign};
    use qualified_traits::{Meters, Thread};

    let mut length = Meters(1.0);
    length += Meters(2.0);
    length *= 2.0;
    assert_eq!(length, Meters(6.0));

    let mut thread = Thread(String::from("a"));
    thread.weave_assign(Thread(String::from("b")));
    thread.weave_assign(1.0);
    assert_eq!(
        thread.weave(&Thread(String::from("c"))),
        Thread(String::from("ab1c"))
    );
}

/// User-defined operator traits that share the names `core::ops` exports.
mod shadowing_traits {
    use these_macros_should_be_illegal::overload_op;

    /// A user-defined `Add` and `AddAssign`, unrelated to the standard ones.
    pub mod custom {
        /// Merging of two values.
        pub trait Add<Rhs> {
            /// Result of merging both operands.
            type Output;
            /// Merges this value with the right operand.
            fn add(self, rhs: Rhs) -> Self::Output;
        }

        /// Augmented assignment of [`Add`].
        pub trait AddAssign<Rhs> {
            /// Merges the right operand into this value.
            fn add_assign(&mut self, rhs: Rhs);
        }
    }

    use custom::{Add, AddAssign};

    #[derive(Clone, Debug, PartialEq)]
    pub struct Tally(pub u32);

    #[overload_op]
    impl self::Add<&Tally> for &Tally {
        type Output = Tally;

        fn add(self, rhs: &Tally) -> Tally {
            Tally(self.0 + rhs.0)
        }
    }
}

/// A path to the operator trait selects the caller's `AddAssign` over the one in `core::ops`.
#[test]
fn selects_the_callers_assignment_trait_through_a_path() {
    use shadowing_traits::Tally;
    use shadowing_traits::custom::AddAssign;

    let mut tally = Tally(1);
    AddAssign::add_assign(&mut tally, Tally(2));
    assert_eq!(tally, Tally(3));
}

/// Two distinct types whose spellings differ only by a leading qualifier.
mod distinct_spellings {
    use std::ops::Add;
    use these_macros_should_be_illegal::overload_op;

    /// A type sharing its name with the outer one.
    pub mod inner {
        #[derive(Clone, Debug, PartialEq)]
        pub struct Level(pub u8);
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct Level(pub u8);

    #[overload_op]
    impl Add<&inner::Level> for &inner::Level {
        type Output = Level;

        fn add(self, rhs: &inner::Level) -> Level {
            Level(self.0 + rhs.0)
        }
    }
}

/// An operator into a distinct type still compiles, with only the ownership overloads.
#[test]
fn compiles_when_qualified_spellings_name_distinct_types() {
    use distinct_spellings::{Level, inner};

    assert_eq!(inner::Level(1) + inner::Level(2), Level(3));
}
