use std::{cmp::PartialEq, fmt, ops::Deref};

use crate::{CellRef, RefOverflow};

/// Reference to a value.
pub struct Ref<'a, V>
where
    V: 'a,
{
    pub(crate) inner: CellRef<'a, V>,
}

impl<'a, V> Ref<'a, V> {
    /// Returns a new `Ref`.
    pub fn new(inner: CellRef<'a, V>) -> Self {
        Self { inner }
    }

    /// Returns a clone of this `Ref`.
    ///
    /// This method allows handling of reference overflows, but:
    ///
    /// * Having 2 billion (32-bit system) / 9 quintillion (64-bit system)
    ///   references to an object is not a realistic scenario in most
    ///   applications.
    ///
    /// * Applications that hold `Ref`s with an ever-increasing reference count
    ///   are not supported by this library.
    ///
    ///     Reaching `isize::MAX` may be possible with
    ///     `std::mem::forget(Ref::clone(&r))`.
    pub fn try_clone(&self) -> Result<Self, RefOverflow> {
        self.inner.try_clone().map(Self::new)
    }
}

impl<'a, V> Deref for Ref<'a, V> {
    type Target = V;

    fn deref(&self) -> &V {
        &self.inner
    }
}

impl<'a, V> fmt::Debug for Ref<'a, V>
where
    V: fmt::Debug + 'a,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let inner: &V = self;
        f.debug_struct("Ref").field("inner", inner).finish()
    }
}

impl<'a, V> PartialEq for Ref<'a, V>
where
    V: PartialEq + 'a,
{
    fn eq(&self, other: &Self) -> bool {
        let r_self: &V = self;
        let r_other: &V = other;
        r_self == r_other
    }
}

impl<'a, V> Clone for Ref<'a, V> {
    /// Returns a clone of this `Ref`.
    ///
    /// # Panics
    ///
    /// Panics if the number of references is `isize::MAX`:
    ///
    /// * Having 2 billion / 9 quintillion references to an object is not a
    ///   realistic scenario in most applications.
    /// * Applications that hold `Ref`s with an ever-increasing reference count
    ///   are not supported by this library.
    ///
    ///     Reaching `isize::MAX` may be possible with
    ///     `std::mem::forget(Ref::clone(&r))`.
    fn clone(&self) -> Self {
        Ref {
            inner: self.inner.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fmt::{self, Write},
        sync::atomic::{AtomicUsize, Ordering},
    };

    use crate::{cell_ref::REF_LIMIT_MAX, CellRef, RefOverflow};

    use super::Ref;

    #[test]
    fn debug_includes_inner_field() -> fmt::Result {
        let flag = AtomicUsize::new(0);
        let value = A(1);
        let r#ref = Ref::new(CellRef {
            flag: &flag,
            value: &value,
        });

        let mut debug_string = String::with_capacity(64);
        write!(&mut debug_string, "{:?}", r#ref)?;
        assert_eq!("Ref { inner: A(1) }", debug_string.as_str());

        Ok(())
    }

    #[test]
    fn partial_eq_compares_value() -> fmt::Result {
        let flag = AtomicUsize::new(0);
        let value = A(1);
        let r#ref = Ref::new(CellRef {
            flag: &flag,
            value: &value,
        });

        assert_eq!(
            Ref::new(CellRef {
                flag: &flag,
                value: &value,
            }),
            r#ref
        );
        assert_ne!(
            Ref::new(CellRef {
                flag: &flag,
                value: &A(2),
            }),
            r#ref
        );

        Ok(())
    }

    #[test]
    fn try_clone_returns_ok_when_ref_count_less_than_usize_max() {
        let flag = &AtomicUsize::new(1);
        let value = &A(1);
        let ref_0 = Ref::new(CellRef { flag, value });

        assert_eq!(1, ref_0.inner.flag.load(Ordering::SeqCst));

        let try_clone_result = ref_0.try_clone();

        let ref_1 = try_clone_result.expect("try_clone_result to be ok");
        assert_eq!(2, ref_0.inner.flag.load(Ordering::SeqCst));
        assert_eq!(2, ref_1.inner.flag.load(Ordering::SeqCst));
    }

    #[test]
    fn try_clone_returns_err_when_ref_count_equals_usize_max() {
        let flag = &AtomicUsize::new(REF_LIMIT_MAX);
        let value = &A(1);
        let ref_0 = Ref::new(CellRef { flag, value });

        assert_eq!(REF_LIMIT_MAX, ref_0.inner.flag.load(Ordering::SeqCst));

        let try_clone_result = ref_0.try_clone();

        let e = try_clone_result.expect_err("try_clone_result to be err");
        assert_eq!(RefOverflow, e);

        // Ensure that the overflow is not persisted
        assert_eq!(REF_LIMIT_MAX, ref_0.inner.flag.load(Ordering::SeqCst));
    }

    #[test]
    fn clone_increments_cell_ref_count() {
        let flag = &AtomicUsize::new(1);
        let value = &A(1);
        let ref_0 = Ref::new(CellRef { flag, value });

        assert_eq!(1, ref_0.inner.flag.load(Ordering::SeqCst));

        let ref_1 = ref_0.clone();

        assert_eq!(2, ref_0.inner.flag.load(Ordering::SeqCst));
        assert_eq!(2, ref_1.inner.flag.load(Ordering::SeqCst));
    }

    #[test]
    #[should_panic(expected = "Failed to clone `CellRef`: Ref count exceeded `isize::MAX`")]
    fn clone_panics_when_ref_count_equals_usize_max() {
        let flag = &AtomicUsize::new(REF_LIMIT_MAX);
        let value = &A(1);
        let ref_0 = Ref::new(CellRef { flag, value });

        assert_eq!(REF_LIMIT_MAX, ref_0.inner.flag.load(Ordering::SeqCst));

        let _cloned = ref_0.clone();
    }

    #[derive(Debug, Clone, PartialEq)]
    struct A(usize);
}
