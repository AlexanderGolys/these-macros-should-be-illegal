//! Consumer tests for operators completed from a type's existing ones.

use std::{
    marker::PhantomData,
    ops::{Add, Mul, Neg, Sub},
};
use these_macros_should_be_illegal::{complete_ops, overload_op};

/// A group acting on itself: subtraction follows from its own addition.
#[derive(Clone, Debug, PartialEq)]
#[complete_ops(Sub)]
struct Vector(f64);

#[overload_op]
impl Add<&Vector> for &Vector {
    type Output = Vector;

    fn add(self, rhs: &Vector) -> Vector {
        Vector(self.0 + rhs.0)
    }
}

#[overload_op]
impl Neg for &Vector {
    type Output = Vector;

    fn neg(self) -> Vector {
        Vector(-self.0)
    }
}

/// A space acted on by a group: the completion never mentions the space's own structure.
#[derive(Clone, Debug, PartialEq)]
#[complete_ops(Sub)]
struct Point(f64);

#[overload_op]
impl Add<&Vector> for &Point {
    type Output = Point;

    fn add(self, rhs: &Vector) -> Point {
        Point(self.0 + rhs.0)
    }
}

/// A difference of two points needs structure the action alone does not give, so it
/// is never completed; where it does exist, it stays hand-written and coexists.
impl Sub<&Point> for &Point {
    type Output = Vector;

    fn sub(self, rhs: &Point) -> Vector {
        Vector(self.0 - rhs.0)
    }
}

/// Negation completed from a scalar multiplication, and subtraction through it.
#[derive(Clone, Debug, PartialEq)]
#[complete_ops(Neg = -1.0f64, Sub)]
struct Scaled(f64);

#[overload_op]
impl Add<&Scaled> for &Scaled {
    type Output = Scaled;

    fn add(self, rhs: &Scaled) -> Scaled {
        Scaled(self.0 + rhs.0)
    }
}

#[overload_op]
impl Mul<&f64> for &Scaled {
    type Output = Scaled;

    fn mul(self, rhs: &f64) -> Scaled {
        Scaled(self.0 * rhs)
    }
}

/// A generic declaration keeps its own parameters alongside the completed operand.
#[derive(Clone, Debug, PartialEq)]
#[complete_ops(Sub)]
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

#[overload_op]
impl<T> Neg for &Pair<T>
where
    for<'b> &'b T: Neg<Output = T>,
{
    type Output = Pair<T>;

    fn neg(self) -> Pair<T> {
        Pair(-&self.0, -&self.1)
    }
}

#[test]
fn completes_subtraction_of_a_group_from_its_addition() {
    assert_eq!(Vector(5.0) - Vector(2.0), Vector(3.0));
}

#[test]
fn completes_the_augmented_assignment_alongside_it() {
    let mut total = Vector(5.0);

    total -= Vector(2.0);
    assert_eq!(total, Vector(3.0));

    total -= Vector(1.0);
    assert_eq!(total, Vector(2.0));
}

#[test]
fn completes_subtraction_of_a_group_acting_on_a_space() {
    assert_eq!(Point(5.0) - Vector(2.0), Point(3.0));

    let mut position = Point(5.0);
    position -= Vector(2.0);
    assert_eq!(position, Point(3.0));
}

#[test]
fn coexists_with_a_hand_written_difference_of_two_points() {
    assert_eq!(&Point(5.0) - &Point(2.0), Vector(3.0));
    assert_eq!(Point(5.0) - Vector(2.0), Point(3.0));
}

#[test]
fn completes_negation_from_a_scalar_multiplication() {
    assert_eq!(-Scaled(2.0), Scaled(-2.0));
    assert_eq!(-&Scaled(2.0), Scaled(-2.0));
    assert_eq!(Scaled(5.0) - Scaled(2.0), Scaled(3.0));
}

#[test]
fn completes_a_generic_declaration() {
    assert_eq!(Pair(5i32, 4i32) - Pair(1i32, 2i32), Pair(4i32, 2i32));

    let mut total = Pair(5i32, 4i32);
    total -= Pair(1i32, 2i32);
    assert_eq!(total, Pair(4i32, 2i32));
}

/// A declaration already using the lifetime name the completion introduces.
#[derive(Clone, Debug, PartialEq)]
#[complete_ops(Sub)]
struct Tagged<'operand>(f64, PhantomData<&'operand ()>);

impl Tagged<'_> {
    /// Builds a tagged value, since the marker carries the lifetime.
    fn new(value: f64) -> Self {
        Self(value, PhantomData)
    }
}

#[overload_op]
impl<'operand> Add<&Tagged<'operand>> for &Tagged<'operand> {
    type Output = Tagged<'operand>;

    fn add(self, rhs: &Tagged<'operand>) -> Tagged<'operand> {
        Tagged(self.0 + rhs.0, PhantomData)
    }
}

#[overload_op]
impl<'operand> Neg for &Tagged<'operand> {
    type Output = Tagged<'operand>;

    fn neg(self) -> Tagged<'operand> {
        Tagged(-self.0, PhantomData)
    }
}

/// The introduced operand lifetime avoids one the declaration already names.
#[test]
fn completes_a_declaration_naming_the_operand_lifetime() {
    assert_eq!(Tagged::new(5.0) - Tagged::new(2.0), Tagged::new(3.0));
}

/// A declaration whose const parameter takes the name the right operand would get.
#[allow(non_upper_case_globals)]
mod const_parameter {
    use std::ops::{Add, Neg};
    use these_macros_should_be_illegal::complete_ops;

    #[derive(Clone, Debug, PartialEq)]
    #[complete_ops(Sub)]
    pub struct Samples<const Rhs: usize>(pub [i32; Rhs]);

    impl<const Rhs: usize> Add<Samples<Rhs>> for Samples<Rhs> {
        type Output = Samples<Rhs>;

        fn add(self, rhs: Samples<Rhs>) -> Samples<Rhs> {
            Samples(std::array::from_fn(|index| self.0[index] + rhs.0[index]))
        }
    }

    impl<const Rhs: usize> Neg for Samples<Rhs> {
        type Output = Samples<Rhs>;

        fn neg(self) -> Samples<Rhs> {
            Samples(self.0.map(|sample| -sample))
        }
    }
}

/// A declaration whose own name is the one the right operand would get.
mod own_name {
    use std::ops::{Add, Neg};
    use these_macros_should_be_illegal::{complete_ops, overload_op};

    #[derive(Clone, Debug, PartialEq)]
    #[complete_ops(Sub)]
    pub struct Rhs(pub i32);

    #[overload_op]
    impl Add<&Rhs> for &Rhs {
        type Output = Rhs;

        fn add(self, rhs: &Rhs) -> Rhs {
            Rhs(self.0 + rhs.0)
        }
    }

    #[overload_op]
    impl Neg for &Rhs {
        type Output = Rhs;

        fn neg(self) -> Rhs {
            Rhs(-self.0)
        }
    }
}

/// The introduced right operand is named around every name the declaration writes.
#[test]
fn names_the_right_operand_around_written_names() {
    use const_parameter::Samples;
    use own_name::Rhs;

    assert_eq!(Samples([5, 7]) - Samples([1, 2]), Samples([4, 5]));
    assert_eq!(Rhs(5) - Rhs(2), Rhs(3));
}
