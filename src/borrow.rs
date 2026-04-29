use core::cell::Cell;

/// A bit mask used to signal the borrow state has an active mutable borrow.
const UNIQUE_BIT: usize = !(usize::MAX >> 1);

const COUNTER_MASK: usize = usize::MAX >> 1;

/// A single-threaded counter used to dynamically enforce borrowing rules
///
/// The most significant bit is used to track mutable borrow, and the rest is a
/// counter for immutable borrows.
///
/// It has four possible states:
///  - `0b00000000...` the counter isn't mut borrowed, and ready for borrowing
///  - `0b0_______...` the counter isn't mut borrowed, and currently borrowed
///  - `0b10000000...` the counter is mut borrowed
///  - `0b1_______...` the counter is mut borrowed, and another shared borrow was attempted
pub struct BorrowState(Cell<usize>);

impl BorrowState {
    pub const fn new() -> Self {
        Self(Cell::new(0))
    }

    pub fn borrow(&self) -> bool {
        let value = self.0.get();

        // If the counter has all of the immutable borrow bits set,
        // the immutable borrow counter overflowed.
        if value & COUNTER_MASK == COUNTER_MASK {
            core::panic!("immutable borrow counter overflowed")
        }

        // If the mutable borrow bit is set, immutable borrow can't occur.
        if value & UNIQUE_BIT != 0 {
            false
        } else {
            self.0.set(value + 1);
            true
        }
    }

    pub fn borrow_mut(&self) -> bool {
        if self.0.get() == 0 {
            self.0.set(UNIQUE_BIT);
            true
        } else {
            false
        }
    }

    pub fn release(&self) {
        let value = self.0.get();
        debug_assert!(value != 0, "unbalanced release");
        debug_assert!(value & UNIQUE_BIT == 0, "shared release of unique borrow");
        self.0.set(value.wrapping_sub(1));
    }

    pub fn release_mut(&self) {
        let value = self.0.get();
        debug_assert_ne!(value & UNIQUE_BIT, 0, "unique release of shared borrow");
        self.0.set(value & !UNIQUE_BIT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "immutable borrow counter overflowed")]
    fn test_borrow_counter_overflow() {
        let counter = BorrowState(Cell::new(COUNTER_MASK));
        counter.borrow();
    }

    #[test]
    #[should_panic(expected = "immutable borrow counter overflowed")]
    fn test_mut_borrow_counter_overflow() {
        let counter = BorrowState(Cell::new(COUNTER_MASK | UNIQUE_BIT));
        counter.borrow();
    }

    #[test]
    fn test_borrow() {
        let counter = BorrowState::new();
        assert!(counter.borrow());
        assert!(counter.borrow());
        assert!(!counter.borrow_mut());
        counter.release();
        counter.release();

        assert!(counter.borrow_mut());
        assert!(!counter.borrow());
        counter.release_mut();
        assert!(counter.borrow());
    }
}
