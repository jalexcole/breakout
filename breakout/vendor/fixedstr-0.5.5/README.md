Library for several alternative string types using const generics.
Most types can be copied and stack-allocated.  Certain types such as
`zstr<8>` and `str8` are smaller in size than a &str on typical
systems.

#### IMPORTANT CHANGES SINCE [Version 0.5.1](https://docs.rs/fixedstr/0.5.1/fixedstr/)

**The *no-alloc* build option has been added.**  In addition to no_std, this feature
will disable compilation of any features that require the alloc crate, in particular
alloc::string::String.

#### IMPORTANT CHANGES SINCE [Version 0.5.0](https://docs.rs/fixedstr/0.5.0/fixedstr/)

**The default availability of some features have changed.** The crate
is now `#![no_std]` by default.  The `std` option, which enables the
`fstr` type, is no longer enabled by default.  The `Flexstr` and
`Sharedstr` types are also no longer enabled by default.  However,
unless you require one of these three types, your build configuration
should most likely work as before.  If you already use
`default-features=false`, your configuration should also work as
before.  See the documentation for details and examples of how to
configure your build.

<br>

Recent enhancements also include const constructors and other functions.

#### Examples
```
  let a:str8 = str8::from("abcdef"); //a str8 can hold up to 7 bytes
  let a2 = a;  // copied, not moved
  let ab = a.substr(1,5);  // copies substring to new string
  assert_eq!(ab, "bcde");  // compare for equality with &str, derefs to &str
  assert!(a<ab); // and Ord, Hash, Debug, Eq, other common traits
  let astr:&str = a.to_str(); // convert to &str (zero copy)
  let aowned:String = a.to_string(); // convert to owned string
  let afstr:fstr<8> = fstr::from(a); // fstr is another fixedstr crate type
  let azstr:zstr<16> = zstr::from(a); // so is zstr
  let a32:str32 = a.resize(); // same kind of string but with 31-byte capacity  
  let mut u = str8::from("aλb"); //unicode support
  assert_eq!(u.nth(1), Some('λ'));  // get nth character
  assert_eq!(u.nth_ascii(3), 'b');  // get nth byte as ascii character
  assert!(u.set(1,'μ'));  // changes a character of the same character class
  assert!(!u.set(1,'c')); // .set returns false on failure
  assert!(u.set(2,'c'));
  assert_eq!(u, "aμc");
  assert_eq!(u.len(),4);  // length in bytes
  assert_eq!(u.charlen(),3);  // length in chars
  let mut ac:str16 = a.reallocate().unwrap(); //copies to larger capacity type
  let remainder = ac.push("ghijklmnopq"); //append up to capacity, returns remainder
  assert_eq!(ac.len(),15);
  assert_eq!(remainder, "pq");
  ac.truncate(9);  // keep first 9 chars
  assert_eq!(&ac,"abcdefghi");
  let (upper,lower) = (str8::make("ABC"), str8::make("abc"));
  assert_eq!(upper, lower.to_ascii_upper()); // no owned String needed

  let c1 = str8::from("abcd"); // string concatenation with + for strN types  
  let c2 = str8::from("xyz");
  let mut c3 = c1 + c2 + "123";           
  assert_eq!(c3,"abcdxyz123");
  assert_eq!(c3.capacity(),15);  // type of c3 is resized to str16
  c3 = "00" + c3;                // concat &str left or right
  assert_eq!(c3,"00abcdxyz123");

  let c4 = str_format!(str16,"abc {}{}{}",1,2,3); // impls core::fmt::Write
  assert_eq!(c4,"abc 123");  //str_format! truncates if capacity exceeded
  let c5 = try_format!(str8,"abcdef{}","ghijklmn");
  assert!(c5.is_none());  // try_format! returns None if capacity exceeded

  const C:str8 = str8::const_make("abcd");    // const constructor
  let xarray = [0u8;C.len()];                 // const length function
```

Consult the [documentation](https://docs.rs/fixedstr/latest/fixedstr/) for details.
