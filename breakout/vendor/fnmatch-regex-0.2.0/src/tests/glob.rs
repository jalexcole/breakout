//! Test the shell glob matching functionality.

/*
 * Copyright (c) 2021, 2022  Peter Pentchev <roam@ringlet.net>
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

use crate::error::Error as FError;
use crate::glob as fglob;

#[rstest::rstest]
#[case(
    "test_const",
    "con.st",
    &["con.st"],
    &[
        "conAst",
        "con!st",
        "con?st",
        "a con.st",
        "con.stant",
        "a con.stant",
    ],
)]
#[case(
    "test_wildcards",
    "this/is/?.test",
    &[
        "this/is/a.test",
        "this/is/b.test",
        "this/is/?.test",
        "this/is/*.test",
    ],
    &[
        "this/is/aa.test",
        "this/is/.test",
        "this/is/some.test",
        "this/is/a?.test",
        "that/is/a.test",
        "this/is/not/a.test",
    ],
)]
#[case(
    "test_wildcards",
    "a?b",
    &["aab", "a.b", "a?b", "a*b"],
    &["a/b"],
)]
#[case(
    "test_class_simple",
    "[0-9]",
    &["5", "0", "9"],
    &["50", "", "a", " ", "?", "+", "-", ".", "/"],
)]
#[case(
    "test_class_simple",
    "[!0-9]",
    &["a", " ", "?", "+", "-", "."],
    &["5", "0", "9", "50", "", "/"],
)]
#[case(
    "test_class_simple",
    "[ab.-9c]",
    &["5", "0", "9", ".", "a"],
    &["50", "ab", "a0", "5b", "", " ", "?", "+", "-", "/"],
)]
#[case(
    "test_class_simple",
    "[ab+-9c]",
    &["5", "0", "9", ".", "a", "+", "-"],
    &["50", "ab", "a0", "5b", "", " ", "?", "/"],
)]
#[case(
    "test_class_simple",
    "[ab0-9c-]",
    &["5", "0", "9", "a", "-"],
    &["50", "ab", "a0", "5b", "", " ", "+", ".", "?", "/"],
)]
#[case(
    "test_class_simple",
    "[+a-]",
    &["+", "a", "-"],
    &[
        "50", "ab", "a0", "5b", "5", "0", "9", "", " ", ".", "?", "/",
    ],
)]
#[case(
    "test_class_simple",
    "[0-9-]",
    &["-", "5", "0", "9"],
    &["50", "ab", "a0", "5b", "a", "", " ", ".", "?", "/", "+"],
)]
#[case(
    "test_class_simple",
    "[]]",
    &["]"],
    &["[]", "[[", "]]", "[[]", "[", "", " ", ".", "?", "/", "+"],
)]
#[case(
    "test_class_simple",
    "[!]]",
    &["[", " ", ".", "?", "+"],
    &["[]", "[[", "]]", "[[]", "]", "", "/"],
)]
#[case(
    "test_class_simple",
    "[-a]",
    &["-", "a"],
    &["--", "-a", "a-", "aa", " ", ".", "?", "+", "]", "", "/"],
)]
#[case(
    "test_class_simple",
    "[!-a]",
    &[" ", ".", "?", "+", "]"],
    &["--", "-a", "a-", "aa", "", "-", "a", "/"],
)]
#[case(
    "test_alternates",
    "look at {th?is,that,...*}",
    &["look at th?is", "look at that", "look at ...*"],
    &[
        "look at this",
        "look at ths",
        "look at ",
        "look at that and stuff",
    ],
)]
#[case(
    "test_alternates",
    "whee{} whoo",
    &["whee{} whoo"],
    &["whee whoo", "whee{ whoo", "whee} whoo"],
)]
#[case(
    "test_escape",
    r"hello\[\]\$\?\.\{\*\}",
    &["hello[]$?.{*}"],
    &["hello", "hello[]", "hello$"],
)]
#[case(
    "test_escape",
    r"hello\\\b\e\f\n\r\t\v",
    &["hello\\\x08\x1b\x0c\n\r\t\x0b"],
    &[r"hello\\\a\b\e\f\n\r\t\v"],
)]
#[case(
    "test_escape",
    r"hell[o$\$\-\]]",
    &["hello", "hell$", "hell]", "hell-", "hell$"],
    &["hell", "hello-", "hell%"],
)]
#[case(
    "test_escape",
    r"hello{\\,\b,\e,\f,\n,\},\],\$,\r,\t,\v}whee",
    &[
        "hello\\whee",
        "hello\x08whee",
        "hello\x1bwhee",
        "hello\x0cwhee",
        "hello\nwhee",
        "hello\rwhee",
        "hello\twhee",
        "hello\x0bwhee",
        "hello}whee",
        "hello]whee",
        "hello$whee",
    ],
    &["hello", "hellowhee", "hello whee", "hello?whee"],
)]
fn test_pattern(
    #[case] test_name: &str,
    #[case] pattern: &str,
    #[case] expect_ok: &[&str],
    #[case] expect_fail: &[&str],
) -> Result<(), FError> {
    let re = fglob::glob_to_regex(pattern)?;
    println!("{}: {} -> {}", test_name, pattern, re);

    for item in expect_ok {
        println!("- {} should match", item);
        assert!(re.is_match(item));
    }

    for item in expect_fail {
        println!("- {} should not match", item);
        assert!(!re.is_match(item));
    }

    Ok(())
}
