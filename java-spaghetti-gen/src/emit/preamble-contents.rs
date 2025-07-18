#![allow(unused_imports)]
#![allow(non_camel_case_types)] // We map Java inner classes to Outer_Inner
#![allow(dead_code)] // We generate structs for private Java types too, just in case.
#![allow(deprecated)] // We're generating deprecated types/methods
#![allow(non_upper_case_globals)] // We might be generating Java style fields/methods
#![allow(non_snake_case)] // We might be generating Java style fields/methods
#![allow(clippy::all)] // we don't ensure generated bindings are clippy-compliant at all.
#![allow(unsafe_code)] // play nice if user has `deny(unsafe_code)` in their crate.

mod util {
    use std::char::DecodeUtf16Error;
    use std::fmt;

    use java_spaghetti::sys::jsize;
    use java_spaghetti::{Env, JavaDebug, Local, Ref, StringChars, ThrowableType};

    use super::java::lang::{String as JString, Throwable};

    impl JavaDebug for Throwable {
        fn fmt(self: &Ref<'_, Self>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            writeln!(f, "java::lang::Throwable")?;

            match self.getMessage() {
                Ok(Some(message)) => writeln!(f, "    getMessage:            {:?}", message)?,
                Ok(None) => writeln!(f, "    getMessage:            N/A (returned null)")?,
                Err(_) => writeln!(f, "    getMessage:            N/A (threw an exception!)")?,
            }

            match self.getLocalizedMessage() {
                Ok(Some(message)) => writeln!(f, "    getLocalizedMessage:   {:?}", message)?,
                Ok(None) => writeln!(f, "    getLocalizedMessage:   N/A (returned null)")?,
                Err(_) => writeln!(f, "    getLocalizedMessage:   N/A (threw an exception!)")?,
            }

            match self.getStackTrace() {
                Err(_) => writeln!(f, "    getStackTrace:         N/A (threw an exception!)")?,
                Ok(None) => writeln!(f, "    getStackTrace:         N/A (returned null)")?,
                Ok(Some(stack_trace)) => {
                    writeln!(f, "    getStackTrace:")?;
                    for frame in stack_trace.iter() {
                        match frame {
                            None => writeln!(f, "        N/A (frame was null)")?,
                            Some(frame) => {
                                let file_line = match (frame.getFileName(), frame.getLineNumber()) {
                                    (Ok(Some(file)), Ok(line)) => {
                                        format!("{}({}):", file.to_string_lossy(), line)
                                    }
                                    (Ok(Some(file)), _) => format!("{}:", file.to_string_lossy()),
                                    (_, _) => "N/A (getFileName threw an exception or returned null)".to_owned(),
                                };

                                let class_method = match (frame.getClassName(), frame.getMethodName()) {
                                    (Ok(Some(class)), Ok(Some(method))) => {
                                        format!("{}.{}", class.to_string_lossy(), method.to_string_lossy())
                                    }
                                    (Ok(Some(class)), _) => class.to_string_lossy(),
                                    (_, Ok(Some(method))) => method.to_string_lossy(),
                                    (_, _) => "N/A (getClassName + getMethodName threw exceptions or returned null)"
                                        .to_owned(),
                                };

                                writeln!(f, "        {:120}{}", file_line, class_method)?;
                            }
                        }
                    }
                }
            }

            // Consider also dumping:
            // API level 1+:
            //      getCause()
            // API level 19+:
            //      getSuppressed()

            Ok(())
        }
    }

    impl JString {
        /// Create new local string from an Env + AsRef<str>
        pub fn from_env_str<'env, S: AsRef<str>>(env: Env<'env>, string: S) -> Local<'env, Self> {
            let chars = string.as_ref().encode_utf16().collect::<Vec<_>>();

            let string = unsafe { env.new_string(chars.as_ptr(), chars.len() as jsize) };
            unsafe { Local::from_raw(env, string) }
        }

        fn string_chars<'env>(self: &Ref<'env, Self>) -> StringChars<'env> {
            unsafe { StringChars::from_env_jstring(self.env(), self.as_raw()) }
        }

        /// Returns a new [Ok]\([String]\), or an [Err]\([DecodeUtf16Error]\) if if it contained any invalid UTF16.
        ///
        /// [Ok]:                       https://doc.rust-lang.org/std/result/enum.Result.html#variant.Ok
        /// [Err]:                      https://doc.rust-lang.org/std/result/enum.Result.html#variant.Err
        /// [DecodeUtf16Error]:         https://doc.rust-lang.org/std/char/struct.DecodeUtf16Error.html
        /// [String]:                   https://doc.rust-lang.org/std/string/struct.String.html
        /// [REPLACEMENT_CHARACTER]:    https://doc.rust-lang.org/std/char/constant.REPLACEMENT_CHARACTER.html
        pub fn to_string(self: &Ref<'_, Self>) -> Result<String, DecodeUtf16Error> {
            self.string_chars().to_string()
        }

        /// Returns a new [String] with any invalid UTF16 characters replaced with [REPLACEMENT_CHARACTER]s (`'\u{FFFD}'`.)
        ///
        /// [String]:                   https://doc.rust-lang.org/std/string/struct.String.html
        /// [REPLACEMENT_CHARACTER]:    https://doc.rust-lang.org/std/char/constant.REPLACEMENT_CHARACTER.html
        pub fn to_string_lossy(self: &Ref<'_, Self>) -> String {
            self.string_chars().to_string_lossy()
        }
    }

    // OsString doesn't implement Display, so neither does java::lang::String.
    impl JavaDebug for JString {
        fn fmt(self: &Ref<'_, Self>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Debug::fmt(&self.to_string_lossy(), f) // XXX: Unneccessary alloc?  Shouldn't use lossy here?
        }
    }

    impl ThrowableType for Throwable {}
}
