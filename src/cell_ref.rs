use std::{
    mem,
    ops::Deref,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::RefOverflow;

/// An immutable reference to data in a `Cell`.
///
/// Access the value via `std::ops::Deref` (e.g. `*val`)
#[derive(Debug)]
pub struct CellRef<'a, T>
where
    T: ?Sized + 'a,
{
    pub(crate) flag: &'a AtomicUsize,
    pub(crate) value: &'a T,
}

/// Cast max `isize` as `usize`, so we don't have to do it in multiple places.
pub(crate) const REF_LIMIT_MAX: usize = isize::MAX as usize;

impl<'a, T> CellRef<'a, T>
where
    T: ?Sized,
{
    /// Returns a clone of this `CellRef`.
    ///
    /// This method allows handling of reference overflows, but:
    ///
    /// * Having 2 billion (32-bit system) / 9 quintillion (64-bit system)
    ///   references to an object is not a realistic scenario in most
    ///   applications.
    ///
    /// * Applications that hold `CellRef`s with an ever-increasing reference
    ///   count are not supported by this library.
    ///
    ///     Reaching `isize::MAX` may be possible with
    ///     `std::mem::forget(CellRef::clone(&r))`.
    // https://github.com/rust-lang/rust-clippy/issues/14275
    #[allow(clippy::doc_overindented_list_items)]
    pub fn try_clone(&self) -> Result<Self, RefOverflow> {
        let previous_value = self.flag.fetch_add(1, Ordering::Relaxed);

        let overflow = previous_value >= REF_LIMIT_MAX;
        if unlikely(overflow) {
            self.flag.fetch_sub(1, Ordering::Relaxed);
            Err(RefOverflow)
        } else {
            Ok(CellRef {
                flag: self.flag,
                value: self.value,
            })
        }
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
    /// Panics if the number of references is `isize::MAX`:
    ///
    /// * Having 2 billion / 9 quintillion references to an object is not a
    ///   realistic scenario in most applications.
    /// * Applications that hold `CellRef`s with an ever-increasing reference
    ///   count are not supported by this library.
    ///
    ///     Reaching `isize::MAX` may be possible with
    ///     `std::mem::forget(CellRef::clone(&r))`.
    // https://github.com/rust-lang/rust-clippy/issues/14275
    #[allow(clippy::doc_overindented_list_items)]
    fn clone(&self) -> Self {
        self.try_clone()
            .unwrap_or_else(|e| panic!("Failed to clone `CellRef`: {e}"))
    }
}

/// Trick to mimic `std::intrinsics::unlikely` on stable Rust.
#[cold]
#[inline(always)]
fn cold() {}

#[inline(always)]
fn unlikely(cond: bool) -> bool {
    if cond {
        cold();
    }
    cond
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use crate::RefOverflow;

    use super::{CellRef, REF_LIMIT_MAX};

    #[test]
    fn try_clone_returns_ok_when_ref_count_less_than_isize_max() {
        let flag = &AtomicUsize::new(1);
        let value = &1u32;
        let cell_ref = CellRef { flag, value };

        assert_eq!(1, cell_ref.flag.load(Ordering::SeqCst));

        let try_clone_result = cell_ref.try_clone();

        let cloned = try_clone_result.expect("try_clone_result to be ok");
        assert_eq!(2, cloned.flag.load(Ordering::SeqCst));
    }

    #[test]
    fn try_clone_returns_err_when_ref_count_equals_isize_max() {
        let flag = &AtomicUsize::new(REF_LIMIT_MAX);
        let value = &1u32;
        let cell_ref = CellRef { flag, value };

        assert_eq!(REF_LIMIT_MAX, cell_ref.flag.load(Ordering::SeqCst));

        let try_clone_result = cell_ref.try_clone();

        let e = try_clone_result.expect_err("try_clone_result to be err");
        assert_eq!(RefOverflow, e);
        assert!(e.source().is_none());

        // Ensure that the overflow is not persisted
        assert_eq!(REF_LIMIT_MAX, cell_ref.flag.load(Ordering::SeqCst));
    }

    #[test]
    fn clone_returns_cell_ref_when_ref_count_less_than_isize_max() {
        let flag = &AtomicUsize::new(1);
        let value = &1u32;
        let cell_ref = CellRef { flag, value };

        assert_eq!(1, cell_ref.flag.load(Ordering::SeqCst));

        let cloned = cell_ref.clone();

        assert_eq!(2, cell_ref.flag.load(Ordering::SeqCst));
        assert_eq!(2, cloned.flag.load(Ordering::SeqCst));
    }

    #[test]
    #[should_panic(expected = "Failed to clone `CellRef`: Ref count exceeded `isize::MAX`")]
    fn clone_panics_when_ref_count_equals_isize_max() {
        let flag = &AtomicUsize::new(REF_LIMIT_MAX);
        let value = &1u32;
        let cell_ref = CellRef { flag, value };

        assert_eq!(REF_LIMIT_MAX, cell_ref.flag.load(Ordering::SeqCst));

        let _clone = cell_ref.clone();
    }
}
