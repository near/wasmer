use crate::common::get_cache_dir;
#[cfg(feature = "debug")]
use crate::logging;
use crate::store::{CompilerType, EngineType, StoreOptions};
use crate::suggestions::suggest_function_exports;
use crate::warning;
use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use std::str::FromStr;
use wasmer::*;

use structopt::StructOpt;

#[derive(Debug, StructOpt, Clone, Default)]
/// The options for the `wasmer run` subcommand
pub struct Run {
    /// File to run
    #[structopt(name = "FILE", parse(from_os_str))]
    path: PathBuf,

    /// Invoke a specified function
    #[structopt(long = "invoke", short = "i")]
    invoke: Option<String>,

    /// The command name is a string that will override the first argument passed
    /// to the wasm program. This is used in wapm to provide nicer output in
    /// help commands and error messages of the running wasm program
    #[structopt(long = "command-name", hidden = true)]
    command_name: Option<String>,

    #[structopt(flatten)]
    store: StoreOptions,

    /// Enable debug output
    #[cfg(feature = "debug")]
    #[structopt(long = "debug", short = "d")]
    debug: bool,

    #[cfg(feature = "debug")]
    #[structopt(short, long, parse(from_occurrences))]
    verbose: u8,

    /// Application arguments
    #[structopt(value_name = "ARGS")]
    args: Vec<String>,
}

impl Run {
    /// Execute the run command
    pub fn execute(&self) -> Result<()> {
        #[cfg(feature = "debug")]
        if self.debug {
            logging::set_up_logging(self.verbose).unwrap();
        }
        self.inner_execute().with_context(|| {
            format!(
                "failed to run `{}`{}",
                self.path.display(),
                if CompilerType::enabled().is_empty() {
                    " (no compilers enabled)"
                } else {
                    ""
                }
            )
        })
    }

    fn inner_execute(&self) -> Result<()> {
        let module = self.get_module()?;
        let instance = Instance::new(&module, &imports! {})?;

        // If this module exports an _initialize function, run that first.
        if let Ok(initialize) = instance.exports.get_function("_initialize") {
            initialize
                .call(&[])
                .with_context(|| "failed to run _initialize function")?;
        }

        // Do we want to invoke a function?
        if let Some(ref invoke) = self.invoke {
            let imports = imports! {};
            let instance = Instance::new(&module, &imports)?;
            let result = self.invoke_function(&instance, &invoke, &self.args)?;
            println!(
                "{}",
                result
                    .iter()
                    .map(|val| val.to_string())
                    .collect::<Vec<String>>()
                    .join(" ")
            );
        } else {
            let start: Function = self.try_find_function(&instance, "_start", &[])?;
            let result = start.call(&[]);
            result?;
        }

        Ok(())
    }

    fn get_module(&self) -> Result<Module> {
        let contents = std::fs::read(self.path.clone())?;
        #[cfg(feature = "universal")]
        {
            use wasmer_engine_universal::{UniversalExecutable, UniversalArtifact, Universal};

            if UniversalExecutable::verify_serialized(&contents) {
                unsafe {
                    let executable = UniversalExecutable::archive_from_slice(&contents)?;
                    let engine = wasmer_engine_universal::Universal::headless().engine();
                    let artifact = engine.load(&executable);
                    let store = Store::new(&engine);
                    let module = unsafe { Module::deserialize_from_file(&store, &self.path)? };
                }
                return Ok(module);
            }
        }
        let (store, engine_type, compiler_type) = self.store.get_store()?;
        let module_result = Module::new(&store, &contents);

        let mut module = module_result.with_context(|| {
            format!(
                "module instantiation failed (engine: {}, compiler: {})",
                engine_type.to_string(),
                compiler_type.to_string()
            )
        })?;
        // We set the name outside the cache, to make sure we dont cache the name
        module.set_name(&self.path.file_name().unwrap_or_default().to_string_lossy());

        Ok(module)
    }

    #[cfg(feature = "cache")]
    fn get_module_from_cache(
        &self,
        store: &Store,
        contents: &[u8],
        engine_type: &EngineType,
        compiler_type: &CompilerType,
    ) -> Result<Module> {
        // We try to get it from cache, in case caching is enabled
        // and the file length is greater than 4KB.
        // For files smaller than 4KB caching is not worth,
        // as it takes space and the speedup is minimal.
        let mut cache = self.get_cache(engine_type, compiler_type)?;
        // Try to get the hash from the provided `--cache-key`, otherwise
        // generate one from the provided file `.wasm` contents.
        let hash = self
            .cache_key
            .as_ref()
            .and_then(|key| Hash::from_str(&key).ok())
            .unwrap_or_else(|| Hash::generate(&contents));
        match unsafe { cache.load(&store, hash) } {
            Ok(module) => Ok(module),
            Err(e) => {
                match e {
                    DeserializeError::Io(_) => {
                        // Do not notify on IO errors
                    }
                    err => {
                        warning!("cached module is corrupted: {}", err);
                    }
                }
                let module = Module::new(&store, &contents)?;
                // Store the compiled Module in cache
                cache.store(hash, &module)?;
                Ok(module)
            }
        }
    }

