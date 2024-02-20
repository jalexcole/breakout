# unsafe-any-ors [![CI](https://github.com/orphanage-rs/rust-unsafe-any/actions/workflows/general.yaml/badge.svg)](https://github.com/orphanage-rs/rust-unsafe-any/actions/workflows/general.yaml)

> Convenience traits for unsafe downcasting from trait objects to concrete types.

## Overview

This crate defines two new traits `UncheckedAnyDowncast` and `UncheckedAnyMutDowncast`,
which define methods for downcasting to any type that implements `Any` from
implemented trait objects.

It also defines two convenience implementations of these traits for `&'a Any`
and `&'a mut Any`, which are the most common trait objects that you might
downcast from.

## Example:

```rust
let a = box 7u as Box<Any>;
unsafe { assert_eq!(*a.downcast_ref_unchecked::<uint>(), 7u); }
```

## License

MIT

