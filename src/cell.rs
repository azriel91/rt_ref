use std::{
    cell::UnsafeCell,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{cell_ref::REF_LIMIT_MAX, BorrowFail, CellRef, CellRefMut};

macro_rules! borrow_panic {
    ($borrow_wanted:expr, $borrow_existing:expr) => {{
        panic!(
            "Expected to borrow `{type_name}` {borrow_wanted}, but it was already borrowed{borrow_existing}.",
            type_name = ::std::any::type_name::<T>(),
            borrow_wanted = $borrow_wanted,
            borrow_existing = $borrow_existing,
        )
    }};
}

/// A custom cell container that is a `RefCell` with thread-safety.
#[cfg_attr(not(feature = "unsafe_debug"), derive(Debug))]
pub struct Cell<T> {
    flag: AtomicUsize,
    inner: UnsafeCell<T>,
}

impl<T> Cell<T> {
    /// Create a new cell, similar to `RefCell::new`
    pub fn new(inner: T) -> Self {
        Cell {
            flag: AtomicUsize::new(0),
            inner: UnsafeCell::new(inner),
        }
    }

    /// Consumes this cell and returns ownership of `T`.
    pub fn into_inner(self) -> T {
        self.inner.into_inner()
    }

    /// Get an immutable reference to the inner data.
    ///
    /// Absence of write accesses is checked at run-time.
    ///
    /// # Panics
    ///
    /// This function will panic if there is a mutable reference to the data
    /// already in use.
    pub fn borrow(&self) -> CellRef<T> {
        if !self.check_flag_read() {
            borrow_panic!("immutably", " mutably");
        }

        CellRef {
            flag: &self.flag,
            value: unsafe { &*self.inner.get() },
        }
    }

    /// Get an immutable reference to the inner data.
    ///
    /// Absence of write accesses is checked at run-time. If access is not
    /// possible, `None` is returned.
    pub fn try_borrow(&self) -> Result<CellRef<T>, BorrowFail> {
        if self.check_flag_read() {
            Ok(CellRef {
                flag: &self.flag,
                value: unsafe { &*self.inner.get() },
            })
        } else {
            Err(BorrowFail::BorrowConflictImm)
        }
    }

    /// Get a mutable reference to the inner data.
    ///
    /// Exclusive access is checked at run-time.
    ///
    /// # Panics
    ///
    /// This function will panic if there are any references to the data already
    /// in use.
    pub fn borrow_mut(&self) -> CellRefMut<T> {
        if !self.check_flag_write() {
            borrow_panic!("mutably", "");
        }

        CellRefMut {
            flag: &self.flag,
            value: unsafe { &mut *self.inner.get() },
        }
    }

    /// Get a mutable reference to the inner data.
    ///
    /// Exclusive access is checked at run-time. If access is not possible,
    /// `None` is returned.
    pub fn try_borrow_mut(&self) -> Result<CellRefMut<T>, BorrowFail> {
        if self.check_flag_write() {
            Ok(CellRefMut {
                flag: &self.flag,
                value: unsafe { &mut *self.inner.get() },
            })
        } else {
            Err(BorrowFail::BorrowConflictMut)
        }
    }

    /// Gets exclusive access to the inner value, bypassing the Cell.
    ///
    /// Exclusive access is checked at compile time.
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.inner.get() }
    }

    /// Make sure we are allowed to acquire a read lock, and increment the read
    /// count by 1
    fn check_flag_read(&self) -> bool {
        loop {
            let val = self.flag.load(Ordering::Acquire);

            if val >= REF_LIMIT_MAX {
                return false;
            }

            if self
                .flag
                .compare_exchange_weak(val, val + 1, Ordering::AcqRel, Ordering::Acquire)
                == Ok(val)
            {
                return true;
            }
        }
    }

    /// Make sure we are allowed to acquire a write lock, and then set the write
    /// lock flag.
    fn check_flag_write(&self) -> bool {
        self.flag
            .compare_exchange(0, usize::MAX, Ordering::AcqRel, Ordering::Acquire)
            == Ok(0)
    }
}

#[cfg(feature = "unsafe_debug")]
use std::fmt;

#[cfg(feature = "unsafe_debug")]
impl<T> fmt::Debug for Cell<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cell")
            .field("flag", &self.flag)
            .field("inner", unsafe { &*self.inner.get() })
            .finish()
    }
}

