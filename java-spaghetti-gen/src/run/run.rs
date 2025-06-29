use std::error::Error;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use crate::config::runtime::Config;
use crate::parser_util::JavaClass;
use crate::{emit_rust, util};

/// The core function of this library: Generate Rust code to access Java APIs.
pub fn run(config: impl Into<Config>) -> Result<(), Box<dyn Error>> {
    let config: Config = config.into();
    println!("output: {}", config.output_path.display());

    let mut context = emit_rust::Context::new(&config);
    for file in config.input_files.iter() {
        gather_file(&mut context, file)?;
    }

    {
        let mut out = Vec::with_capacity(4096);
        context.write(&mut out)?;
        util::write_generated(&context, &config.output_path, &out[..])?;
    }

    Ok(())
}

fn gather_file(context: &mut emit_rust::Context, path: &Path) -> Result<(), Box<dyn Error>> {
    let verbose = context.config.logging_verbose;

    context
        .progress
        .lock()
        .unwrap()
        .update(format!("reading {}...", path.display()).as_str());

    let ext = if let Some(ext) = path.extension() {
        ext
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Input files must have an extension",
        ))?;
    };

    match ext.to_string_lossy().to_ascii_lowercase().as_str() {
        "class" => {
            let class = JavaClass::read(std::fs::read(path)?)?;
            context.add_class(class)?;
        }
        "jar" => {
            let mut jar = zip::ZipArchive::new(io::BufReader::new(File::open(path)?))?;
            let n = jar.len();

            for i in 0..n {
                let mut file = jar.by_index(i)?;
                if !file.name().ends_with(".class") {
                    continue;
                }

                if verbose {
                    context
                        .progress
                        .lock()
                        .unwrap()
                        .update(format!("  reading {:3}/{}: {}...", i, n, file.name()).as_str());
                }

                let mut buf = Vec::new();
                file.read_to_end(&mut buf)?;
                let class = JavaClass::read(buf)?;
                context.add_class(class)?;
            }
        }
        unknown => {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Input files must have a '.class' or '.jar' extension, not a '.{unknown}' extension",),
            ))?;
        }
    }
    Ok(())
}
