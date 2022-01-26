use enumset::EnumSet;
use loupe::MemoryUsage;
use rkyv::ser::serializers::{
    AllocScratch, AllocScratchError, CompositeSerializer, CompositeSerializerError,
    SharedSerializeMap, SharedSerializeMapError, WriteSerializer,
};
use rkyv::{
    de::deserializers::SharedDeserializeMap, ser::Serializer as RkyvSerializer, Archive,
    Deserialize as RkyvDeserialize, Serialize as RkyvSerialize,
};
use wasmer_compiler::{
    CompileModuleInfo, CompiledFunctionFrameInfo, CustomSection, Dwarf, FunctionBody,
    JumpTableOffsets, Relocation, SectionIndex, TrampolinesSection,
};
use wasmer_engine::DeserializeError;
use wasmer_types::entity::PrimaryMap;
use wasmer_types::{FunctionIndex, LocalFunctionIndex, OwnedDataInitializer, SignatureIndex};

static MAGIC_HEADER: [u8; 22] = *b"\0wasmer-universal\0\0\0\0\0";

/// A wasm module compiled to some shape, ready to be loaded with `UniversalEngine` to produce an
/// `UniversalArtifact`.
///
/// This is the result obtained after validating and compiling a WASM module with any of the
/// supported compilers. This type falls in-between a module and [`Artifact`](crate::Artifact).
#[derive(MemoryUsage, Archive, RkyvDeserialize, RkyvSerialize, Clone)]
pub struct UniversalExecutable {
    pub(crate) function_bodies: PrimaryMap<LocalFunctionIndex, FunctionBody>,
    pub(crate) function_relocations: PrimaryMap<LocalFunctionIndex, Vec<Relocation>>,
    pub(crate) function_jt_offsets: PrimaryMap<LocalFunctionIndex, JumpTableOffsets>,
    pub(crate) function_frame_info: PrimaryMap<LocalFunctionIndex, CompiledFunctionFrameInfo>,
    pub(crate) function_call_trampolines: PrimaryMap<SignatureIndex, FunctionBody>,
    pub(crate) dynamic_function_trampolines: PrimaryMap<FunctionIndex, FunctionBody>,
    pub(crate) custom_sections: PrimaryMap<SectionIndex, CustomSection>,
    pub(crate) custom_section_relocations: PrimaryMap<SectionIndex, Vec<Relocation>>,
    // The section indices corresponding to the Dwarf debug info
    pub(crate) debug: Option<Dwarf>,
    // the Trampoline for Arm arch
    pub(crate) trampolines: Option<TrampolinesSection>,
    pub(crate) compile_info: CompileModuleInfo,
    pub(crate) data_initializers: Vec<OwnedDataInitializer>,
    pub(crate) cpu_features: u64,
}

impl UniversalExecutable {
    /// Deserialize a Module from a slice.
    /// The slice must have the following format:
    /// RKYV serialization (any length) + POS (8 bytes)
    ///
    /// # Safety
    ///
    /// This method is unsafe since it deserializes data directly
    /// from memory.
    /// Right now we are not doing any extra work for validation, but
    /// `rkyv` has an option to do bytecheck on the serialized data before
    /// serializing (via `rkyv::check_archived_value`).
    pub unsafe fn deserialize(metadata_slice: &[u8]) -> Result<Self, DeserializeError> {
        let archived = Self::archive_from_slice(metadata_slice)?;
        let mut deserializer = SharedDeserializeMap::new();
        RkyvDeserialize::deserialize(archived, &mut deserializer)
            .map_err(|e| DeserializeError::CorruptedBinary(format!("{:?}", e)))
    }

    /// # Safety
    ///
    /// This method is unsafe.
    /// Please check `SerializableModule::deserialize` for more details.
    pub unsafe fn archive_from_slice<'a>(
        data: &'a [u8],
    ) -> Result<&'a rkyv::Archived<UniversalExecutable>, DeserializeError> {
        if !data.starts_with(&MAGIC_HEADER) {
            return Err(DeserializeError::Incompatible(
                "the provided bytes are not wasmer-universal".to_string(),
            ));
        } else if data.len() < MAGIC_HEADER.len() + 8 {
            return Err(DeserializeError::Incompatible(
                "invalid serialized data".into(),
            ));
        }
        let (archive, position) = data.split_at(data.len() - 8);
        let mut position_value = [0u8; 8];
        position_value.copy_from_slice(position);
        Ok(rkyv::archived_value::<UniversalExecutable>(
            archive,
            u64::from_le_bytes(position_value) as usize,
        ))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ExecutableSerializeError {
    #[error("could not write universal executable header")]
    WriteHeader(#[source] std::io::Error),
    #[error("could not serialize the executable data")]
    Executable(
        #[source]
        CompositeSerializerError<std::io::Error, AllocScratchError, SharedSerializeMapError>,
    ),
    #[error("could not write the position information")]
    WritePosition(#[source] std::io::Error),
    #[error("could not flush the writer")]
    Flush(#[source] std::io::Error),
}

// SAFETY: the pointers in `rkyv::AllocScratchError` are present there for display purposes – this
// type does not expose any mechanism to access or operate on the pointers, unless they are
// accessed directly by the user.
unsafe impl Send for ExecutableSerializeError {}
unsafe impl Sync for ExecutableSerializeError {}

impl wasmer_engine::Executable for UniversalExecutable {
    fn features(&self) -> &wasmer_compiler::Features {
        &self.compile_info.features
    }

    fn cpu_features(&self) -> EnumSet<wasmer_compiler::CpuFeature> {
        EnumSet::from_u64(self.cpu_features)
    }

    fn memory_styles(&self) -> &PrimaryMap<wasmer_types::MemoryIndex, wasmer_vm::MemoryStyle> {
        &self.compile_info.memory_styles
    }

    fn table_styles(&self) -> &PrimaryMap<wasmer_types::TableIndex, wasmer_vm::TableStyle> {
        &self.compile_info.table_styles
    }

    fn data_initializers(&self) -> &[wasmer_types::OwnedDataInitializer] {
        &self.data_initializers
    }

    fn serialize(
        &self,
        out: &mut dyn std::io::Write,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // The format is as thus:
        //
        // HEADER
        // RKYV PAYLOAD
        // RKYV POSITION
        //
        // It is expected that any framing for message length is handled by the caller.
        out.write_all(&MAGIC_HEADER)
            .map_err(ExecutableSerializeError::WriteHeader)?;
        let mut serializer = CompositeSerializer::new(
            WriteSerializer::with_pos(out, MAGIC_HEADER.len()),
            AllocScratch::new(),
            SharedSerializeMap::new(),
        );
        let pos = serializer
            .serialize_value(self)
            .map_err(ExecutableSerializeError::Executable)? as u64;
        let out = serializer.into_serializer().into_inner();
        out.write_all(&pos.to_le_bytes())
            .map_err(ExecutableSerializeError::WritePosition)?;
        Ok(out.flush().map_err(ExecutableSerializeError::Flush)?)
    }
}
