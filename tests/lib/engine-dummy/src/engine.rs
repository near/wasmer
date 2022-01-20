//! Dummy Engine.
use loupe::MemoryUsage;
use std::sync::Arc;
use wasmer_compiler::{CompileError, Features, Target};
use wasmer_engine::{Engine, EngineId, Tunables};
use wasmer_types::FunctionType;
use wasmer_vm::{
    FuncDataRegistry, SignatureRegistry, VMCallerCheckedAnyfunc, VMContext, VMFuncRef,
    VMFunctionBody, VMSharedSignatureIndex,
};

#[allow(dead_code)]
extern "C" fn dummy_trampoline(
    _context: *mut VMContext,
    _body: *const VMFunctionBody,
    _values: *mut u128,
) {
    panic!("Dummy engine can't call functions, since Wasm function bodies are not really compiled")
}

/// A WebAssembly `Dummy` Engine.
#[derive(Clone, MemoryUsage)]
pub struct DummyEngine {
    signatures: Arc<SignatureRegistry>,
    func_data: Arc<FuncDataRegistry>,
    features: Arc<Features>,
    target: Arc<Target>,
    engine_id: EngineId,
}

impl DummyEngine {
    #[cfg(feature = "compiler")]
    pub fn new() -> Self {
        Self {
            signatures: Arc::new(SignatureRegistry::new()),
            func_data: Arc::new(FuncDataRegistry::new()),
            features: Arc::new(Default::default()),
            target: Arc::new(Default::default()),
            engine_id: EngineId::default(),
        }
    }

    pub fn features(&self) -> &Features {
        &self.features
    }

    /// Shared func metadata registry.
    pub(crate) fn func_data(&self) -> &Arc<FuncDataRegistry> {
        &self.func_data
    }
}

impl Engine for DummyEngine {
    /// Get the tunables
    fn target(&self) -> &Target {
        &self.target
    }

    /// Register a signature
    fn register_signature(&self, func_type: &FunctionType) -> VMSharedSignatureIndex {
        self.signatures.register(func_type)
    }

    fn register_function_metadata(&self, func_data: VMCallerCheckedAnyfunc) -> VMFuncRef {
        self.func_data.register(func_data)
    }

    /// Lookup a signature
    fn lookup_signature(&self, sig: VMSharedSignatureIndex) -> Option<FunctionType> {
        self.signatures.lookup(sig)
    }

    #[cfg(feature = "compiler")]
    /// Validates a WebAssembly module
    fn validate(&self, binary: &[u8]) -> Result<(), CompileError> {
        use wasmer_compiler::wasmparser::{Validator, WasmFeatures};

        let features = self.features();
        let mut validator = Validator::new();
        let wasm_features = WasmFeatures {
            bulk_memory: features.bulk_memory,
            threads: features.threads,
            reference_types: features.reference_types,
            multi_value: features.multi_value,
            simd: features.simd,
            tail_call: features.tail_call,
            module_linking: features.module_linking,
            multi_memory: features.multi_memory,
            memory64: features.memory64,
            exceptions: features.exceptions,
            deterministic_only: false,
        };
        validator.wasm_features(wasm_features);
        validator
            .validate_all(binary)
            .map_err(|e| CompileError::Validate(format!("{}", e)))?;
        Ok(())
    }

    #[cfg(not(feature = "compiler"))]
    /// Validates a WebAssembly module
    fn validate(&self, binary: &[u8]) -> Result<(), CompileError> {
        // We mark all Wasm modules as valid
        Ok(())
    }

    /// Compile a WebAssembly binary
    fn compile(
        &self,
        _binary: &[u8],
        _tunables: &dyn Tunables,
    ) -> Result<Box<dyn wasmer_engine::Executable>, CompileError> {
        todo!()
        // Ok(Box::new(DummyArtifact::new(&self, binary, tunables)?))
    }

    fn id(&self) -> &EngineId {
        &self.engine_id
    }

    fn cloned(&self) -> Arc<dyn Engine + Send + Sync> {
        Arc::new(self.clone())
    }

    fn load(
        &self,
        _xecutable: &(dyn wasmer_engine::Executable + 'static),
    ) -> Result<Arc<dyn wasmer_engine::Artifact>, CompileError> {
        todo!()
    }
}
