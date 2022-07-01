use std::{
    mem,
    ops::Deref,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::RefOverflow;

/// Maximum number of references that can be held so that it is safe to add
/// another.
const REF_LIMIT_MAX: usize = usize::MAX - 1;

/// An immutable reference to data in a `Cell`.
///
/// Access the value via `std::ops::Deref` (e.g. `*val`)
#[derive(Debug)]
pub struct CellRef<'a, T>
where
    T: ?Sized + 'a,
{
    pub flag: &'a AtomicUsize,
    pub value: &'a T,
}

impl<'a, T> CellRef<'a, T>
where
    T: ?Sized,
{
    /// Returns a clone of this `CellRef`.
    ///
    /// This method allows handling of reference overflows, but:
    ///
    /// * Having 4 billion / 9 quintillion references to an object is not a
    ///   realistic scenario in most applications.
    /// * Applications that hold `CellRef`s with an ever-increasing reference
    ///   count is not supported by this library.
    ///
    ///     Reaching `usize::MAX` may be possible with
    ///     `std::mem::forget(CellRef::clone(&r))`.
    pub fn try_clone(&self) -> Result<Self, RefOverflow> {
        self.flag
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current_value| {
                if current_value <= REF_LIMIT_MAX {
                    Some(current_value + 1)
                } else {
                    None
                }
            })
            .map(|_| CellRef {
                flag: self.flag,
                value: self.value,
            })
            .map_err(|_| RefOverflow)
    }

    /// Makes a new `CellRef` for a component of the borrowed data which
    /// preserves the existing borrow.
    ///
    /// The `Cell` is already immutably borrowed, so this cannot fail.
    ///
    /// This is an associated function that needs to be used as
    /// `CellRef::map(...)`. A method would interfere with methods of the
    /// same name on the contents of a `CellRef` used through `Deref`.
    /// Further this preserves the borrow of the value and hence does the
    /// proper cleanup when it's dropped.
    ///
    /// # Examples
    ///
    /// This can be used to avoid pointer indirection when a boxed item is
    /// stored in the `Cell`.
    ///
    /// ```rust
    /// use rt_ref::{Cell, CellRef};
    ///
    /// let cb = Cell::new(Box::new(5));
    ///
    /// // Borrowing the cell causes the `CellRef` to store a reference to the `Box`, which is a
    /// // pointer to the value on the heap, not the actual value.
    /// let boxed_ref: CellRef<'_, Box<usize>> = cb.borrow();
    /// assert_eq!(**boxed_ref, 5); // Notice the double deref to get the actual value.
    ///
    /// // By using `map` we can let `CellRef` store a reference directly to the value on the heap.
    /// let pure_ref: CellRef<'_, usize> = CellRef::map(boxed_ref, Box::as_ref);
    ///
    /// assert_eq!(*pure_ref, 5);
    /// ```
    ///
    /// We can also use `map` to get a reference to a sub-part of the borrowed
    /// value.
    ///
    /// ```rust
    /// # use rt_ref::{Cell, CellRef};
    ///
    /// let c = Cell::new((5, 'b'));
    /// let b1: CellRef<'_, (u32, char)> = c.borrow();
    /// let b2: CellRef<'_, u32> = CellRef::map(b1, |t| &t.0);
    /// assert_eq!(*b2, 5);
    /// ```
    pub fn map<U, F>(self, f: F) -> CellRef<'a, U>
    where
        F: FnOnce(&T) -> &U,
        U: ?Sized,
    {
        let flag = unsafe { &*(self.flag as *const _) };
        let value = unsafe { &*(self.value as *const _) };

        mem::forget(self);

        CellRef {
            flag,
            value: f(value),
        }
    }
}

impl<'a, T> Deref for CellRef<'a, T>
where
    T: ?Sized,
{
    type Target = T;

    fn deref(&self) -> &T {
        self.value
    }
}

impl<'a, T> Drop for CellRef<'a, T>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        self.flag.fetch_sub(1, Ordering::Release);
    }
}

impl<'a, T> Clone for CellRef<'a, T>
where
    T: ?Sized,
{
    /// Returns a clone of this `CellRef`.
    ///
    /// # Panics
    ///
    /// Panics if the number of references is `usize::MAX`:
    ///
    /// * Having 4 billion / 9 quintillion references to an object is not a
    ///   realistic scenario in most applications.
    /// * Applications that hold `CellRef`s with an ever-increasing reference
    ///   count is not supported by this library.
    ///
    ///     Reaching `usize::MAX` may be possible with
    ///     `std::mem::forget(CellRef::clone(&r))`.
    fn clone(&self) -> Self {
        self.try_clone()
            .unwrap_or_else(|e| panic!("Failed to clone `CellRef`: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use crate::RefOverflow;

    use super::CellRef;

    #[test]
    fn try_clone_returns_ok_when_ref_count_less_than_usize_max() {
        let flag = &AtomicUsize::new(1);
        let value = &1u32;
        let cell_ref = CellRef { flag, value };

        assert_eq!(1, cell_ref.flag.load(Ordering::SeqCst));

        let try_clone_result = cell_ref.try_clone();

        let cloned = try_clone_result.expect("try_clone_result to be ok");
        assert_eq!(2, cloned.flag.load(Ordering::SeqCst));
    }

    #[test]
    fn try_clone_returns_err_when_ref_count_equals_usize_max() {
        let flag = &AtomicUsize::new(usize::MAX);
        let value = &1u32;
        let cell_ref = CellRef { flag, value };

        assert_eq!(usize::MAX, cell_ref.flag.load(Ordering::SeqCst));

        let try_clone_result = cell_ref.try_clone();

        let e = try_clone_result.expect_err("try_clone_result to be err");
        assert_eq!(RefOverflow, e);
        assert!(e.source().is_none());

        // Ensure that the overflow is not persisted
        assert_eq!(usize::MAX, cell_ref.flag.load(Ordering::SeqCst));
    }

    #[test]
    fn clone_returns_cell_ref_when_ref_count_less_than_usize_max() {
        let flag = &AtomicUsize::new(1);
        let value = &1u32;
        let cell_ref = CellRef { flag, value };

        assert_eq!(1, cell_ref.flag.load(Ordering::SeqCst));

        let cloned = cell_ref.clone();

        assert_eq!(2, cell_ref.flag.load(Ordering::SeqCst));
        assert_eq!(2, cloned.flag.load(Ordering::SeqCst));
    }

    #[test]
    #[should_panic(expected = "Failed to clone `CellRef`: Ref count exceeded `usize::MAX`")]
    fn clone_panics_when_ref_count_equals_usize_max() {
        let flag = &AtomicUsize::new(usize::MAX);
        let value = &1u32;
        let cell_ref = CellRef { flag, value };

        assert_eq!(usize::MAX, cell_ref.flag.load(Ordering::SeqCst));

        let _clone = cell_ref.clone();
    }
}
