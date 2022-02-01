//! Universal compilation.

use crate::executable::{unrkyv, UniversalExecutableRef};
use crate::{CodeMemory, UniversalArtifact, UniversalExecutable};
use loupe::MemoryUsage;
use rkyv::de::deserializers::SharedDeserializeMap;
use std::collections::{HashMap, BTreeMap};
use std::convert::TryFrom;
use std::sync::{Arc, Mutex};
#[cfg(feature = "compiler")]
use wasmer_compiler::Compiler;
use wasmer_compiler::{
    CompileError, CustomSectionProtection, CustomSectionRef, FunctionBodyRef, JumpTable,
    ModuleMiddlewareChain, Relocation, SectionIndex, Target, TrampolinesSection,
};
use wasmer_engine::{Artifact, Engine, EngineId, Tunables};
use wasmer_types::entity::{EntityRef, PrimaryMap};
use wasmer_types::{
    DataInitializer, EntityCounts, Features, FunctionIndex, FunctionType, FunctionTypeRef,
    GlobalType, ImportIndex, LocalFunctionIndex, MemoryIndex, MemoryType, SignatureIndex,
    TableIndex, TableType, DataIndex,
};
use wasmer_vm::{
    FuncDataRegistry, FunctionBodyPtr, MemoryStyle, SectionBodyPtr, SignatureRegistry, TableStyle,
    VMCallerCheckedAnyfunc, VMFuncRef, VMFunctionBody, VMImport, VMImportType, VMLocalFunction,
    VMOffsets, VMSharedSignatureIndex, VMTrampoline,
};

/// A WebAssembly `Universal` Engine.
#[derive(Clone, MemoryUsage)]
pub struct UniversalEngine {
    inner: Arc<Mutex<UniversalEngineInner>>,
    /// The target for the compiler
    target: Arc<Target>,
    engine_id: EngineId,
}

impl UniversalEngine {
    /// Create a new `UniversalEngine` with the given config
    #[cfg(feature = "compiler")]
    pub fn new(compiler: Box<dyn Compiler>, target: Target, features: Features) -> Self {
        Self {
            inner: Arc::new(Mutex::new(UniversalEngineInner {
                compiler: Some(compiler),
                code_memory: vec![],
                signatures: SignatureRegistry::new(),
                func_data: Arc::new(FuncDataRegistry::new()),
                features,
            })),
            target: Arc::new(target),
            engine_id: EngineId::default(),
        }
    }

    /// Create a headless `UniversalEngine`
    ///
    /// A headless engine is an engine without any compiler attached.
    /// This is useful for assuring a minimal runtime for running
    /// WebAssembly modules.
    ///
    /// For example, for running in IoT devices where compilers are very
    /// expensive, or also to optimize startup speed.
    ///
    /// # Important
    ///
    /// Headless engines can't compile or validate any modules,
    /// they just take already processed Modules (via `Module::serialize`).
    pub fn headless() -> Self {
        Self {
            inner: Arc::new(Mutex::new(UniversalEngineInner {
                #[cfg(feature = "compiler")]
                compiler: None,
                code_memory: vec![],
                signatures: SignatureRegistry::new(),
                func_data: Arc::new(FuncDataRegistry::new()),
                features: Features::default(),
            })),
            target: Arc::new(Target::default()),
            engine_id: EngineId::default(),
        }
    }

