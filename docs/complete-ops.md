# Operators that follow from the ones a type has

API reference on docs.rs: [`complete_ops`](https://docs.rs/these-macros-should-be-illegal/latest/these_macros_should_be_illegal/attr.complete_ops.html).

Subtraction is not new information. If `x + v` makes sense, and `v` lies in an
abelian group, then `x - v` makes sense too: `-v` lies in that same group, so
`x + (-v)` is an addition the type already has. `complete_ops` writes that down,
and claims nothing else.

```rust
use std::ops::{Add, Neg};
#[derive(Debug, PartialEq)]
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

assert_eq!(Vector(5.0) - Vector(2.0), Vector(3.0));

let mut total = Vector(5.0);
total -= Vector(2.0);
assert_eq!(total, Vector(3.0));
```

## The right operand stays generic

The completion says nothing about which type is subtracted, only that it is a
group the attributed type can add:

```rust,ignore
impl<Rhs> Sub<Rhs> for Space
where
    Rhs: Neg<Output = Rhs>,
    Space: Add<Rhs>,
{
    type Output = <Space as Add<Rhs>>::Output;

    fn sub(self, rhs: Rhs) -> Self::Output {
        self + -rhs
    }
}
```

The completion is plain Rust, so it can be tried out directly. Edit the block
below and run it, for example subtracting `&Vector(2.0)` to see a borrowed
operand rejected:

```rust,editable,mdbook-runnable
use std::ops::{Add, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq)]
struct Vector(f64);

impl Add for Vector {
    type Output = Vector;

    fn add(self, rhs: Vector) -> Vector {
        Vector(self.0 + rhs.0)
    }
}

impl Neg for Vector {
    type Output = Vector;

    fn neg(self) -> Vector {
        Vector(-self.0)
    }
}

// The impl `#[complete_ops(Sub)]` adds.
impl<Rhs> Sub<Rhs> for Vector
where
    Rhs: Neg<Output = Rhs>,
    Vector: Add<Rhs>,
{
    type Output = <Vector as Add<Rhs>>::Output;

    fn sub(self, rhs: Rhs) -> Self::Output {
        self + -rhs
    }
}

fn main() {
    println!("{:?}", Vector(5.0) - Vector(2.0));
}
```

`Neg<Output = Rhs>` is what makes the operand a group element rather than merely
something negatable: an inverse stays in the group it came from. One impl
therefore covers every group the type can add, rather than one impl per operand
type, and the result type is whatever that addition already returns. `Rhs` and
the lifetime of the borrowed operand are only default names: when the declaration
already writes either, as a parameter, a bounded type, or its own name, the
introduced one is numbered instead, as `Rhs2`.
Nothing about the attributed type's own structure is assumed, so a space acted
on by a group completes exactly as a group acting on itself does:

```rust
use std::ops::{Add, Neg};
#[derive(Debug, PartialEq)]
struct Vector(f64);

#[derive(Debug, PartialEq)]
#[complete_ops(Sub)]
struct Point(f64);

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

#[overload_op]
impl Add<&Vector> for &Point {
    type Output = Point;

    fn add(self, rhs: &Vector) -> Point {
        Point(self.0 + rhs.0)
    }
}

assert_eq!(Point(5.0) - Vector(2.0), Point(3.0));
```

`Point` has no addition of its own and no negation. It is only a space the group
acts on, and translating it backwards is still `x + (-v)`.

## What it does not claim

The completion covers exactly the subtractions that are additions of an inverse,
and the difference of two points of a space is not one of them. It is not merely
a different operation: in general it does not exist. Take the circle acted on by
the reals, `t` rotating a point by `t`. The action is transitive, but recovering
a real number from two points of the circle needs a choice of branch of the
logarithm, and no such choice follows from the action. So `x - x'` is never
completed. Where a type does have one, it stays hand-written and coexists with
the completion:

```rust
use std::ops::{Add, Neg, Sub};
#[derive(Debug, PartialEq)]
struct Vector(f64);

#[derive(Debug, PartialEq)]
#[complete_ops(Sub)]
struct Point(f64);
# #[overload_op]
# impl Add<&Vector> for &Vector {
#     type Output = Vector;
#     fn add(self, rhs: &Vector) -> Vector { Vector(self.0 + rhs.0) }
# }
# #[overload_op]
# impl Neg for &Vector {
#     type Output = Vector;
#     fn neg(self) -> Vector { Vector(-self.0) }
# }
# #[overload_op]
# impl Add<&Vector> for &Point {
#     type Output = Point;
#     fn add(self, rhs: &Vector) -> Point { Point(self.0 + rhs.0) }
# }

impl Sub<&Point> for &Point {
    type Output = Vector;

    fn sub(self, rhs: &Point) -> Vector {
        Vector(self.0 - rhs.0)
    }
}

assert_eq!(&Point(5.0) - &Point(2.0), Vector(3.0));
assert_eq!(Point(5.0) - Vector(2.0), Point(3.0));
```

That coexistence rests on the space having no negation of its own: the two impls
are disjoint because `Point: Neg<Output = Point>` cannot hold. A type that *is*
its own group has a negation, so its completed `Sub` claims every right operand
and no other `Sub` can be written for it. That is usually what you want — there is no second
subtraction on a group — but it is a door that closes.

## Completing the negation too

A type that multiplies by a scalar does not need a hand-written negation either,
since `-x` is `x` scaled by `-1`. Naming the scalar completes `Neg`, which the
subtraction then goes through:

```rust
use std::ops::{Add, Mul};
#[derive(Debug, PartialEq)]
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

assert_eq!(-Scaled(2.0), Scaled(-2.0));
assert_eq!(Scaled(5.0) - Scaled(2.0), Scaled(3.0));
```

The scalar is written as an expression, so its type cannot be named in the
generated header and the completion states no bound for the multiplication it
performs. On a concrete declaration, as above, that costs nothing. On a generic
one the multiplication must already be available for every instantiation, which
means carrying the bounds on the declaration itself:

```rust,ignore
// `Pair<T>` negates only where every `Pair<T>` can be scaled by the scalar.
#[complete_ops(Neg = -1.0f64)]
struct Pair<T: Copy + Mul<f64, Output = T>>(T, T);
```

Without them the expansion fails inside the generated `neg`, reported against
the `#[complete_ops(...)]` attribute rather than against the declaration.

Suffix the literal, as `-1.0f64` above, whenever the type has more than one
scalar multiplication: the value is passed to `Mul::mul` as written, so its type
is what selects the impl. The order inside the attribute does not matter, since a
completed negation is always emitted ahead of the subtraction that uses it. Each
operator may be named once; a repeated one is rejected rather than producing
conflicting impls.

## Borrowed operands

A borrowed *right* operand is not a group element: negating `&Vector` yields an
owned `Vector`, so `&Vector: Neg<Output = &Vector>` does not hold and the
completion does not apply. `x - &v` is therefore rejected:

```text
error[E0271]: type mismatch resolving `<&Vector as Neg>::Output == &Vector`
```

Subtract the group element itself, as `x - v`.

The borrowed *left* operand fails for an unrelated reason. The completion does
emit `impl Sub<Rhs> for &Lhs`, but reaching it needs `Add<Rhs> for &Lhs` — the
ref-plus-owned combination [`overload_op`](overload-op.md) deliberately does not
generate — so that impl stays inert unless the addition is written by hand.
