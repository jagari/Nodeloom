//! Splitting a frame range into contiguous chunks.

/// Splits the inclusive range `start..=end` into at most `parts` contiguous,
/// non-overlapping inclusive ranges whose sizes differ by at most one frame.
///
/// Returns fewer than `parts` ranges when there are fewer frames than parts,
/// and an empty vector when `parts == 0` or `start > end`.
pub fn split_frames(start: u32, end: u32, parts: usize) -> Vec<(u32, u32)> {
    if parts == 0 || start > end {
        return Vec::new();
    }
    let total = u64::from(end) - u64::from(start) + 1;
    let parts = (parts as u64).min(total);
    let base = total / parts;
    let extra = total % parts;

    let mut ranges = Vec::with_capacity(parts as usize);
    let mut next = u64::from(start);
    for i in 0..parts {
        let len = base + u64::from(i < extra);
        ranges.push((next as u32, (next + len - 1) as u32));
        next += len;
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn even_split() {
        assert_eq!(split_frames(1, 10, 2), vec![(1, 5), (6, 10)]);
    }

    #[test]
    fn uneven_split_gives_earlier_chunks_the_extra_frame() {
        assert_eq!(split_frames(1, 10, 3), vec![(1, 4), (5, 7), (8, 10)]);
    }

    #[test]
    fn more_parts_than_frames() {
        assert_eq!(split_frames(4, 5, 8), vec![(4, 4), (5, 5)]);
    }

    #[test]
    fn degenerate_inputs() {
        assert!(split_frames(1, 10, 0).is_empty());
        assert!(split_frames(10, 1, 3).is_empty());
        assert_eq!(split_frames(7, 7, 3), vec![(7, 7)]);
    }

    #[test]
    fn covers_range_exactly_at_u32_edge() {
        let r = split_frames(u32::MAX - 4, u32::MAX, 2);
        assert_eq!(
            r,
            vec![(u32::MAX - 4, u32::MAX - 2), (u32::MAX - 1, u32::MAX)]
        );
    }

    #[test]
    fn chunks_are_contiguous_and_complete() {
        for parts in 1..20 {
            let r = split_frames(3, 97, parts);
            assert_eq!(r.first().unwrap().0, 3);
            assert_eq!(r.last().unwrap().1, 97);
            for w in r.windows(2) {
                assert_eq!(w[0].1 + 1, w[1].0);
            }
        }
    }
}
