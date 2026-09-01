# Function-like syntax for callable objects

`callable` gives one method of a user-owned trait the structural alias
`__priv_tmsbi_call`. `make_fn!` can then bind any implementation and generate a
same-name macro which forwards its arguments to that alias. No shared callable
trait is imposed by this crate.

```rust
use these_macros_should_be_illegal::{callable, make_fn};

#[callable(apply)]
trait Action {
    fn apply(&self, point: usize) -> usize;
}

#[derive(Clone)]
struct Shift(usize);

impl Action for Shift {
    fn apply(&self, point: usize) -> usize {
        point + self.0
    }
}

impl Shift {
    fn inverse(&self) -> Shift {
        Shift(usize::MAX - self.0 + 1)
    }
}

make_fn!(sigma = Shift(3));

assert_eq!(sigma!(2), 5);
let _inverse = sigma.inverse();
```

The macro and value occupy different namespaces. Consequently `sigma!(2)` can
expand to `sigma.__priv_tmsbi_call(2)` while ordinary methods remain available
on `sigma`.

The generated alias is a default trait method. Existing implementations only
implement the method selected by the attribute. Its receiver, arguments,
generics, return type, bounds, and async or unsafe behavior are retained.

Mutable callable methods require a mutable binding:

```rust
use these_macros_should_be_illegal::{callable, make_fn};

#[callable(advance)]
trait Advance {
    fn advance(&mut self, amount: usize) -> usize;
}

struct Counter(usize);

impl Advance for Counter {
    fn advance(&mut self, amount: usize) -> usize {
        self.0 += amount;
        self.0
    }
}

make_fn!(mut counter = Counter(0));
assert_eq!(counter!(2), 2);
assert_eq!(counter!(3), 5);
```

`make_fn!` creates a `let` binding and is therefore intended for statement
position inside a function or block. Method resolution must find exactly one
accessible `__priv_tmsbi_call`; implementing multiple registered callable
traits for one type deliberately makes the sugar ambiguous.