unsafe impl<T> Sync for Cell<T> where T: Sync {}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::Cell;
    use crate::{BorrowFail, CellRef, CellRefMut};

    #[test]
    fn allow_multiple_reads() {
        let cell = Cell::new(5);

        let a = cell.borrow();
        let b = cell.borrow();

        assert_eq!(10, *a + *b);
    }

    #[test]
    fn allow_clone_reads() {
        let cell = Cell::new(5);

        let a = cell.borrow();
        let b = a.clone();

        assert_eq!(10, *a + *b);
    }

    #[test]
    fn allow_single_write() {
        let cell = Cell::new(5);

        {
            let mut a = cell.borrow_mut();
            *a += 2;
            *a += 3;
        }

        assert_eq!(10, *cell.borrow());
    }

    #[test]
    fn get_mut_allows_write() {
        let mut cell = Cell::new(5);

        {
            let a = cell.get_mut();
            *a += 2;
            *a += 3;
        }

        assert_eq!(10, *cell.borrow());
    }

    #[test]
    fn into_inner_returns_value() {
        #[derive(Debug, PartialEq)]
        struct A(usize);

        let mut cell = Cell::new(A(5));

        {
            let a = cell.get_mut();
            a.0 += 2;
            a.0 += 3;
        }

        assert_eq!(A(10), cell.into_inner());
    }

    #[test]
    #[should_panic(
        expected = "Expected to borrow `i32` immutably, but it was already borrowed mutably."
    )]
    fn panic_write_and_read() {
        let cell = Cell::new(5);

        let mut a = cell.borrow_mut();
        *a = 7;

        assert_eq!(7, *cell.borrow());
    }

    #[test]
    #[should_panic(expected = "Expected to borrow `i32` mutably, but it was already borrowed.")]
    fn panic_write_and_write() {
        let cell = Cell::new(5);

        let mut a = cell.borrow_mut();
        *a = 7;

        assert_eq!(7, *cell.borrow_mut());
    }

    #[test]
    #[should_panic(expected = "Expected to borrow `i32` mutably, but it was already borrowed.")]
    fn panic_read_and_write() {
        let cell = Cell::new(5);

        let _a = cell.borrow();

        assert_eq!(7, *cell.borrow_mut());
    }

    #[test]
    fn try_write_and_read() {
        let cell = Cell::new(5);

        let mut a = cell.try_borrow_mut().unwrap();
        *a = 7;

        assert_eq!(
            BorrowFail::BorrowConflictImm,
            cell.try_borrow().unwrap_err()
        );

        *a = 8;
    }

    #[test]
    fn try_write_and_write() {
        let cell = Cell::new(5);

        let mut a = cell.try_borrow_mut().unwrap();
        *a = 7;

        assert_eq!(
            BorrowFail::BorrowConflictMut,
            cell.try_borrow_mut().unwrap_err()
        );

        *a = 8;
    }

    #[test]
    fn try_read_and_write() {
        let cell = Cell::new(5);

        let _a = cell.try_borrow().unwrap();

        assert_eq!(
            BorrowFail::BorrowConflictMut,
            cell.try_borrow_mut().unwrap_err()
        );
    }

    #[test]
    fn cloned_borrow_does_not_allow_write() {
        let cell = Cell::new(5);

        let a = cell.borrow();
        let b = a.clone();

        drop(a);

        assert_eq!(
            BorrowFail::BorrowConflictMut,
            cell.try_borrow_mut().unwrap_err()
        );
        assert_eq!(5, *b);
    }

    #[test]
    fn ref_with_non_sized() {
        let r: CellRef<'_, [i32]> = CellRef {
            flag: &AtomicUsize::new(1),
            value: &[2, 3, 4, 5][..],
        };

        assert_eq!(&*r, &[2, 3, 4, 5][..]);
    }

    #[test]
    fn ref_with_non_sized_clone() {
        let r: CellRef<'_, [i32]> = CellRef {
            flag: &AtomicUsize::new(1),
            value: &[2, 3, 4, 5][..],
        };
        let rr = r.clone();

        assert_eq!(&*r, &[2, 3, 4, 5][..]);
        assert_eq!(r.flag.load(Ordering::SeqCst), 2);

        assert_eq!(&*rr, &[2, 3, 4, 5][..]);
        assert_eq!(rr.flag.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn ref_with_trait_obj() {
        let ra: CellRef<'_, dyn std::any::Any> = CellRef {
            flag: &AtomicUsize::new(1),
            value: &2i32,
        };

        assert_eq!(ra.downcast_ref::<i32>().unwrap(), &2i32);
    }

    #[test]
    fn ref_mut_with_non_sized() {
        let mut r: CellRefMut<'_, [i32]> = CellRefMut {
            flag: &AtomicUsize::new(1),
            value: &mut [2, 3, 4, 5][..],
        };

        assert_eq!(&mut *r, &mut [2, 3, 4, 5][..]);
    }

    #[test]
    fn ref_mut_with_trait_obj() {
        let mut ra: CellRefMut<'_, dyn std::any::Any> = CellRefMut {
            flag: &AtomicUsize::new(1),
            value: &mut 2i32,
        };

        assert_eq!(ra.downcast_mut::<i32>().unwrap(), &mut 2i32);
    }

    #[test]
    fn ref_map_box() {
        let cell = Cell::new(Box::new(10));

        let r: CellRef<'_, Box<usize>> = cell.borrow();
        assert_eq!(&**r, &10);

        let rr: CellRef<'_, usize> = cell.borrow().map(Box::as_ref);
        assert_eq!(&*rr, &10);
    }

    #[test]
    fn ref_map_preserves_flag() {
        let cell = Cell::new(Box::new(10));

        let r: CellRef<'_, Box<usize>> = cell.borrow();
        assert_eq!(cell.flag.load(Ordering::SeqCst), 1);
        let _nr: CellRef<'_, usize> = r.map(Box::as_ref);
        assert_eq!(cell.flag.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn ref_map_retains_borrow() {
        let cell = Cell::new(Box::new(10));

        let _r: CellRef<'_, usize> = cell.borrow().map(Box::as_ref);
        assert_eq!(cell.flag.load(Ordering::SeqCst), 1);

        let _rr: CellRef<'_, usize> = cell.borrow().map(Box::as_ref);
        assert_eq!(cell.flag.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn ref_map_drops_borrow() {
        let cell = Cell::new(Box::new(10));

        let r: CellRef<'_, usize> = cell.borrow().map(Box::as_ref);

        assert_eq!(cell.flag.load(Ordering::SeqCst), 1);
        drop(r);
        assert_eq!(cell.flag.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn ref_mut_map_box() {
        let cell = Cell::new(Box::new(10));

        {
            let mut r: CellRefMut<'_, Box<usize>> = cell.borrow_mut();
            assert_eq!(&mut **r, &mut 10);
        }
        {
            let mut rr: CellRefMut<'_, usize> = cell.borrow_mut().map(Box::as_mut);
            assert_eq!(&mut *rr, &mut 10);
        }
    }

    #[test]
    fn ref_mut_map_preserves_flag() {
        let cell = Cell::new(Box::new(10));

        let r: CellRefMut<'_, Box<usize>> = cell.borrow_mut();
        assert_eq!(cell.flag.load(Ordering::SeqCst), usize::MAX);
        let _nr: CellRefMut<'_, usize> = r.map(Box::as_mut);
        assert_eq!(cell.flag.load(Ordering::SeqCst), usize::MAX);
    }

    #[test]
    #[should_panic(
        expected = "Expected to borrow `alloc::boxed::Box<usize>` mutably, but it was already borrowed."
    )]
    fn ref_mut_map_retains_mut_borrow() {
        let cell = Cell::new(Box::new(10));

        let _rr: CellRefMut<'_, usize> = cell.borrow_mut().map(Box::as_mut);

        let _ = cell.borrow_mut();
    }

    #[test]
    fn ref_mut_map_drops_borrow() {
        let cell = Cell::new(Box::new(10));

        let r: CellRefMut<'_, usize> = cell.borrow_mut().map(Box::as_mut);

        assert_eq!(cell.flag.load(Ordering::SeqCst), usize::MAX);
        drop(r);
        assert_eq!(cell.flag.load(Ordering::SeqCst), 0);
    }

    #[cfg(not(feature = "unsafe_debug"))]
    #[test]
    fn debug() {
        assert_eq!(
            "Cell { flag: 0, inner: UnsafeCell { .. } }",
            format!("{:?}", Cell::new(1))
        );
        assert_eq!(
            "Cell { flag: 0, inner: UnsafeCell { .. } }",
            format!("{:?}", Cell::new("a"))
        );

        #[allow(dead_code)]
        #[derive(Debug)]
        struct A(u32);
        assert_eq!(
            "Cell { flag: 0, inner: UnsafeCell { .. } }",
            format!("{:?}", Cell::new(A(1)))
        );

        #[allow(dead_code)]
        #[derive(Debug)]
        struct B {
            value: u32,
        }
        assert_eq!(
            "Cell { flag: 0, inner: UnsafeCell { .. } }",
            format!("{:?}", Cell::new(B { value: 1 }))
        );
    }

    #[cfg(feature = "unsafe_debug")]
    #[test]
    fn unsafe_debug() {
        assert_eq!("Cell { flag: 0, inner: 1 }", format!("{:?}", Cell::new(1)));
        assert_eq!(
            "Cell { flag: 0, inner: \"a\" }",
            format!("{:?}", Cell::new("a"))
        );

        #[allow(dead_code)]
        #[derive(Debug)]
        struct A(u32);
        assert_eq!(
            "Cell { flag: 0, inner: A(1) }",
            format!("{:?}", Cell::new(A(1)))
        );

        #[allow(dead_code)]
        #[derive(Debug)]
        struct B {
            value: u32,
        }
        assert_eq!(
            "Cell { flag: 0, inner: B { value: 1 } }",
            format!("{:?}", Cell::new(B { value: 1 }))
        );
    }
}
