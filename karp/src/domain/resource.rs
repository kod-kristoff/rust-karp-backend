#[cfg(test)]
mod aggregate_tests {
    use super::*;

    #[test]
    fn resource_has_aggregate_type() {
        let resource = Resource::new();

        assert_eq!(resource.aggregate_type(), "resource");
    }
}
