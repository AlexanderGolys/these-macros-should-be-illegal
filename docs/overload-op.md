# Ownership overloads of one operator

API reference on docs.rs: [`overload_op`](https://docs.rs/these-macros-should-be-illegal/latest/these_macros_should_be_illegal/attr.overload_op.html).

Rust resolves `a + b`, `a + &b`, and `a += b` through separate traits and
separate impls, so one operator written once is not usable at the call sites
that ordinary code actually contains. `overload_op` goes on the impl written for
references and repeats it across the remaining forms.

It takes no arguments. The trait, both operand types, the result type and the
method name are all already written in the impl it is attached to, which is
emitted unchanged.

```rust
use std::ops::Add;
#[derive(Debug, PartialEq)]
struct Vector(f64, f64);

#[overload_op]
impl Add<&Vector> for &Vector {
    type Output = Vector;

    fn add(self, rhs: &Vector) -> Vector {
        Vector(self.0 + rhs.0, self.1 + rhs.1)
    }
}

assert_eq!(&Vector(1.0, 2.0) + &Vector(3.0, 4.0), Vector(4.0, 6.0));
assert_eq!(Vector(1.0, 2.0) + &Vector(3.0, 4.0), Vector(4.0, 6.0));
assert_eq!(Vector(1.0, 2.0) + Vector(3.0, 4.0), Vector(4.0, 6.0));

let mut total = Vector(1.0, 2.0);
total += &Vector(3.0, 4.0);
total += Vector(1.0, 1.0);
assert_eq!(total, Vector(5.0, 7.0));
```

## The reference impl is the one to write

Both operands must be shared references, and every generated impl delegates by
borrowing.
Nothing is cloned and no `Clone` bound is introduced. Writing the owned impl by
hand instead would force the borrowed forms to clone, so that direction is
rejected rather than supported:

```rust,compile_fail
use std::ops::Add;
struct Vector(f64);

// error: the operator must be implemented for a reference, as `impl Add<&T> for &T`
#[overload_op]
impl Add<&Vector> for Vector {
    type Output = Vector;

    fn add(self, rhs: &Vector) -> Vector {
        Vector(self.0 + rhs.0)
    }
}
```

An owned right operand is rejected the same way, and so is a `&mut` operand on
either side, since the generated impls only ever lend their operands with `&`.
Neither is assumed; both are checked, so a mistake is reported against the impl
rather than showing up as a strange error inside generated code.

## What is generated

From a binary operator, four impls follow:

| Generated | Left operand | Right operand |
| --- | --- | --- |
| `Trait<&Rhs> for Lhs` | owned | borrowed |
| `Trait<Rhs> for Lhs` | owned | owned |
| `TraitAssign<&Rhs> for Lhs` | mutated | borrowed |
| `TraitAssign<Rhs> for Lhs` | mutated | owned |

Spelled out for an addition, the generated impls are the following. Run it, or
reveal the hidden lines to see the hand-written impl they all delegate to:

```rust,mdbook-runnable
# use std::ops::{Add, AddAssign};
#
# #[derive(Debug, PartialEq)]
# struct Vector(f64);
#
# impl Add<&Vector> for &Vector {
#     type Output = Vector;
#
#     fn add(self, rhs: &Vector) -> Vector {
#         Vector(self.0 + rhs.0)
#     }
# }
#
impl Add<&Vector> for Vector {
    type Output = Vector;

    fn add(self, rhs: &Vector) -> Vector {
        &self + rhs
    }
}

impl Add<Vector> for Vector {
    type Output = Vector;

    fn add(self, rhs: Vector) -> Vector {
        &self + &rhs
    }
}

impl AddAssign<&Vector> for Vector {
    fn add_assign(&mut self, rhs: &Vector) {
        *self = &*self + rhs;
    }
}

impl AddAssign<Vector> for Vector {
    fn add_assign(&mut self, rhs: Vector) {
        *self = &*self + &rhs;
    }
}

let mut total = Vector(1.0) + Vector(2.0);
total += &Vector(3.0);
println!("{total:?}");
```

`Trait<Rhs> for &Lhs` is deliberately absent: `&a + b` mixes a borrowed left
operand with an owned right one, which is not a spelling ordinary code uses.

A unary operator generates the single owned form:

```rust
use std::ops::Neg;
#[derive(Debug, PartialEq)]
struct Vector(f64, f64);

#[overload_op]
impl Neg for &Vector {
    type Output = Vector;

    fn neg(self) -> Vector {
        Vector(-self.0, -self.1)
    }
}

assert_eq!(-&Vector(1.0, 2.0), Vector(-1.0, -2.0));
assert_eq!(-Vector(1.0, 2.0), Vector(-1.0, -2.0));
```

Unary and binary are told apart by the method's own arity, so a trait naming no
right operand still has one when its method takes an argument — `Rhs` then
defaults to `Self`, which is the borrowed left operand:

```rust
use std::ops::Add;
#[derive(Debug, PartialEq)]
struct Count(u32);

#[overload_op]
impl Add for &Count {
    type Output = Count;

    fn add(self, rhs: &Count) -> Count {
        Count(self.0 + rhs.0)
    }
}

assert_eq!(Count(1) + Count(2), Count(3));
```

## Augmented assignment

`a += b` is generated as exactly `a = &a + b`. That is the standard case, and it
is the only one the macro claims: a type whose in-place update is genuinely
cheaper than rebuilding the value should write all four impls by hand rather
than reach for the macro.

The assignment overloads are skipped when the result type differs from the left
operand, because `a = a op b` is then not even well typed. An operator such as
`Point - Point -> Vector` therefore generates only the two ownership overloads:

```rust
use std::ops::Sub;
#[derive(Debug, PartialEq)]
struct Vector(f64);

#[derive(Debug, PartialEq)]
struct Point(f64);

#[overload_op]
impl Sub<&Point> for &Point {
    type Output = Vector;

    fn sub(self, rhs: &Point) -> Vector {
        Vector(self.0 - rhs.0)
    }
}

assert_eq!(Point(5.0) - Point(2.0), Vector(3.0));
```

The comparison is made on the types as written, before any name is resolved, and
a difference in leading qualifiers alone does not count: `crate::Vector` in one
position and `Vector` in the other still generate the assignment. In case the two
spellings turn out to name distinct types, each assignment impl is bounded by the
operator itself returning the left operand type,

```rust,ignore
impl AddAssign<&Vector> for Vector
where
    for<'operand> &'operand Vector: Add<&'operand Vector, Output = Vector>,
```

so for distinct types the bound fails and the impl simply never applies, leaving
the two ownership overloads, rather than failing to compile.

## Any operator trait

Nothing here is specific to `std::ops`. The impl supplies the method name, so
there is no table of operator names anywhere; only the augmented assignment is
named by convention, suffixing the trait with `Assign` and the method with
`_assign`:

```rust
trait Join<Rhs> {
    type Output;
    fn join(self, rhs: Rhs) -> Self::Output;
}

trait JoinAssign<Rhs> {
    fn join_assign(&mut self, rhs: Rhs);
}

#[derive(Debug, PartialEq)]
struct Path(String);

#[overload_op]
impl Join<&Path> for &Path {
    type Output = Path;

    fn join(self, rhs: &Path) -> Path {
        Path(format!("{}/{}", self.0, rhs.0))
    }
}

fn main() {
    let mut path = Path(String::from("usr"));
    path.join_assign(Path(String::from("local")));
    assert_eq!(path, Path(String::from("usr/local")));
}
```

Where the assignment trait is looked up depends on how the operator trait is
written:

- **A path**, such as `std::ops::Add` or `crate::ops::Join`: the assignment trait
  is the same path with the last segment suffixed, `std::ops::AddAssign` or
  `crate::ops::JoinAssign`.
- **A bare name**, such as `Add` or `Join`: the suffixed name is looked up both
  in `core::ops` and among the names of the module surrounding the impl, so
  `use std::ops::Add;` alone is enough for the standard `+=`, and a custom
  `JoinAssign` is found beside a custom `Join`.

The two lookups of a bare name are equal, so a custom trait sharing a name with
`core::ops`, such as a custom `AddAssign` imported beside a custom `Add`, makes
that name ambiguous. Write the operator trait as a path, even just `self::Add`,
to take the assignment trait from beside it instead.

The assignment trait is never found inside a function body, where items have no
path and no module sees them, so declare custom operator and assignment traits at
module scope, as above. Operand types are unaffected: the generated impls stay
where the attributed impl is, so types declared in a function body work.

## Generics and lifetimes

Generic parameters, bounds, and where predicates of the attributed impl are
repeated on every generated impl. Lifetimes are the exception: one that named
only an operand's outer reference has nothing left to name once that reference
is removed, so it is dropped from the generated header, and the generated bodies
borrow with elided lifetimes instead.

A bound mentioning that lifetime is kept rather than dropped with it. The
generated impls still need the bound, only for the fresh borrow their bodies
create, so it becomes higher-ranked over the lifetime that went away. Writing

```rust,ignore
impl<'a, T> Add<&'a Pair<T>> for &'a Pair<T>
where
    &'a T: Add<&'a T, Output = T>,
```

therefore generates the owned impl under `for<'a> &'a T: Add<&'a T, Output = T>`,
which is what its body requires. The explicit `for<'b>` spelling below states the
same thing directly. An outlives bound such as `T: 'a` is the one exception: it
constrained the lifetime being removed, and `for<'a> T: 'a` would demand
`'static` rather than what was written, so it is dropped with the lifetime.

```rust
use std::ops::Add;
#[derive(Debug, PartialEq)]
struct Pair<T>(T, T);

#[overload_op]
impl<'a, T> Add<&'a Pair<T>> for &'a Pair<T>
where
    for<'b> &'b T: Add<&'b T, Output = T>,
{
    type Output = Pair<T>;

    fn add(self, rhs: &'a Pair<T>) -> Pair<T> {
        Pair(&self.0 + &rhs.0, &self.1 + &rhs.1)
    }
}

assert_eq!(Pair(1u32, 2u32) + Pair(3u32, 4u32), Pair(4u32, 6u32));
```