    pub(crate) fn inner(&self) -> std::sync::MutexGuard<'_, UniversalEngineInner> {
        self.inner.lock().unwrap()
    }

    pub(crate) fn inner_mut(&self) -> std::sync::MutexGuard<'_, UniversalEngineInner> {
        self.inner.lock().unwrap()
    }

    pub(crate) fn load_owned(
        &self,
        executable: &UniversalExecutable,
    ) -> Result<std::sync::Arc<dyn Artifact>, CompileError> {
        let info = &executable.compile_info;
        let module = &info.module;
        let imports = {
            let mut inner_engine = self.inner_mut();
            module
                .imports
                .iter()
                .map(|((module_name, field, idx), entity)| wasmer_vm::VMImport {
                    module: String::from(module_name),
                    field: String::from(field),
                    import_no: *idx,
                    ty: match entity {
                        ImportIndex::Function(i) => {
                            let sig_idx = module.functions[*i];
                            let sig = (&module.signatures[sig_idx]).into();
                            VMImportType::Function(inner_engine.signatures.register(sig))
                        }
                        ImportIndex::Table(i) => VMImportType::Table(module.tables[*i]),
                        &ImportIndex::Memory(i) => {
                            let ty = module.memories[i];
                            VMImportType::Memory(ty, info.memory_styles[i].clone())
                        }
                        ImportIndex::Global(i) => VMImportType::Global(module.globals[*i]),
                    },
                })
                .collect()
        };
        let local_functions = executable.function_bodies.iter().map(|(idx, b)| {
            let sig = &module.signatures[module.functions[module.func_index(idx)]];
            (sig.into(), b.into())
        });
        let local_memories = (module.import_counts.memories..module.memories.len())
            .map(|idx| {
                let idx = MemoryIndex::new(idx);
                (module.memories[idx], info.memory_styles[idx].clone())
            })
            .collect();
        let local_tables = (module.import_counts.tables..module.tables.len())
            .map(|idx| {
                let idx = TableIndex::new(idx);
                (module.tables[idx], info.table_styles[idx].clone())
            })
            .collect();
        let local_globals = module
            .globals
            .iter()
            .skip(module.import_counts.globals)
            .map(|(_, t)| *t)
            .collect();

        self.load_common(
            module.signatures.values().map(Into::into),
            local_functions,
            executable
                .function_call_trampolines
                .iter()
                .map(|(_, b)| b.into()),
            executable
                .dynamic_function_trampolines
                .iter()
                .map(|(_, b)| b.into()),
            executable.custom_sections.iter().map(|(_, s)| s.into()),
            |func_idx, jt_idx| executable.function_jt_offsets[func_idx][jt_idx],
            executable
                .function_relocations
                .iter()
                .map(|(i, rs)| (i, rs.iter().cloned())),
            executable
                .custom_section_relocations
                .iter()
                .map(|(i, rs)| (i, rs.iter().cloned())),
            &executable.trampolines,
            executable
                .debug
                .as_ref()
                .map(|d| (d.eh_frame, (&executable.custom_sections[d.eh_frame]).into())),
            executable.data_initializers.iter().map(Into::into),
            module.passive_data.clone(),
            imports,
            module.import_counts,
            VMOffsets::for_host().with_module_info(&*module),
            local_memories,
            local_tables,
            local_globals,
        )
    }

    pub(crate) fn load_archived(
        &self,
        executable: &UniversalExecutableRef,
    ) -> Result<std::sync::Arc<dyn Artifact>, CompileError> {
        let info = &executable.compile_info;
        let module = &info.module;
        let imports = {
            let mut inner_engine = self.inner_mut();
            module
                .imports
                .iter()
                .map(|((module_name, field, idx), entity)| wasmer_vm::VMImport {
                    module: String::from(module_name.as_str()),
                    field: String::from(field.as_str()),
                    import_no: *idx,
                    ty: match entity {
                        ImportIndex::Function(i) => {
                            let sig_idx = module.functions[i];
                            let sig = (&module.signatures[&sig_idx]).into();
                            VMImportType::Function(inner_engine.signatures.register(sig))
                        }
                        ImportIndex::Table(i) => VMImportType::Table(unrkyv(&module.tables[i])),
                        ImportIndex::Memory(i) => {
                            let ty = unrkyv(&module.memories[i]);
                            VMImportType::Memory(ty, unrkyv(&info.memory_styles[i]))
                        }
                        ImportIndex::Global(i) => VMImportType::Global(unrkyv(&module.globals[i])),
                    },
                })
                .collect()
        };
        let import_counts: EntityCounts = unrkyv(&module.import_counts);
        let local_functions = executable.function_bodies.iter().map(|(idx, b)| {
            let func_idx = FunctionIndex::new(import_counts.functions + idx.index());
            let sig = &module.signatures[&module.functions[&func_idx]];
            (sig.into(), b.into())
        });
        let local_memories = (import_counts.memories..module.memories.len())
            .map(|idx| {
                let idx = MemoryIndex::new(idx);
                let mty = &module.memories[&idx];
                (unrkyv(mty), unrkyv(&info.memory_styles[&idx]))
            })
            .collect();
        let local_tables = (import_counts.tables..module.tables.len())
            .map(|idx| {
                let idx = TableIndex::new(idx);
                let tty = &module.tables[&idx];
                (unrkyv(tty), unrkyv(&info.table_styles[&idx]))
            })
            .collect();
        let local_globals = module
            .globals
            .iter()
            .skip(import_counts.globals)
            .map(|(_, t)| *t)
            .collect();
        let passive_data = rkyv::Deserialize::deserialize(
            &module.passive_data,
            &mut SharedDeserializeMap::new(),
        ).map_err(|e| CompileError::Validate("could not deserialize passive data".into()))?;
        self.load_common(
            module.signatures.values().map(Into::into),
            local_functions,
            executable
                .function_call_trampolines
                .iter()
                .map(|(_, b)| b.into()),
            executable
                .dynamic_function_trampolines
                .iter()
                .map(|(_, b)| b.into()),
            executable.custom_sections.iter().map(|(_, s)| s.into()),
            |func_idx, jt_idx| {
                let func_idx = rkyv::Archived::<LocalFunctionIndex>::new(func_idx.index());
                let jt_idx = rkyv::Archived::<JumpTable>::new(jt_idx.index());
                executable.function_jt_offsets[&func_idx][&jt_idx]
            },
            executable
                .function_relocations
                .iter()
                .map(|(i, r)| (i, r.iter().map(unrkyv))),
            executable
                .custom_section_relocations
                .iter()
                .map(|(i, r)| (i, r.iter().map(unrkyv))),
            &unrkyv(&executable.trampolines),
            executable.debug.as_ref().map(|d| {
                (
                    unrkyv(&d.eh_frame),
                    (&executable.custom_sections[&d.eh_frame]).into(),
                )
            }),
            executable.data_initializers.iter().map(Into::into),
            // TODO(0-copy): the passive data could be a single heap buffer with indices into it.
            passive_data,
            imports,
            unrkyv(&module.import_counts),
            VMOffsets::for_host().with_archived_module_info(&*module),
            local_memories,
            local_tables,
            local_globals,
        )
    }

    fn load_common<'a>(
        &self,
        signatures: impl Iterator<Item = FunctionTypeRef<'a>>,
        local_functions: impl ExactSizeIterator<Item = (FunctionTypeRef<'a>, FunctionBodyRef<'a>)>,
        call_trampolines: impl ExactSizeIterator<Item = FunctionBodyRef<'a>>,
        dynamic_trampolines: impl ExactSizeIterator<Item = FunctionBodyRef<'a>>,
        local_sections: impl ExactSizeIterator<Item = CustomSectionRef<'a>>,
        jt_offsets: impl Fn(LocalFunctionIndex, JumpTable) -> wasmer_compiler::CodeOffset,
        function_relocations: impl Iterator<
            Item = (LocalFunctionIndex, impl Iterator<Item = Relocation>),
        >,
        section_relocations: impl Iterator<Item = (SectionIndex, impl Iterator<Item = Relocation>)>,
        trampolines: &Option<TrampolinesSection>,
        eh_frame: Option<(SectionIndex, CustomSectionRef<'a>)>,
        data_initializers: impl Iterator<Item = DataInitializer<'a>>,
        passive_data: BTreeMap<DataIndex, Arc<[u8]>>,
        imports: Vec<VMImport>,
        import_counts: EntityCounts,
        vmoffsets: VMOffsets,
        local_memories: Vec<(MemoryType, MemoryStyle)>,
        local_tables: Vec<(TableType, TableStyle)>,
        local_globals: Vec<GlobalType>,
    ) -> Result<std::sync::Arc<dyn Artifact>, CompileError> {
        let mut inner_engine = self.inner_mut();
        let signatures: PrimaryMap<_, _> = signatures
            .map(|sig| inner_engine.signatures.register(sig))
            .collect();
        // TODO(0-copy): allocate passive data here too.
        let (
            finished_functions,
            finished_call_trampolines,
            finished_dynamic_trampolines,
            custom_sections,
        ) = inner_engine.allocate(
            local_functions,
            call_trampolines,
            dynamic_trampolines,
            local_sections,
        )?;
        crate::link_module(
            &finished_functions,
            jt_offsets,
            function_relocations,
            &custom_sections,
            section_relocations,
            trampolines,
        );
        let eh_frame = eh_frame.map(|(idx, section)| unsafe {
            // SAFETY: custom sections should contain the debuginfo section at `idx`.
            std::slice::from_raw_parts(*custom_sections[idx], section.bytes.len())
        });

        // Make all code compiled thus far executable.
        inner_engine.publish_compiled_code();
        inner_engine.publish_eh_frame(eh_frame)?;

        Ok(Arc::new(UniversalArtifact {
            engine: self.clone(),
            import_counts,
            imports,
            finished_functions: finished_functions.into_boxed_slice(),
            finished_function_call_trampolines: finished_call_trampolines.into_boxed_slice(),
            finished_dynamic_function_trampolines: finished_dynamic_trampolines.into_boxed_slice(),
            signatures: signatures.into_boxed_slice(),
            frame_info_registration: Mutex::new(None),
            data_initializers: data_initializers.map(Into::into).collect(),
            passive_data,
            local_memories,
            local_tables,
            local_globals,
            vmoffsets,
        }))
    }
}

