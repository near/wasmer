//! Define the `Resolver` trait, allowing custom resolution for external
//! references.

use crate::{Engine, Export, ExportFunctionMetadata, ImportError, LinkError};
use more_asserts::assert_ge;
use wasmer_types::entity::{BoxedSlice, EntityRef, PrimaryMap};
use wasmer_types::{EntityCounts, ExternType, FunctionIndex, MemoryType, TableType};

use wasmer_vm::{
    FunctionBodyPtr, ImportFunctionEnv, Imports, MemoryStyle, VMFunctionBody,
    VMFunctionEnvironment, VMFunctionImport, VMFunctionKind, VMGlobalImport, VMImport,
    VMImportType, VMMemoryImport, VMTableImport,
};

/// Import resolver connects imports with available exported values.
pub trait Resolver {
    /// Resolves an import a WebAssembly module to an export it's hooked up to.
    ///
    /// The `index` provided is the index of the import in the wasm module
    /// that's being resolved. For example 1 means that it's the second import
    /// listed in the wasm module.
    ///
    /// The `module` and `field` arguments provided are the module/field names
    /// listed on the import itself.
    ///
    /// # Notes:
    ///
    /// The index is useful because some WebAssembly modules may rely on that
    /// for resolving ambiguity in their imports. Such as:
    /// ```ignore
    /// (module
    ///   (import "" "" (func))
    ///   (import "" "" (func (param i32) (result i32)))
    /// )
    /// ```
    fn resolve(&self, _index: u32, module: &str, field: &str) -> Option<Export>;
}

/// Import resolver connects imports with available exported values.
///
/// This is a specific subtrait for [`Resolver`] for those users who don't
/// care about the `index`, but only about the `module` and `field` for
/// the resolution.
pub trait NamedResolver {
    /// Resolves an import a WebAssembly module to an export it's hooked up to.
    ///
    /// It receives the `module` and `field` names and return the [`Export`] in
    /// case it's found.
    fn resolve_by_name(&self, module: &str, field: &str) -> Option<Export>;
}

// All NamedResolvers should extend `Resolver`.
impl<T: NamedResolver> Resolver for T {
    /// By default this method will be calling [`NamedResolver::resolve_by_name`],
    /// dismissing the provided `index`.
    fn resolve(&self, _index: u32, module: &str, field: &str) -> Option<Export> {
        self.resolve_by_name(module, field)
    }
}

impl<T: NamedResolver> NamedResolver for &T {
    fn resolve_by_name(&self, module: &str, field: &str) -> Option<Export> {
        (**self).resolve_by_name(module, field)
    }
}

impl NamedResolver for Box<dyn NamedResolver + Send + Sync> {
    fn resolve_by_name(&self, module: &str, field: &str) -> Option<Export> {
        (**self).resolve_by_name(module, field)
    }
}

impl NamedResolver for () {
    /// Always returns `None`.
    fn resolve_by_name(&self, _module: &str, _field: &str) -> Option<Export> {
        None
    }
}

/// `Resolver` implementation that always resolves to `None`. Equivalent to `()`.
pub struct NullResolver {}

impl Resolver for NullResolver {
    fn resolve(&self, _idx: u32, _module: &str, _field: &str) -> Option<Export> {
        None
    }
}

fn is_compatible_table(ex: &TableType, im: &TableType) -> bool {
    (ex.ty == wasmer_types::Type::FuncRef || ex.ty == im.ty)
        && im.minimum <= ex.minimum
        && (im.maximum.is_none()
            || (!ex.maximum.is_none() && im.maximum.unwrap() >= ex.maximum.unwrap()))
}

fn is_compatible_memory(ex: &MemoryType, im: &MemoryType) -> bool {
    im.minimum <= ex.minimum
        && (im.maximum.is_none()
            || (!ex.maximum.is_none() && im.maximum.unwrap() >= ex.maximum.unwrap()))
        && ex.shared == im.shared
}

