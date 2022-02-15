use enumset::EnumSet;
use loupe::MemoryUsage;
use rkyv::de::deserializers::SharedDeserializeMap;
use rkyv::ser::serializers::{
    AllocScratch, AllocScratchError, CompositeSerializer, CompositeSerializerError,
    SharedSerializeMap, SharedSerializeMapError, WriteSerializer,
};
use wasmer_compiler::{
    CompileError, CompileModuleInfo, CompiledFunctionFrameInfo, CpuFeature, CustomSection, Dwarf,
    Features, FunctionBody, JumpTableOffsets, Relocation, SectionIndex, TrampolinesSection,
};
use wasmer_engine::{DeserializeError, Engine};
use wasmer_types::entity::PrimaryMap;
use wasmer_types::{FunctionIndex, LocalFunctionIndex, OwnedDataInitializer, SignatureIndex};
use wasmer_vm::Artifact;

static MAGIC_HEADER: [u8; 22] = *b"\0wasmer-universal\xFF\xFF\xFF\xFF\xFF";

/// A 0-copy view of the encoded `UniversalExecutable` payload.
#[derive(Clone, Copy)]
pub struct UniversalExecutableRef<'a> {
    buffer: &'a [u8],
    archive: &'a ArchivedUniversalExecutable,
}

impl<'a> std::ops::Deref for UniversalExecutableRef<'a> {
    type Target = ArchivedUniversalExecutable;
    fn deref(&self) -> &Self::Target {
        self.archive
    }
}

impl<'a> UniversalExecutableRef<'a> {
    /// Verify the buffer for whether it is a valid `UniversalExecutable`.
    pub fn verify_serialized(data: &[u8]) -> Result<(), &'static str> {
        if !data.starts_with(&MAGIC_HEADER) {
            return Err("the provided bytes are not wasmer-universal");
        }
        if data.len() < MAGIC_HEADER.len() + 8 {
            return Err("the data buffer is too small to be valid");
        }
        let (_, position) = data.split_at(data.len() - 8);
        let mut position_value = [0u8; 8];
        position_value.copy_from_slice(position);
        if u64::from_le_bytes(position_value) < data.len() as u64 {
            return Err("the buffer is malformed");
        }
        // TODO(0-copy): bytecheck too.
        Ok(())
    }

    /// # Safety
    ///
    /// This method is unsafe since it deserializes data directly
    /// from memory.
    /// Right now we are not doing any extra work for validation, but
    /// `rkyv` has an option to do bytecheck on the serialized data before
    /// serializing (via `rkyv::check_archived_value`).
    pub unsafe fn deserialize(
        data: &'a [u8],
    ) -> Result<UniversalExecutableRef<'a>, DeserializeError> {
        Self::verify_serialized(data).map_err(|e| DeserializeError::Incompatible(e.to_string()))?;
        let (archive, position) = data.split_at(data.len() - 8);
        let mut position_value = [0u8; 8];
        position_value.copy_from_slice(position);
        Ok(UniversalExecutableRef {
            buffer: data,
            archive: rkyv::archived_value::<UniversalExecutable>(
                archive,
                u64::from_le_bytes(position_value) as usize,
            ),
        })
    }

    // TODO(0-copy): this should never fail.
    /// Convert this reference to an owned `UniversalExecutable` value.
    pub fn to_owned(self) -> Result<UniversalExecutable, DeserializeError> {
        let mut deserializer = SharedDeserializeMap::new();
        rkyv::Deserialize::deserialize(self.archive, &mut deserializer)
            .map_err(|e| DeserializeError::CorruptedBinary(format!("{:?}", e)))
    }
}

/// A wasm module compiled to some shape, ready to be loaded with `UniversalEngine` to produce an
/// `UniversalArtifact`.
///
/// This is the result obtained after validating and compiling a WASM module with any of the
/// supported compilers. This type falls in-between a module and [`Artifact`](crate::Artifact).
#[derive(MemoryUsage, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
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

#[derive(thiserror::Error, Debug)]
pub enum ExecutableSerializeError {
    #[error("could not serialize the executable data")]
    Executable(
        #[source]
        CompositeSerializerError<std::io::Error, AllocScratchError, SharedSerializeMapError>,
    ),
}

// SAFETY: the pointers in `rkyv::AllocScratchError` are present there for display purposes – this
// type does not expose any mechanism to access or operate on the pointers, unless they are
// accessed directly by the user.
unsafe impl Send for ExecutableSerializeError {}
unsafe impl Sync for ExecutableSerializeError {}

impl wasmer_engine::Executable for UniversalExecutable {
    fn load(
        &self,
        engine: &(dyn Engine + 'static),
    ) -> Result<std::sync::Arc<dyn Artifact>, CompileError> {
        engine
            .downcast_ref::<crate::UniversalEngine>()
            .ok_or_else(|| CompileError::Codegen("can't downcast TODO FIXME".into()))?
            .load_owned(self)
    }

    fn features(&self) -> Features {
        self.compile_info.features.clone()
    }

    fn cpu_features(&self) -> EnumSet<CpuFeature> {
        EnumSet::from_u64(self.cpu_features)
    }

    fn serialize(&self) -> Result<Vec<u8>, Box<(dyn std::error::Error + Send + Sync + 'static)>> {
        // The format is as thus:
        //
        // HEADER
        // RKYV PAYLOAD
        // RKYV POSITION
        //
        // It is expected that any framing for message length is handled by the caller.
        let mut out = Vec::with_capacity(32);
        out.extend(&MAGIC_HEADER);
        let mut serializer = CompositeSerializer::new(
            WriteSerializer::with_pos(std::io::Cursor::new(&mut out), MAGIC_HEADER.len()),
            AllocScratch::new(),
            SharedSerializeMap::new(),
        );
        let pos = rkyv::ser::Serializer::serialize_value(&mut serializer, self)
            .map_err(ExecutableSerializeError::Executable)? as u64;
        out.extend(&pos.to_le_bytes());
        Ok(out)
    }
}

impl<'a> wasmer_engine::Executable for UniversalExecutableRef<'a> {
    fn load(
        &self,
        engine: &(dyn Engine + 'static),
    ) -> Result<std::sync::Arc<dyn Artifact>, CompileError> {
        engine
            .downcast_ref::<crate::UniversalEngine>()
            .ok_or_else(|| CompileError::Codegen("can't downcast TODO FIXME".into()))?
            .load_archived(self)
    }

    fn features(&self) -> Features {
        unrkyv(&self.archive.compile_info.features)
    }

    fn cpu_features(&self) -> EnumSet<CpuFeature> {
        EnumSet::from_u64(unrkyv(&self.archive.cpu_features))
    }

    fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.buffer.to_vec())
    }
}

pub(crate) fn unrkyv<T>(archive: &T::Archived) -> T
where
    T: rkyv::Archive,
    T::Archived: rkyv::Deserialize<T, rkyv::Infallible>,
{
    Result::<_, std::convert::Infallible>::unwrap(rkyv::Deserialize::deserialize(
        archive,
        &mut rkyv::Infallible,
    ))
}
