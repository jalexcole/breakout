extern crate regex;

#[macro_use]
extern crate lazy_static;

use regex::{Regex, RegexBuilder};

lazy_static! {
    static ref ILLEGAL_RE: Regex = Regex::new(r#"[/\?<>\\:\*\|":]"#).unwrap();
    static ref CONTROL_RE: Regex = Regex::new(r#"[\x00-\x1f\x80-\x9f]"#).unwrap();
    static ref RESERVED_RE: Regex = Regex::new(r#"^\.+$"#).unwrap();
    static ref WINDOWS_RESERVED_RE: Regex = RegexBuilder::new(r#"(?i)^(con|prn|aux|nul|com[0-9]|lpt[0-9])(\..*)?$"#)
        .case_insensitive(true)
        .build()
        .unwrap();
    static ref WINDOWS_TRAILING_RE: Regex = Regex::new(r#"[\. ]+$"#).unwrap();
}

#[derive(Clone)]
pub struct Options<'a> {
    pub windows: bool,
    pub truncate: bool,
    pub replacement: &'a str
}

impl<'a> Default for Options<'a> {
    fn default() -> Self {
        Options {
            windows: cfg!(windows),
            truncate: true,
            replacement: ""
        }
    }
}

pub fn sanitize<S: AsRef<str>>(name: S) -> String {
    sanitize_with_options(name, Options::default())
}

pub fn sanitize_with_options<S: AsRef<str>>(name: S, options: Options) -> String {

    let Options { windows, truncate, replacement } = options;
    let name = name.as_ref();
    
    let name = ILLEGAL_RE.replace_all(&name, replacement);
    let name = CONTROL_RE.replace_all(&name, replacement);
    let name = RESERVED_RE.replace(&name, replacement);
    
    let collect = |name: ::std::borrow::Cow<str>| {
        if truncate && name.len() > 255 {
            let mut end = 255;
            loop {
                if name.is_char_boundary(end) { break; }
                end -= 1;
            }
            String::from(&name[..end])
        } else {
            String::from(name)
        }
    };

    if windows {
        let name = WINDOWS_RESERVED_RE.replace(&name, replacement);
        let name = WINDOWS_TRAILING_RE.replace(&name, replacement);
        collect(name)
    } else {
        collect(name)
    }
    
}

#[derive(Clone)]
pub struct OptionsForCheck {
    pub windows: bool,
    pub truncate: bool,
}

impl Default for OptionsForCheck {
    fn default() -> Self {
        OptionsForCheck {
            windows: cfg!(windows),
            truncate: true,
        }
    }
}

pub fn is_sanitized<S: AsRef<str>>(name: S) -> bool {
    is_sanitized_with_options(name, OptionsForCheck::default())
}

pub fn is_sanitized_with_options<S: AsRef<str>>(name: S, options: OptionsForCheck) -> bool {

    let OptionsForCheck { windows, truncate } = options;
    let name = name.as_ref();
    
    if ILLEGAL_RE.is_match(&name) {
        return false;
    }
    if CONTROL_RE.is_match(&name) {
        return false;
    }
    if RESERVED_RE.is_match(&name) {
        return false;
    }
    if truncate && name.len() > 255 {
        return false;
    }
    if windows {
        if WINDOWS_RESERVED_RE.is_match(&name) {
            return false;
        }
        if WINDOWS_TRAILING_RE.is_match(&name) {
            return false;
        }
    }

    return true;

}

#[cfg(test)]
mod tests {


    // From https://github.com/parshap/node-sanitize-filename/blob/master/test.js
    static NAMES: &'static [&'static str] = &[
        "the quick brown fox jumped over the lazy dog",
        "résumé",
        "hello\u{0000}world",
        "hello\nworld",
        "semi;colon.js",
        ";leading-semi.js",
        "slash\\.js",
        "slash/.js",
        "col:on.js",
        "star*.js",
        "question?.js",
        "quote\".js",
        "singlequote'.js",
        "brack<e>ts.js",
        "p|pes.js",
        "plus+.js",
        "'five and six<seven'.js",
        " space at front",
        "space at end ",
        ".period",
        "period.",
        "relative/path/to/some/dir",
        "/abs/path/to/some/dir",
        "~/.\u{0000}notssh/authorized_keys",
        "",
        "h?w",
        "h/w",
        "h*w",
        ".",
        "..",
        "./",
        "../",
        "/..",
        "/../",
        "*.|.",
        "./",
        "./foobar",
        "../foobar",
        "../../foobar",
        "./././foobar",
        "|*.what",
        "LPT9.asdf",
        "foobar..."
    ];

    static NAMES_CLEANED: &'static [&'static str] = &[
        "the quick brown fox jumped over the lazy dog",
        "résumé",
        "helloworld",
        "helloworld",
        "semi;colon.js",
        ";leading-semi.js",
        "slash.js",
        "slash.js",
        "colon.js",
        "star.js",
        "question.js",
        "quote.js",
        "singlequote'.js",
        "brackets.js",
        "ppes.js",
        "plus+.js",
        "'five and sixseven'.js",
        " space at front",
        "space at end",
        ".period",
        "period",
        "relativepathtosomedir",
        "abspathtosomedir",
        "~.notsshauthorized_keys",
        "",
        "hw",
        "hw",
        "hw",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        ".foobar",
        "..foobar",
        "....foobar",
        "...foobar",
        ".what",
        "",
        "foobar"
    ];

    static NAMES_IS_SANITIZED: &'static [bool] = &[
        true,
        true,
        false,
        false,
        true,
        true,
        false,
        false,
        false,
        false,
        false,
        false,
        true,
        false,
        false,
        true,
        false,
        true,
        false,
        true,
        false,
        false,
        false,
        false,
        true,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        false
    ];

    #[test]
    fn it_works() {
        // sanitize
        let options = super::Options {
            windows: true,
            truncate: true,
            replacement: ""
        };

        for (idx, name) in NAMES.iter().enumerate() {
            assert_eq!(super::sanitize_with_options(name, options.clone()), NAMES_CLEANED[idx]);
        }

        let long = ::std::iter::repeat('a').take(300).collect::<String>();
        let shorter = ::std::iter::repeat('a').take(255).collect::<String>();
        assert_eq!(super::sanitize_with_options(long, options.clone()), shorter);

        // is_sanitized
        let options = super::OptionsForCheck {
            windows: true,
            truncate: true,
        };

        for (idx, name) in NAMES.iter().enumerate() {
            assert_eq!(super::is_sanitized_with_options(name, options.clone()), NAMES_IS_SANITIZED[idx]);
        }

        let long = ::std::iter::repeat('a').take(300).collect::<String>();
        assert_eq!(super::is_sanitized_with_options(long, options.clone()), false);
    }
}