impl Engine for UniversalEngine {
    /// The target
    fn target(&self) -> &Target {
        &self.target
    }

    /// Register a signature
    fn register_signature(&self, func_type: FunctionTypeRef<'_>) -> VMSharedSignatureIndex {
        self.inner().signatures.register(func_type)
    }

    fn register_function_metadata(&self, func_data: VMCallerCheckedAnyfunc) -> VMFuncRef {
        self.inner().func_data().register(func_data)
    }

    /// Lookup a signature
    fn lookup_signature(&self, sig: VMSharedSignatureIndex) -> Option<FunctionType> {
        self.inner().signatures.lookup(sig).cloned()
    }

    /// Validates a WebAssembly module
    fn validate(&self, binary: &[u8]) -> Result<(), CompileError> {
        self.inner().validate(binary)
    }

    /// Compile a WebAssembly binary
    fn compile(
        &self,
        binary: &[u8],
        tunables: &dyn Tunables,
    ) -> Result<Box<dyn wasmer_engine::Executable>, CompileError> {
        if !cfg!(feature = "compiler") {
            return Err(CompileError::Codegen(
                "The UniversalEngine is operating in headless mode, so it can not compile Modules."
                    .to_string(),
            ));
        }
        let inner_engine = self.inner_mut();
        let features = inner_engine.features();
        let compiler = inner_engine.compiler()?;
        let environ = wasmer_compiler::ModuleEnvironment::new();
        let translation = environ.translate(binary).map_err(CompileError::Wasm)?;

        // Apply the middleware first
        let mut module = translation.module;
        let middlewares = compiler.get_middlewares();
        middlewares.apply_on_module_info(&mut module);

        let memory_styles: PrimaryMap<wasmer_types::MemoryIndex, _> = module
            .memories
            .values()
            .map(|memory_type| tunables.memory_style(memory_type))
            .collect();
        let table_styles: PrimaryMap<wasmer_types::TableIndex, _> = module
            .tables
            .values()
            .map(|table_type| tunables.table_style(table_type))
            .collect();
        let compile_info = wasmer_compiler::CompileModuleInfo {
            module: Arc::new(module),
            features: features.clone(),
            memory_styles,
            table_styles,
        };

        // Compile the Module
        let compilation = compiler.compile_module(
            &self.target(),
            &compile_info,
            // SAFETY: Calling `unwrap` is correct since
            // `environ.translate()` above will write some data into
            // `module_translation_state`.
            translation.module_translation_state.as_ref().unwrap(),
            translation.function_body_inputs,
        )?;
        let function_call_trampolines = compilation.get_function_call_trampolines();
        let dynamic_function_trampolines = compilation.get_dynamic_function_trampolines();
        let data_initializers = translation
            .data_initializers
            .iter()
            .map(wasmer_types::OwnedDataInitializer::new)
            .collect();

        let frame_infos = compilation.get_frame_info();
        Ok(Box::new(crate::UniversalExecutable {
            function_bodies: compilation.get_function_bodies(),
            function_relocations: compilation.get_relocations(),
            function_jt_offsets: compilation.get_jt_offsets(),
            function_frame_info: frame_infos,
            function_call_trampolines,
            dynamic_function_trampolines,
            custom_sections: compilation.get_custom_sections(),
            custom_section_relocations: compilation.get_custom_section_relocations(),
            debug: compilation.get_debug(),
            trampolines: compilation.get_trampolines(),
            compile_info,
            data_initializers,
            cpu_features: self.target().cpu_features().as_u64(),
        }))
    }

