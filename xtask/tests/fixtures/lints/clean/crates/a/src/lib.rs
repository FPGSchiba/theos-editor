//! A crate whose attributes all say why.

#[expect(dead_code, reason = "the example keeps it for a later task")]
fn unused() {}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "needs the network"]
    fn slow() {}
}
