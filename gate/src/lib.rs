//! Empty on purpose. See `Cargo.toml`.

#[cfg(test)]
mod tests {
    /// This crate has no code, and that is its job: it exists so a `[patch]`
    /// table has something to patch, and so one lock file covers the stack.
    ///
    /// So the one thing worth asserting is that it stays that way. If anything
    /// grows above the test module, this crate has stopped being a resolution
    /// fixture and started being a thing with behaviour — which needs tests of
    /// its own rather than this one.
    ///
    /// Everything from `#[cfg(test)]` down is this test, and counting it would
    /// be counting itself.
    #[test]
    fn this_crate_has_no_code_of_its_own() {
        let source = include_str!("lib.rs");
        let above_the_tests = source
            .split("#[cfg(test)]")
            .next()
            .expect("split always yields one");

        let logic = above_the_tests
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .count();

        assert_eq!(
            logic, 0,
            "something was added to a crate whose whole value is having nothing in it"
        );
    }
}
