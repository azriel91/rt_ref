# Changelog

## 0.2.0 (2022-07-15)

* Restrict visibility to `CellRef(Mut)::{flag, value}` to crate. ([#5], [#6])

[#5]: https://github.com/azriel91/rt_ref/issues/5
[#6]: https://github.com/azriel91/rt_ref/pull/6

## 0.1.3 (2022-07-14)

* Limit number of `CellRef` references to `isize::MAX`, as it wouldn't realistically fit into memory. ([#1], [#3])
* Add `"unsafe_debug"` feature to use borrowed type's `Debug` implementation. ([#4])

[#3]: https://github.com/azriel91/rt_ref/pull/3
[#4]: https://github.com/azriel91/rt_ref/pull/4

## 0.1.2 (2022-07-01)

* Fix `CellRef` unsoundness by panicking when number of references is `usize::MAX`. ([#1], [#2])
* Add `Ref::try_clone` for recoverable clonability. ([#1], [#2])

[#1]: https://github.com/azriel91/rt_ref/issues/1
[#2]: https://github.com/azriel91/rt_ref/pull/2

## 0.1.1 (2022-06-27)

* Use `compare_exchange_weak` for performance gain.

## 0.1.0 (2022-06-27)

* Initial version with `BorrowFail`, `Cell`, `CellRef`, `CellRefMut`, `Ref`, `RefMut`.
