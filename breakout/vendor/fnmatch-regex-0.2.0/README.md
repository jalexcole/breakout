# fnmatch-regex - build regular expressions to match glob-style patterns

This crate currently provides a single function, `glob_to_regex`, that
converts a glob-style pattern with some shell extensions to a regular
expression. Note that it only handles text pattern matching, there are
no attempts to verify or construct any filesystem paths.

The glob-style pattern features currently supported are:

- any character except `?`, `*`, `[`, `\`, or `{` is matched literally

- `?` matches any single character except a slash (`/`)

- `*` matches any sequence of zero or more characters that does not
  contain a slash (`/`)

- a backslash allows the next character to be matched literally, except
  for the `\a`, `\b`, `\e`, `\n`, `\r`, and `\v` sequences

- a `[...]` character class supports ranges, negation if the very first
  character is `!`, backslash-escaping, and also matching
  a `]` character if it is the very first character possibly after
  the `!` one (e.g. `[]]` would only match a single `]` character)

- an `{a,bbb,cc}` alternation supports backslash-escaping, but not
  nested alternations or character classes yet

Note that the `*` and `?` wildcard patterns, as well as the character
classes, will never match a slash.

Examples:

- `abc.txt` would only match `abc.txt`

- `foo/test?.txt` would match e.g. `foo/test1.txt` or `foo/test".txt`,
  but not `foo/test/.txt`

- `/etc/c[--9].conf` would match e.g. `/etc/c-.conf`, `/etc/c..conf`,
   or `/etc/7.conf`, but not `/etc/c/.conf`

- `linux-[0-9]*-{generic,aws}` would match `linux-5.2.27b1-generic`
  and `linux-4.0.12-aws`, but not `linux-unsigned-5.2.27b1-generic`

Note that the negation modifier for character classes is `!`, not `^`. 

    let re_name = fnmatch_regex::glob_to_regex("linux-[0-9]*-{generic,aws}")?;
    for name in &[
        "linux-5.2.27b1-generic",
        "linux-4.0.12-aws",
        "linux-unsigned-5.2.27b1-generic"
    ] {
        let okay = re_name.is_match(name);
        println!(
            "{}: {}",
            name,
            match okay { true => "yes", false => "no" },
        );
        assert!(okay == !name.contains("unsigned"));
    }