    fn load(
        &self,
        executable: &(dyn wasmer_engine::Executable),
    ) -> Result<Arc<dyn wasmer_engine::Artifact>, CompileError> {
        executable.load(self)
    }

    fn id(&self) -> &EngineId {
        &self.engine_id
    }

    fn cloned(&self) -> Arc<dyn Engine + Send + Sync> {
        Arc::new(self.clone())
    }
}

/// The inner contents of `UniversalEngine`
#[derive(MemoryUsage)]
pub struct UniversalEngineInner {
    /// The compiler
    #[cfg(feature = "compiler")]
    compiler: Option<Box<dyn Compiler>>,
    /// The features to compile the Wasm module with
    features: Features,
    /// The code memory is responsible of publishing the compiled
    /// functions to memory.
    code_memory: Vec<CodeMemory>,
    /// The signature registry is used mainly to operate with trampolines
    /// performantly.
    signatures: SignatureRegistry,
    /// The backing storage of `VMFuncRef`s. This centralized store ensures that 2
    /// functions with the same `VMCallerCheckedAnyfunc` will have the same `VMFuncRef`.
    /// It also guarantees that the `VMFuncRef`s stay valid until the engine is dropped.
    func_data: Arc<FuncDataRegistry>,
}

impl UniversalEngineInner {
    /// Gets the compiler associated to this engine.
    #[cfg(feature = "compiler")]
    pub fn compiler(&self) -> Result<&dyn Compiler, CompileError> {
        if self.compiler.is_none() {
            return Err(CompileError::Codegen("The UniversalEngine is operating in headless mode, so it can only execute already compiled Modules.".to_string()));
        }
        Ok(&**self.compiler.as_ref().unwrap())
    }

