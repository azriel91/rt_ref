# Changelog

## 0.1.2 (unreleased)

* Fix `CellRef` unsoundness by panicking when number of references is `usize::MAX`. ([#1], [#2])
* Add `Ref::try_clone` for recoverable clonability. ([#1], [#2])

[#1]: https://github.com/azriel91/rt_ref/issues/1
[#2]: https://github.com/azriel91/rt_ref/pull/2

## 0.1.1 (2022-06-27)

* Use `compare_exchange_weak` for performance gain.

## 0.1.0 (2022-06-27)

* Initial version with `BorrowFail`, `Cell`, `CellRef`, `CellRefMut`, `Ref`, `RefMut`.
