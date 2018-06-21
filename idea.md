This is something that neither the rust compiler or clippy can currently catch:

```rust
fn main() {
    let a = 5;
    let b = a;
    let c = a;
    if b == c { // useless if - always true
        println!("hello");
    }
}
```

or:

```rust
fn something() -> i32 {
    let a = 5;
    let b = 10;
    if a < b { // always true
        return 5;
    }
    0
}
```

or:

```rust
fn something_other(a: usize, b: usize, c: usize) -> i32 {
    if a < b {
      if b < c {
        if c < a { // always false, never executed
            5
        } else {
            6
        }
      } else {
        7
      }
    } else {
        8
    }
}
```

I got these ideas while reading this post: https://medium.com/@Coder_HarryLee/development-of-a-new-static-analyzer-pvs-studio-java-f92f7c139362

Now I realize that this is a very complex topic, but it's just an idea for improvement - analyzing constraints and it would probably break on more complex cases. **I propose to do this just for numbers at first and just for immutable variables.** Not perfect, but at least a start.

How I'd imagine this (from a high-level view) to be implemented is using a system of ranges (to check for overlap between two possible cases), dependencies (which variables come from where) and constraints (

The first case could be implemented (at least for numbers) with numerical ranges:

```rust
fn something() -> i32 {
    // let constraints_to_check = [];
    let a = 5;            // a = range(min: 5, max: 5), dependency: none, constraint: none
    let b = 10;            // b = range (min: 10, max: 10), dependency: none, constraint: none
    if a < b {             // constraints_to_check.insert(a < b);
         // check for always true / always false:
         // a and b have no dependencies on other variables, so just take the values of each
         // question: is "range(5:5) < range(10:10)" always true or false:
         //     negate question: is it possible for this statement to not be always true or always false?
         //     - is it possible for the ranges to overlap: no -> remove = operator
         //     -
        return 5;
    }
    0
}
```