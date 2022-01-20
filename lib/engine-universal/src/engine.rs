//! Universal compilation.

use crate::{CodeMemory, UniversalArtifact, UniversalExecutable};
use loupe::MemoryUsage;
use std::sync::{Arc, Mutex};
#[cfg(feature = "compiler")]
use wasmer_compiler::Compiler;
use wasmer_compiler::{
    CompileError, CustomSection, CustomSectionProtection, FunctionBody, ModuleMiddlewareChain,
    SectionIndex, Target,
};
use wasmer_engine::{Engine, EngineId, FunctionExtent, Tunables};
use wasmer_types::entity::PrimaryMap;
use wasmer_types::{
    Features, FunctionIndex, FunctionType, LocalFunctionIndex, ModuleInfo, SignatureIndex,
};
use wasmer_vm::{
    FuncDataRegistry, FunctionBodyPtr, SectionBodyPtr, SignatureRegistry, VMCallerCheckedAnyfunc,
    VMFuncRef, VMFunctionBody, VMSharedSignatureIndex, VMTrampoline,
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
}

impl Engine for UniversalEngine {
    /// The target
    fn target(&self) -> &Target {
        &self.target
    }

    /// Register a signature
    fn register_signature(&self, func_type: &FunctionType) -> VMSharedSignatureIndex {
        let compiler = self.inner();
        compiler.signatures().register(func_type)
    }

    fn register_function_metadata(&self, func_data: VMCallerCheckedAnyfunc) -> VMFuncRef {
        let compiler = self.inner();
        compiler.func_data().register(func_data)
    }

    /// Lookup a signature
    fn lookup_signature(&self, sig: VMSharedSignatureIndex) -> Option<FunctionType> {
        let compiler = self.inner();
        compiler.signatures().lookup(sig)
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
        executable: &(dyn wasmer_engine::Executable + 'static),
    ) -> Result<Arc<dyn wasmer_engine::Artifact>, CompileError> {
        let executable = executable
            .downcast_ref::<UniversalExecutable>()
            .expect("TODO");
        let mut inner_engine = self.inner_mut();
        let (
            finished_functions,
            finished_function_call_trampolines,
            finished_dynamic_function_trampolines,
            custom_sections,
        ) = inner_engine.allocate(
            &executable.compile_info.module,
            &executable.function_bodies,
            &executable.function_call_trampolines,
            &executable.dynamic_function_trampolines,
            &executable.custom_sections,
        )?;
        crate::link_module(
            &executable.compile_info.module,
            &finished_functions,
            &executable.function_jt_offsets,
            executable.function_relocations.clone(),
            &custom_sections,
            &executable.custom_section_relocations,
            &executable.trampolines,
        );
        // Compute indices into the shared signature table.
        let signatures = {
            let signature_registry = inner_engine.signatures();
            executable
                .compile_info
                .module
                .signatures
                .values()
                .map(|sig| signature_registry.register(sig))
                .collect::<PrimaryMap<_, _>>()
        };

        let eh_frame = match &executable.debug {
            Some(debug) => {
                let eh_frame_section_size = executable.custom_sections[debug.eh_frame].bytes.len();
                let eh_frame_section_pointer = custom_sections[debug.eh_frame];
                Some(unsafe {
                    std::slice::from_raw_parts(*eh_frame_section_pointer, eh_frame_section_size)
                })
            }
            None => None,
        };

        // Make all code compiled thus far executable.
        inner_engine.publish_compiled_code();
        inner_engine.publish_eh_frame(eh_frame)?;

        let finished_function_lengths = finished_functions
            .values()
            .map(|extent| extent.length)
            .collect::<PrimaryMap<LocalFunctionIndex, usize>>()
            .into_boxed_slice();
        let finished_functions = finished_functions
            .values()
            .map(|extent| extent.ptr)
            .collect::<PrimaryMap<LocalFunctionIndex, FunctionBodyPtr>>()
            .into_boxed_slice();
        let finished_function_call_trampolines =
            finished_function_call_trampolines.into_boxed_slice();
        let finished_dynamic_function_trampolines =
            finished_dynamic_function_trampolines.into_boxed_slice();
        let signatures = signatures.into_boxed_slice();
        let func_data_registry = inner_engine.func_data().clone();
        Ok(Arc::new(UniversalArtifact {
            executable: executable.clone(),
            finished_functions,
            finished_function_call_trampolines,
            finished_dynamic_function_trampolines,
            signatures,
            frame_info_registration: Mutex::new(None),
            finished_function_lengths,
            func_data_registry,
        }))
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
    pub(crate) fn allocate(
        &mut self,
        _module: &ModuleInfo,
        functions: &PrimaryMap<LocalFunctionIndex, FunctionBody>,
        function_call_trampolines: &PrimaryMap<SignatureIndex, FunctionBody>,
        dynamic_function_trampolines: &PrimaryMap<FunctionIndex, FunctionBody>,
        custom_sections: &PrimaryMap<SectionIndex, CustomSection>,
    ) -> Result<
        (
            PrimaryMap<LocalFunctionIndex, FunctionExtent>,
            PrimaryMap<SignatureIndex, VMTrampoline>,
            PrimaryMap<FunctionIndex, FunctionBodyPtr>,
            PrimaryMap<SectionIndex, SectionBodyPtr>,
        ),
        CompileError,
    > {
        let function_bodies = functions
            .values()
            .chain(function_call_trampolines.values())
            .chain(dynamic_function_trampolines.values())
            .collect::<Vec<_>>();
        let (executable_sections, data_sections): (Vec<_>, _) = custom_sections
            .values()
            .partition(|section| section.protection == CustomSectionProtection::ReadExecute);
        self.code_memory.push(CodeMemory::new());

        let (mut allocated_functions, allocated_executable_sections, allocated_data_sections) =
            self.code_memory
                .last_mut()
                .unwrap()
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
            .drain(0..functions.len())
            .map(|slice| FunctionExtent {
                ptr: FunctionBodyPtr(slice.as_ptr()),
                length: slice.len(),
            })
            .collect::<PrimaryMap<LocalFunctionIndex, _>>();

        let mut allocated_function_call_trampolines: PrimaryMap<SignatureIndex, VMTrampoline> =
            PrimaryMap::new();
        for ptr in allocated_functions
            .drain(0..function_call_trampolines.len())
            .map(|slice| slice.as_ptr())
        {
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
        let allocated_custom_sections = custom_sections
            .iter()
            .map(|(_, section)| {
                SectionBodyPtr(
                    if section.protection == CustomSectionProtection::ReadExecute {
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

    /// Shared signature registry.
    pub fn signatures(&self) -> &SignatureRegistry {
        &self.signatures
    }

    /// Shared func metadata registry.
    pub(crate) fn func_data(&self) -> &Arc<FuncDataRegistry> {
        &self.func_data
    }
}
