use crate::io_data_err;
use crate::io_data_error;

use tempfile::NamedTempFile;

use std::fs::*;
use std::io::{self, BufRead, BufReader, ErrorKind, Seek, SeekFrom, Write};
use std::path::{Path};

const MARKER_COMMENT : &'static str = "WARNING:  This file was autogenerated by jni-bindgen.  Any changes to this file may be lost!!!";

pub struct GeneratedFile<'a> {
    context:    &'a crate::emit_rust::Context<'a>,
    path:       &'a Path,
    original:   Option<BufReader<File>>,
    rewrite:    NamedTempFile,
}

pub struct Difference {
    pub line_no:    u32,
    pub original:   String,
    pub rewrite:    String,
}

impl Difference {
    pub fn original_missing() -> Self {
        Self {
            line_no:    0,
            original:   String::new(),
            rewrite:    String::new(),
        }
    }
}

impl<'a> GeneratedFile<'a> {
    pub fn new(context: &'a crate::emit_rust::Context, path: &'a impl AsRef<Path>) -> io::Result<GeneratedFile<'a>> {
        let path = path.as_ref();

        let dir = path.parent().ok_or_else(|| io_data_error!("{:?} has no parent directory", path))?;
        let _ = create_dir_all(dir);

        let original = match File::open(path) {
            Ok(file) => {
                let mut reader = BufReader::new(file);
                let mut first_line = String::new();
                read_line_no_eol(&mut reader, &mut first_line)?;

                let mut found_marker = false;
                for prefix in &["// ", "# "] {
                    if first_line.starts_with(prefix) && (&first_line[prefix.len()..] == MARKER_COMMENT) {
                        found_marker = true;
                        break;
                    }
                }

                if !found_marker {
                    return io_data_err!("Cannot overwrite {:?}:  File exists, and first line {:?} doesn't match expected MARKER_COMMENT {:?}", path, first_line, MARKER_COMMENT);
                }

                Some(reader)
            },
            Err(ref e) if e.kind() == ErrorKind::NotFound => { None }, // OK
            Err(e) => {
                return Err(e);
            },
        };

        let rewrite = tempfile::Builder::new()
            .suffix(".rs.tmp")
            .tempfile_in(dir)?;

        Ok(GeneratedFile{ context, path, original, rewrite })
    }

    /// Persist the generated file, clobbering the old version if it existed.
    pub fn clobber(mut self) -> io::Result<&'a Path> {
        let difference = self.find_difference()?;
        let Self { context, path, original, rewrite } = self;
        match difference {
            None => {
                context.progress.lock().unwrap().update(format!("unchanged: {}...", path.display()).as_str());
                return Ok(path);
            }
            Some(Difference { line_no: 0, .. }) => {
                context.progress.lock().unwrap().update(format!("NEW: {}", path.display()).as_str());
            },
            Some(_difference) => {
                context.progress.lock().unwrap().update(format!("MODIFIED: {}", path.display()).as_str());
            },
        }

        std::mem::drop(original);
        rewrite.persist(path)?;
        Ok(path)
    }

    /// **WARNING**: leaves self in an inconsistent state on Err.
    pub fn find_difference(&mut self) -> io::Result<Option<Difference>> {
        let original = if let Some(f) = self.original.as_mut() { f } else { return Ok(Some(Difference::original_missing())); }; // If there was no original file, there are no differences.
        let mut rewrite = BufReader::new(&mut self.rewrite);

        original.seek(SeekFrom::Start(0))?;
        rewrite.seek(SeekFrom::Start(0))?;

        let mut original_line = String::new();
        let mut rewrite_line = String::new();

        let mut line_no = 0;
        loop {
            line_no += 1;

            let a = read_line_no_eol(original, &mut original_line)?;
            let b = read_line_no_eol(&mut rewrite, &mut rewrite_line)?;

            if a == 0 && b ==  0 { return Ok(None); }

            if original_line != rewrite_line {
                rewrite.seek(SeekFrom::End(0))?;
                return Ok(Some(Difference { line_no, original: original_line, rewrite: rewrite_line }));
            }
        }
    }
}

impl<'a> Write for GeneratedFile<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> { self.rewrite.write(buf) }
    fn flush(&mut self) -> io::Result<()> { self.rewrite.flush() }
}

fn read_line_no_eol(reader: &mut impl BufRead, buffer: &mut String) -> io::Result<usize> {
    let size = reader.read_line(buffer)?;
    while buffer.ends_with('\r') || buffer.ends_with('\n') {
        buffer.pop();
    }
    Ok(size)
}