/// This function allows to match all imports of a `ModuleInfo` with concrete definitions provided by
/// a `Resolver`.
///
/// If all imports are satisfied returns an `Imports` instance required for a module instantiation.
pub fn resolve_imports(
    engine: &dyn Engine,
    resolver: &dyn Resolver,
    import_counts: &EntityCounts,
    imports: &[VMImport],
    finished_dynamic_function_trampolines: &BoxedSlice<FunctionIndex, FunctionBodyPtr>,
) -> Result<Imports, LinkError> {
    let mut function_imports = PrimaryMap::with_capacity(import_counts.functions);
    let mut host_function_env_initializers = PrimaryMap::with_capacity(import_counts.functions);
    let mut table_imports = PrimaryMap::with_capacity(import_counts.tables);
    let mut memory_imports = PrimaryMap::with_capacity(import_counts.memories);
    let mut global_imports = PrimaryMap::with_capacity(import_counts.globals);
    for VMImport {
        import_no,
        module,
        field,
        ty,
    } in imports
    {
        let resolved = resolver.resolve(*import_no, module, field);
        let resolved = match resolved {
            Some(r) => r,
            None => {
                return Err(LinkError::Import(
                    module.to_string(),
                    field.to_string(),
                    // TODO(0-copy): convert `VMImportType` to a nice type for presentation here.
                    ImportError::UnknownImport(None.unwrap()),
                ));
            }
        };
        let export_extern = || match resolved {
            Export::Function(ref f) => ExternType::Function(
                engine
                    .lookup_signature(f.vm_function.signature)
                    .expect("VMSharedSignatureIndex not registered with engine (wrong engine?)")
                    .clone(),
            ),
            Export::Table(ref t) => ExternType::Table(*t.ty()),
            Export::Memory(ref m) => ExternType::Memory(m.ty()),
            Export::Global(ref g) => {
                let global = g.from.ty();
                ExternType::Global(*global)
            }
        };
        match (&resolved, ty) {
            (Export::Function(ex), VMImportType::Function(im))
                if ex.vm_function.signature == *im =>
            {
                let address = match ex.vm_function.kind {
                    VMFunctionKind::Dynamic => {
                        // If this is a dynamic imported function,
                        // the address of the function is the address of the
                        // reverse trampoline.
                        let index = FunctionIndex::new(function_imports.len());
                        finished_dynamic_function_trampolines[index].0 as *mut VMFunctionBody as _

                        // TODO: We should check that the f.vmctx actually matches
                        // the shape of `VMDynamicFunctionImportContext`
                    }
                    VMFunctionKind::Static => ex.vm_function.address,
                };

                // Clone the host env for this `Instance`.
                let env = if let Some(ExportFunctionMetadata {
                    host_env_clone_fn: clone,
                    ..
                }) = ex.metadata.as_deref()
                {
                    // TODO: maybe start adding asserts in all these
                    // unsafe blocks to prevent future changes from
                    // horribly breaking things.
                    unsafe {
                        assert!(!ex.vm_function.vmctx.host_env.is_null());
                        (clone)(ex.vm_function.vmctx.host_env)
                    }
                } else {
                    // No `clone` function means we're dealing with some
                    // other kind of `vmctx`, not a host env of any
                    // kind.
                    unsafe { ex.vm_function.vmctx.host_env }
                };

                function_imports.push(VMFunctionImport {
                    body: FunctionBodyPtr(address),
                    signature: *im,
                    environment: VMFunctionEnvironment { host_env: env },
                });

                let initializer = ex
                    .metadata
                    .as_ref()
                    .and_then(|m| m.import_init_function_ptr);
                let clone = ex.metadata.as_ref().map(|m| m.host_env_clone_fn);
                let destructor = ex.metadata.as_ref().map(|m| m.host_env_drop_fn);
                let import_function_env =
                    if let (Some(clone), Some(destructor)) = (clone, destructor) {
                        ImportFunctionEnv::Env {
                            env,
                            clone,
                            initializer,
                            destructor,
                        }
                    } else {
                        ImportFunctionEnv::NoEnv
                    };

                host_function_env_initializers.push(import_function_env);
            }
            (Export::Table(ex), VMImportType::Table(im)) if is_compatible_table(ex.ty(), im) => {
                let import_table_ty = ex.from.ty();
                if import_table_ty.ty != im.ty {
                    return Err(LinkError::Import(
                        module.to_string(),
                        field.to_string(),
                        // TODO(0-copy): nice presentation of the error here.
                        ImportError::IncompatibleType(None.unwrap(), export_extern()),
                    ));
                }
                table_imports.push(VMTableImport {
                    definition: ex.from.vmtable(),
                    from: ex.from.clone(),
                });
            }
            (Export::Memory(ex), VMImportType::Memory(im, import_memory_style))
                if is_compatible_memory(&ex.ty(), im) =>
            {
                // Sanity-check: Ensure that the imported memory has at least
                // guard-page protections the importing module expects it to have.
                let export_memory_style = ex.style();
                if let (
                    MemoryStyle::Static { bound, .. },
                    MemoryStyle::Static {
                        bound: import_bound,
                        ..
                    },
                ) = (export_memory_style.clone(), &import_memory_style)
                {
                    assert_ge!(bound, *import_bound);
                }
                assert_ge!(
                    export_memory_style.offset_guard_size(),
                    import_memory_style.offset_guard_size()
                );
                memory_imports.push(VMMemoryImport {
                    definition: ex.from.vmmemory(),
                    from: ex.from.clone(),
                });
            }

            (Export::Global(ex), VMImportType::Global(im)) if ex.from.ty() == im => {
                global_imports.push(VMGlobalImport {
                    definition: ex.from.vmglobal(),
                    from: ex.from.clone(),
                });
            }
            _ => {
                return Err(LinkError::Import(
                    module.to_string(),
                    field.to_string(),
                    // TODO(0-copy): convert types to a nice presentation here.
                    ImportError::IncompatibleType(None.unwrap(), export_extern()),
                ));
            }
        }
    }
    Ok(Imports::new(
        function_imports,
        host_function_env_initializers,
        table_imports,
        memory_imports,
        global_imports,
    ))
}

