//! `Ref` types with internal mutability that implement `Send` and `Sync`.
//!
//! These types are shared by [`rt_map`] and [`rt_vec`].
//!
//!
//! ## Usage
//!
//! Add the following to `Cargo.toml`:
//!
//! ```toml
//! rt_ref = "0.2.0" # or
//! rt_ref = { version = "0.2.0", features = ["unsafe_debug"] }
//! ```
//!
//! In code:
//!
//! ```rust
//! use rt_ref::{Cell, Ref, RefMut};
//!
//! let a = 1;
//!
//! // Insert a value into a collection, wrapped with `Cell`.
//! let mut v = Vec::new();
//! v.push(Cell::new(a));
//!
//! let v = v; // v is now compile-time immutable.
//! let a = v.get(0).map(|cell| RefMut::new(cell.borrow_mut()));
//! a.map(|mut a| {
//!     *a += 2;
//! });
//!
//! let a = v.get(0).map(|cell| Ref::new(cell.borrow()));
//! assert_eq!(Some(3), a.map(|a| *a));
//! ```
//!
//!
//! ### Features
//!
//! #### `"unsafe_debug"`:
//!
//! The borrowed reference will use the inner type's `Debug` implementation when
//! formatted.
//!
//! ```rust
//! use rt_ref::{Cell, Ref, RefMut};
//!
//! let mut v = Vec::new();
//! v.push(Cell::new("a"));
//!
//! #[cfg(not(feature = "unsafe_debug"))]
//! assert_eq!(
//!     r#"[Cell { flag: 0, inner: UnsafeCell { .. } }]"#,
//!     format!("{v:?}")
//! );
//! #[cfg(feature = "unsafe_debug")]
//! assert_eq!(r#"[Cell { flag: 0, inner: "a" }]"#, format!("{v:?}"));
//! ```
//!
//!
//! [`rt_map`]: https://crates.io/crates/rt_map
//! [`rt_vec`]: https://crates.io/crates/rt_vec

pub use crate::{
    borrow_fail::BorrowFail, cell::Cell, cell_ref::CellRef, cell_ref_mut::CellRefMut, r#ref::Ref,
    ref_mut::RefMut, ref_overflow::RefOverflow,
};

mod borrow_fail;
mod cell;
mod cell_ref;
mod cell_ref_mut;
mod r#ref;
mod ref_mut;
mod ref_overflow;
