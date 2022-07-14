# ♐ rt_ref

[![Crates.io](https://img.shields.io/crates/v/rt_ref.svg)](https://crates.io/crates/rt_ref)
[![docs.rs](https://img.shields.io/docsrs/rt_ref)](https://docs.rs/rt_ref)
[![CI](https://github.com/azriel91/rt_ref/workflows/CI/badge.svg)](https://github.com/azriel91/rt_ref/actions/workflows/ci.yml)
[![Coverage Status](https://codecov.io/gh/azriel91/rt_ref/branch/main/graph/badge.svg)](https://codecov.io/gh/azriel91/rt_ref)

`Ref` types with internal mutability that implement `Send` and `Sync`.

These types are shared by [`rt_map`] and [`rt_vec`].


## Usage

Add the following to `Cargo.toml`:

```toml
rt_ref = "0.1.3" # or
rt_ref = { version = "0.1.3", features = ["unsafe_debug"] }
```

In code:

```rust
use rt_ref::{Cell, Ref, RefMut};

let a = 1;

// Insert a value into a collection, wrapped with `Cell`.
let mut v = Vec::new();
v.push(Cell::new(a));

let v = v; // v is now compile-time immutable.
let a = v.get(0).map(|cell| RefMut::new(cell.borrow_mut()));
a.map(|mut a| {
    *a += 2;
});

let a = v.get(0).map(|cell| Ref::new(cell.borrow()));
assert_eq!(Some(3), a.map(|a| *a));
```


### Features

#### `"unsafe_debug"`:

The borrowed reference will use the inner type's `Debug` implementation when formatted.

```rust
use rt_ref::{Cell, Ref, RefMut};

let mut v = Vec::new();
v.push(Cell::new("a"));

#[cfg(not(feature = "unsafe_debug"))]
assert_eq!(
    r#"[Cell { flag: 0, inner: UnsafeCell { .. } }]"#,
    format!("{v:?}")
);
#[cfg(feature = "unsafe_debug")]
assert_eq!(r#"[Cell { flag: 0, inner: "a" }]"#, format!("{v:?}"));
```


## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.


### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.


[`rt_map`]: https://crates.io/crates/rt_map
[`rt_vec`]: https://crates.io/crates/rt_vec