    /// Validate the module
    #[cfg(feature = "compiler")]
    pub fn validate<'data>(&self, data: &'data [u8]) -> Result<(), CompileError> {
        self.compiler()?.validate_module(self.features(), data)
    }

    /// Validate the module
    #[cfg(not(feature = "compiler"))]
    pub fn validate<'data>(&self, _data: &'data [u8]) -> Result<(), CompileError> {
        Err(CompileError::Validate(
            "The UniversalEngine is not compiled with compiler support, which is required for validating"
                .to_string(),
        ))
    }

    /// The Wasm features
    pub fn features(&self) -> &Features {
        &self.features
    }

    /// Allocate compiled functions into memory
    #[allow(clippy::type_complexity)]
    pub(crate) fn allocate<'a>(
        &mut self,
        local_functions: impl ExactSizeIterator<Item = (FunctionTypeRef<'a>, FunctionBodyRef<'a>)>,
        call_trampolines: impl ExactSizeIterator<Item = FunctionBodyRef<'a>>,
        dynamic_trampolines: impl ExactSizeIterator<Item = FunctionBodyRef<'a>>,
        custom_sections: impl ExactSizeIterator<Item = CustomSectionRef<'a>>,
    ) -> Result<
        (
            PrimaryMap<LocalFunctionIndex, VMLocalFunction>,
            PrimaryMap<SignatureIndex, VMTrampoline>,
            PrimaryMap<FunctionIndex, FunctionBodyPtr>,
            PrimaryMap<SectionIndex, SectionBodyPtr>,
        ),
        CompileError,
    > {
        let function_count = local_functions.len();
        let call_trampoline_count = call_trampolines.len();
        // TODO: these allocations should be unnecessary somehow.
        let mut function_types = Vec::with_capacity(function_count);
        let local_functions = local_functions.map(|(sig, b)| {
            function_types.push(self.signatures.register(sig));
            b
        });
        let function_bodies = local_functions
            .chain(call_trampolines)
            .chain(dynamic_trampolines)
            .collect::<Vec<_>>();

        // TOOD: this shouldn't be necessary....
        let mut section_types = Vec::with_capacity(custom_sections.len());
        let mut executable_sections = Vec::new();
        let mut data_sections = Vec::new();
        for section in custom_sections {
            if let CustomSectionProtection::ReadExecute = section.protection {
                executable_sections.push(section);
            } else {
                data_sections.push(section);
            }
            section_types.push(section.protection);
        }

        self.code_memory.push(CodeMemory::new());
        let code_memory = self.code_memory.last_mut().expect("infallible");

        let (mut allocated_functions, allocated_executable_sections, allocated_data_sections) =
            code_memory
                .allocate(
                    function_bodies.as_slice(),
                    executable_sections.as_slice(),
                    data_sections.as_slice(),
                )
                .map_err(|message| {
                    CompileError::Resource(format!(
                        "failed to allocate memory for functions: {}",
                        message
                    ))
                })?;

        let allocated_functions_result = allocated_functions
            .drain(0..function_count)
            .zip(function_types.into_iter())
            .map(|(slice, signature)| -> Result<_, CompileError> {
                Ok(VMLocalFunction {
                    body: FunctionBodyPtr(slice.as_ptr()),
                    length: u32::try_from(slice.len()).map_err(|_| {
                        CompileError::Codegen("function body length exceeds 4GiB".into())
                    })?,
                    signature,
                })
            })
            .collect::<Result<PrimaryMap<LocalFunctionIndex, _>, _>>()?;

        let mut allocated_function_call_trampolines: PrimaryMap<SignatureIndex, VMTrampoline> =
            PrimaryMap::new();
        for ptr in allocated_functions
            .drain(0..call_trampoline_count)
            .map(|slice| slice.as_ptr())
        {
            // TODO: What in damnation have you done?! – Bannon
            let trampoline =
                unsafe { std::mem::transmute::<*const VMFunctionBody, VMTrampoline>(ptr) };
            allocated_function_call_trampolines.push(trampoline);
        }

        let allocated_dynamic_function_trampolines = allocated_functions
            .drain(..)
            .map(|slice| FunctionBodyPtr(slice.as_ptr()))
            .collect::<PrimaryMap<FunctionIndex, _>>();

        let mut exec_iter = allocated_executable_sections.iter();
        let mut data_iter = allocated_data_sections.iter();
        let allocated_custom_sections = section_types
            .into_iter()
            .map(|protection| {
                SectionBodyPtr(
                    if protection == CustomSectionProtection::ReadExecute {
                        exec_iter.next()
                    } else {
                        data_iter.next()
                    }
                    .unwrap()
                    .as_ptr(),
                )
            })
            .collect::<PrimaryMap<SectionIndex, _>>();

        Ok((
            allocated_functions_result,
            allocated_function_call_trampolines,
            allocated_dynamic_function_trampolines,
            allocated_custom_sections,
        ))
    }

    /// Make memory containing compiled code executable.
    pub(crate) fn publish_compiled_code(&mut self) {
        self.code_memory.last_mut().unwrap().publish();
    }

    /// Register DWARF-type exception handling information associated with the code.
    pub(crate) fn publish_eh_frame(&mut self, eh_frame: Option<&[u8]>) -> Result<(), CompileError> {
        self.code_memory
            .last_mut()
            .unwrap()
            .unwind_registry_mut()
            .publish(eh_frame)
            .map_err(|e| {
                CompileError::Resource(format!("Error while publishing the unwind code: {}", e))
            })?;
        Ok(())
    }

    /// Shared func metadata registry.
    pub(crate) fn func_data(&self) -> &Arc<FuncDataRegistry> {
        &self.func_data
    }
}
