pub mod artifact;
pub mod procs;

pub fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn adds() {
        assert_eq!(1, add(0, 1));
    }
}