/// A [`Resolver`] that links two resolvers together in a chain.
pub struct NamedResolverChain<A: NamedResolver + Send + Sync, B: NamedResolver + Send + Sync> {
    a: A,
    b: B,
}

/// A trait for chaining resolvers together.
///
/// ```
/// # use wasmer_engine::{ChainableNamedResolver, NamedResolver};
/// # fn chainable_test<A, B>(imports1: A, imports2: B)
/// # where A: NamedResolver + Sized + Send + Sync,
/// #       B: NamedResolver + Sized + Send + Sync,
/// # {
/// // override duplicates with imports from `imports2`
/// imports1.chain_front(imports2);
/// # }
/// ```
pub trait ChainableNamedResolver: NamedResolver + Sized + Send + Sync {
    /// Chain a resolver in front of the current resolver.
    ///
    /// This will cause the second resolver to override the first.
    ///
    /// ```
    /// # use wasmer_engine::{ChainableNamedResolver, NamedResolver};
    /// # fn chainable_test<A, B>(imports1: A, imports2: B)
    /// # where A: NamedResolver + Sized + Send + Sync,
    /// #       B: NamedResolver + Sized + Send + Sync,
    /// # {
    /// // override duplicates with imports from `imports2`
    /// imports1.chain_front(imports2);
    /// # }
    /// ```
    fn chain_front<U>(self, other: U) -> NamedResolverChain<U, Self>
    where
        U: NamedResolver + Send + Sync,
    {
        NamedResolverChain { a: other, b: self }
    }

    /// Chain a resolver behind the current resolver.
    ///
    /// This will cause the first resolver to override the second.
    ///
    /// ```
    /// # use wasmer_engine::{ChainableNamedResolver, NamedResolver};
    /// # fn chainable_test<A, B>(imports1: A, imports2: B)
    /// # where A: NamedResolver + Sized + Send + Sync,
    /// #       B: NamedResolver + Sized + Send + Sync,
    /// # {
    /// // override duplicates with imports from `imports1`
    /// imports1.chain_back(imports2);
    /// # }
    /// ```
    fn chain_back<U>(self, other: U) -> NamedResolverChain<Self, U>
    where
        U: NamedResolver + Send + Sync,
    {
        NamedResolverChain { a: self, b: other }
    }
}

// We give these chain methods to all types implementing NamedResolver
impl<T: NamedResolver + Send + Sync> ChainableNamedResolver for T {}

impl<A, B> NamedResolver for NamedResolverChain<A, B>
where
    A: NamedResolver + Send + Sync,
    B: NamedResolver + Send + Sync,
{
    fn resolve_by_name(&self, module: &str, field: &str) -> Option<Export> {
        self.a
            .resolve_by_name(module, field)
            .or_else(|| self.b.resolve_by_name(module, field))
    }
}

impl<A, B> Clone for NamedResolverChain<A, B>
where
    A: NamedResolver + Clone + Send + Sync,
    B: NamedResolver + Clone + Send + Sync,
{
    fn clone(&self) -> Self {
        Self {
            a: self.a.clone(),
            b: self.b.clone(),
        }
    }
}
