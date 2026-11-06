use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const DUMMY: Span = Span { start: 0, end: 0 };

    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn is_dummy(self) -> bool {
        self.start == 0 && self.end == 0
    }

    pub fn join(self, other: Span) -> Span {
        if self.is_dummy() {
            return other;
        }
        if other.is_dummy() {
            return self;
        }
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub fn to_range(self) -> Range<usize> {
        self.start..self.end
    }
}

impl From<Range<usize>> for Span {
    fn from(range: Range<usize>) -> Self {
        Span {
            start: range.start,
            end: range.end,
        }
    }
}

impl From<Span> for Range<usize> {
    fn from(span: Span) -> Self {
        span.to_range()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_covers_both_operands() {
        let left = Span::new(4, 9);
        let right = Span::new(12, 20);
        assert_eq!(left.join(right), Span::new(4, 20));
        assert_eq!(right.join(left), Span::new(4, 20));
    }

    #[test]
    fn join_ignores_a_dummy_operand() {
        let real = Span::new(10, 14);
        assert_eq!(real.join(Span::DUMMY), real);
        assert_eq!(Span::DUMMY.join(real), real);
        assert_eq!(Span::DUMMY.join(Span::DUMMY), Span::DUMMY);
    }

    #[test]
    fn a_span_that_points_nowhere_is_dummy() {
        assert!(Span::DUMMY.is_dummy());
        assert!(Span::new(0, 0).is_dummy());
        assert!(!Span::new(0, 1).is_dummy());
    }

    #[test]
    fn a_span_round_trips_through_a_range() {
        let span = Span::new(3, 7);
        assert_eq!(Span::from(span.to_range()), span);
        assert_eq!(Range::from(span), 3..7);
    }
}