    #[cfg(feature = "cache")]
    /// Get the Compiler Filesystem cache
    fn get_cache(
        &self,
        engine_type: &EngineType,
        compiler_type: &CompilerType,
    ) -> Result<FileSystemCache> {
        let mut cache_dir_root = get_cache_dir();
        cache_dir_root.push(compiler_type.to_string());
        let mut cache = FileSystemCache::new(cache_dir_root)?;

        // Important: Dylib files need to have a `.dll` extension on
        // Windows, otherwise they will not load, so we just add an
        // extension always to make it easier to recognize as well.
        #[allow(unreachable_patterns)]
        let extension = match *engine_type {
            #[cfg(feature = "dylib")]
            EngineType::Dylib => {
                wasmer_engine_dylib::DylibArtifact::get_default_extension(&Triple::host())
                    .to_string()
            }
            #[cfg(feature = "universal")]
            EngineType::Universal => {
                wasmer_engine_universal::UniversalArtifact::get_default_extension(&Triple::host())
                    .to_string()
            }
            // We use the compiler type as the default extension
            _ => compiler_type.to_string(),
        };
        cache.set_cache_extension(Some(extension));
        Ok(cache)
    }

    fn try_find_function(
        &self,
        instance: &Instance,
        name: &str,
        args: &[String],
    ) -> Result<Function> {
        Ok(instance
            .exports
            .get_function(&name)
            .map_err(|e| {
                if instance.module().info().functions.is_empty() {
                    anyhow!("The module has no exported functions to call.")
                } else {
                    let suggested_functions = suggest_function_exports(instance.module(), "");
                    let names = suggested_functions
                        .iter()
                        .take(3)
                        .map(|arg| format!("`{}`", arg))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let suggested_command = format!(
                        "wasmer {} -i {} {}",
                        self.path.display(),
                        suggested_functions.get(0).unwrap_or(&String::new()),
                        args.join(" ")
                    );
                    let suggestion = if suggested_functions.len() == 0 {
                        String::from("Can not find any export functions.")
                    } else {
                        format!(
                            "Similar functions found: {}.\nTry with: {}",
                            names, suggested_command
                        )
                    };
                    match e {
                        ExportError::Missing(_) => {
                            anyhow!("No export `{}` found in the module.\n{}", name, suggestion)
                        }
                        ExportError::IncompatibleType => anyhow!(
                            "Export `{}` found, but is not a function.\n{}",
                            name,
                            suggestion
                        ),
                    }
                }
            })?
            .clone())
    }

    fn invoke_function(
        &self,
        instance: &Instance,
        invoke: &str,
        args: &[String],
    ) -> Result<Box<[Val]>> {
        let func: Function = self.try_find_function(&instance, invoke, args)?;
        let func_ty = func.ty();
        let required_arguments = func_ty.params().len();
        let provided_arguments = args.len();
        if required_arguments != provided_arguments {
            bail!(
                "Function expected {} arguments, but received {}: \"{}\"",
                required_arguments,
                provided_arguments,
                self.args.join(" ")
            );
        }
        let invoke_args = args
            .iter()
            .zip(func_ty.params().iter())
            .map(|(arg, param_type)| match param_type {
                ValType::I32 => {
                    Ok(Val::I32(arg.parse().map_err(|_| {
                        anyhow!("Can't convert `{}` into a i32", arg)
                    })?))
                }
                ValType::I64 => {
                    Ok(Val::I64(arg.parse().map_err(|_| {
                        anyhow!("Can't convert `{}` into a i64", arg)
                    })?))
                }
                ValType::F32 => {
                    Ok(Val::F32(arg.parse().map_err(|_| {
                        anyhow!("Can't convert `{}` into a f32", arg)
                    })?))
                }
                ValType::F64 => {
                    Ok(Val::F64(arg.parse().map_err(|_| {
                        anyhow!("Can't convert `{}` into a f64", arg)
                    })?))
                }
                _ => Err(anyhow!(
                    "Don't know how to convert {} into {:?}",
                    arg,
                    param_type
                )),
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(func.call(&invoke_args)?)
    }

    /// Create Run instance for arguments/env,
    /// assuming we're being run from a CFP binfmt interpreter.
    pub fn from_binfmt_args() -> Run {
        Self::from_binfmt_args_fallible().unwrap_or_else(|e| {
            crate::error::PrettyError::report::<()>(
                Err(e).context("Failed to set up wasmer binfmt invocation"),
            )
        })
    }

    #[cfg(target_os = "linux")]
    fn from_binfmt_args_fallible() -> Result<Run> {
        let argv = std::env::args_os().collect::<Vec<_>>();
        let (_interpreter, executable, original_executable, args) = match &argv[..] {
            [a, b, c, d @ ..] => (a, b, c, d),
            _ => {
                bail!("Wasmer binfmt interpreter needs at least three arguments (including $0) - must be registered as binfmt interpreter with the CFP flags. (Got arguments: {:?})", argv);
            }
        };
        // TODO: Optimally, args and env would be passed as an UTF-8 Vec.
        // (Can be pulled out of std::os::unix::ffi::OsStrExt)
        // But I don't want to duplicate or rewrite run.rs today.
        let args = args
            .iter()
            .enumerate()
            .map(|(i, s)| {
                s.clone().into_string().map_err(|s| {
                    anyhow!(
                        "Cannot convert argument {} ({:?}) to UTF-8 string",
                        i + 1,
                        s
                    )
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let original_executable = original_executable
            .clone()
            .into_string()
            .map_err(|s| anyhow!("Cannot convert executable name {:?} to UTF-8 string", s))?;
        let store = StoreOptions::default();
        // TODO: store.compiler.features.all = true; ?
        Ok(Self {
            args,
            path: executable.into(),
            command_name: Some(original_executable),
            store,
            ..Self::default()
        })
    }
    #[cfg(not(target_os = "linux"))]
    fn from_binfmt_args_fallible() -> Result<Run> {
        bail!("binfmt_misc is only available on linux.")
    }
}
