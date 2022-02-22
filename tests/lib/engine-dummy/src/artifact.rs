//! Define `DummyArtifact` to allow compiling and instantiating to be
//! done as separate steps.

use crate::engine::DummyEngine;
use loupe::MemoryUsage;
#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use wasmer_compiler::CompileError;
#[cfg(feature = "compiler")]
use wasmer_compiler::ModuleEnvironment;
use wasmer_engine::{DeserializeError, Engine as _};
use wasmer_types::entity::{BoxedSlice, PrimaryMap};
use wasmer_types::{
    Features, FunctionIndex, InstanceConfig, LocalFunctionIndex, MemoryIndex, ModuleInfo,
    OwnedDataInitializer, SignatureIndex, TableIndex,
};
use wasmer_vm::{
    Artifact, FunctionBodyPtr, InstanceHandle, MemoryStyle, Resolver, TableStyle, Tunables,
    VMContext, VMFunctionBody, VMLocalFunction, VMSharedSignatureIndex, VMTrampoline,
};

/// Serializable struct for the artifact
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
#[derive(MemoryUsage)]
pub struct DummyArtifactMetadata {
    pub module: Arc<ModuleInfo>,
    pub features: Features,
    pub data_initializers: Box<[OwnedDataInitializer]>,
    // Plans for that module
    pub memory_styles: PrimaryMap<MemoryIndex, MemoryStyle>,
    pub table_styles: PrimaryMap<TableIndex, TableStyle>,
    pub cpu_features: u64,
}

/// A Dummy artifact.
///
/// This artifact will point to fake finished functions and trampolines
/// as no functions are really compiled.
#[derive(MemoryUsage)]
pub struct DummyArtifact {
    engine: Arc<Mutex<crate::engine::Inner>>,
    metadata: DummyArtifactMetadata,
    finished_functions: BoxedSlice<LocalFunctionIndex, VMLocalFunction>,
    #[loupe(skip)]
    finished_function_call_trampolines: BoxedSlice<SignatureIndex, VMTrampoline>,
    finished_dynamic_function_trampolines: BoxedSlice<FunctionIndex, FunctionBodyPtr>,
    signatures: BoxedSlice<SignatureIndex, VMSharedSignatureIndex>,
}

extern "C" fn dummy_function(_context: *mut VMContext) {
    panic!("Dummy engine can't generate functions")
}

extern "C" fn dummy_trampoline(
    _context: *mut VMContext,
    _callee: *const VMFunctionBody,
    _values: *mut u128,
) {
    panic!("Dummy engine can't generate trampolines")
}

impl DummyArtifact {
    const MAGIC_HEADER: &'static [u8] = b"\0wasmer-dummy";

    /// Check if the provided bytes look like a serialized `DummyArtifact`.
    pub fn is_deserializable(bytes: &[u8]) -> bool {
        bytes.starts_with(Self::MAGIC_HEADER)
    }

    #[cfg(feature = "compiler")]
    /// Compile a data buffer into a `DummyArtifact`, which may then be instantiated.
    pub fn new(
        engine: &DummyEngine,
        data: &[u8],
        tunables: &dyn Tunables,
    ) -> Result<Self, CompileError> {
        let environ = ModuleEnvironment::new();

        let translation = environ.translate(data).map_err(CompileError::Wasm)?;

        let memory_styles: PrimaryMap<MemoryIndex, MemoryStyle> = translation
            .module
            .memories
            .values()
            .map(|memory_type| tunables.memory_style(memory_type))
            .collect();
        let table_styles: PrimaryMap<TableIndex, TableStyle> = translation
            .module
            .tables
            .values()
            .map(|table_type| tunables.table_style(table_type))
            .collect();

        let data_initializers = translation
            .data_initializers
            .iter()
            .map(OwnedDataInitializer::new)
            .collect::<Vec<_>>()
            .into_boxed_slice();

        let metadata = DummyArtifactMetadata {
            module: Arc::new(translation.module),
            features: Features::default(),
            data_initializers,
            memory_styles,
            table_styles,
            cpu_features: engine.target().cpu_features().as_u64(),
        };
        Self::from_parts(&engine, metadata)
    }

    #[cfg(not(feature = "compiler"))]
    pub fn new(engine: &DummyEngine, data: &[u8]) -> Result<Self, CompileError> {
        CompileError::Generic("The compiler feature is not enabled in the DummyEngine")
    }

    #[cfg(feature = "serialize")]
    /// Deserialize a DummyArtifact
    pub fn deserialize(engine: &DummyEngine, bytes: &[u8]) -> Result<Self, DeserializeError> {
        if !Self::is_deserializable(bytes) {
            return Err(DeserializeError::Incompatible(
                "The provided bytes are not of the dummy engine".to_string(),
            ));
        }

        let inner_bytes = &bytes[Self::MAGIC_HEADER.len()..];

        let metadata: DummyArtifactMetadata = bincode::deserialize(inner_bytes)
            .map_err(|e| DeserializeError::CorruptedBinary(format!("{:?}", e)))?;

        Self::from_parts(&engine, metadata).map_err(DeserializeError::Compiler)
    }

    #[cfg(not(feature = "serialize"))]
    pub fn deserialize(engine: &DummyEngine, bytes: &[u8]) -> Result<Self, DeserializeError> {
        Err(DeserializeError::Generic(
            "The serializer feature is not enabled in the DummyEngine",
        ))
    }

    /// Construct a `DummyArtifact` from component parts.
    pub fn from_parts(
        engine: &DummyEngine,
        metadata: DummyArtifactMetadata,
    ) -> Result<Self, CompileError> {
        todo!()
    }
}

impl Artifact for DummyArtifact {
    unsafe fn instantiate(
        self: Arc<Self>,
        _: &dyn Tunables,
        _: &dyn Resolver,
        _: Box<dyn std::any::Any>,
        _: InstanceConfig,
    ) -> Result<InstanceHandle, Box<dyn std::error::Error + Send + Sync>> {
        todo!()
    }

    fn offsets(&self) -> &wasmer_vm::VMOffsets {
        todo!()
    }

    fn import_counts(&self) -> &wasmer_types::EntityCounts {
        todo!()
    }

    fn functions(&self) -> &BoxedSlice<LocalFunctionIndex, VMLocalFunction> {
        todo!()
    }

    fn passive_elements(
        &self,
    ) -> &std::collections::BTreeMap<wasmer_types::ElemIndex, Box<[FunctionIndex]>> {
        todo!()
    }

    fn element_segments(&self) -> &[wasmer_types::OwnedTableInitializer] {
        todo!()
    }

    fn data_segments(&self) -> &[OwnedDataInitializer] {
        todo!()
    }

    fn globals(&self) -> &[(wasmer_types::GlobalType, wasmer_types::GlobalInit)] {
        todo!()
    }

    fn start_function(&self) -> Option<FunctionIndex> {
        todo!()
    }

    fn function_by_export_field(&self, name: &str) -> Option<FunctionIndex> {
        todo!()
    }

    fn signatures(&self) -> &[VMSharedSignatureIndex] {
        todo!()
    }

    fn function_signature(&self, index: FunctionIndex) -> Option<VMSharedSignatureIndex> {
        todo!()
    }
}
