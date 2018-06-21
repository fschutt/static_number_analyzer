fn main() {
    let a = 5;
    let b = a;
    let c = a;
    if b == c {
        println!("{}", something());
        println!("{}", something_other(5, 6, 7));
    }
}

fn something() -> i32 {
    let mut a = 5;
    let b = 10;
    if a < b {
        return 5;
    }
    0
}

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