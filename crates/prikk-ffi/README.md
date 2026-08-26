# prikk-ffi

Prikk's only FFI surface -- on Windows, it confirms that an open filesystem handle still refers to
the object it was bound to, and it compiles to nothing on other platforms. This crate is not meant
to be used as a dependency on its own; its Rust API may change without notice before `prikk`
reaches 1.0. If you're looking for the tool itself, that's the `prikk` CLI.
