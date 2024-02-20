# Change log for the fnmatch-regex crate

## 0.2.0 (2022-06-11)

- INCOMPATIBLE change: the `fnmatch_regex::error::Error` class is now
  an enum that uses the quick-error library, not our own hand-rolled
  struct any more.
- INCOMPATIBLE change: the `glob_to_regex()` function returns a plain
  error object now, not a boxed one.
- Switch to Rust 2021 edition.
- Add an EditorConfig definitions file.
- Refactor the code to follow Rust best practices and some Clippy
  suggestions; among other things, the code will no longer panic.
- Refactor the code to avoid pushing to strings and vectors, using
  some internal iterator/adapter structs instead.
  Thanks to Kevin Reid for a couple of iterator-related suggestions!
- Add the `categories` and `keywords` Cargo package attributes.
- Use the rstest library for data-driven testing instead of doing it
  by ourselves.
- Use the itertools library to simplify some operations a whole lot.
  Thanks again to Kevin Reid for pointing it out to me!

## 0.1.0 (2021-06-22)

- First public release.

Peter Pentchev <[roam@ringlet.net](mailto:roam@ringlet.net)>
