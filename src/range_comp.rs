use std::ops::Range;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum RangeComparisonResult {
    AlwaysLarger,
    AlwaysEqual,
    AlwaysSmaller,
    RangeOverlaps,
}

pub(crate) trait RangeExt<T: PartialOrd> {
    /// Is `self` larger than other
    /// (5..10).compare((0..5)) = RangeComparisonResult::AlwaysSmaller
    fn compare(&self, other: &Range<T>) -> RangeComparisonResult;
}

impl<Idx: PartialOrd + Copy> RangeExt<Idx> for Range<Idx> {
    fn compare(&self, other: &Range<Idx>) -> RangeComparisonResult {

        if self.start == other.start && self.end == other.end {
            return RangeComparisonResult::AlwaysEqual;
        }

        let max_a = if self.start > self.end { self.start } else { self.end };
        let min_a = if self.start < self.end { self.start } else { self.end };
        let max_b = if other.start > other.end { other.start } else { other.end };
        let min_b = if other.start < other.end { other.start } else { other.end };

        if min_a <= min_b && max_a <= min_b {
            // [min_a, max_a] [min_b, max_b]
            RangeComparisonResult::AlwaysSmaller
        } else if  min_b <= min_a && max_b <= min_a {
            // [min_b, max_b] [min_a, max_a]
            RangeComparisonResult::AlwaysLarger
        } else {
            RangeComparisonResult::RangeOverlaps
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_range_comparison() {
        assert_eq!((0..5).compare(&(7..10)), RangeComparisonResult::AlwaysSmaller);
        assert_eq!((0..8).compare(&(7..10)), RangeComparisonResult::RangeOverlaps);
        assert_eq!((0..60).compare(&(7..10)), RangeComparisonResult::RangeOverlaps);
        assert_eq!((41..60).compare(&(1..3)), RangeComparisonResult::AlwaysLarger);
        assert_eq!((5..5).compare(&(5..5)), RangeComparisonResult::AlwaysEqual);
    }
}
