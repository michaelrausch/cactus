/// `n` as a `u32`, or `u32::MAX` when it doesn't fit: for counts and
/// durations sent to the UI, where a clamped value beats an error.
pub fn saturating_u32<T: TryInto<u32>>(n: T) -> u32 {
    n.try_into().unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::saturating_u32;

    #[test]
    fn clamps_what_does_not_fit() {
        assert_eq!(saturating_u32(7_usize), 7);
        assert_eq!(saturating_u32(u128::from(u32::MAX) + 1), u32::MAX);
    }
}
