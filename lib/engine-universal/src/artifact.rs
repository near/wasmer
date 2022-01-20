//! Define `UniversalArtifact` to allow compiling and instantiating to be
//! done as separate steps.

use crate::engine::UniversalEngine;
#[cfg(feature = "compiler")]
use crate::serialize::SerializableCompilation;
use crate::serialize::SerializableModule;
use enumset::EnumSet;
use loupe::MemoryUsage;
use std::sync::Arc;
use wasmer_compiler::{CompileError, CpuFeature, Features, Triple};
#[cfg(feature = "compiler")]
use wasmer_compiler::{CompileModuleInfo, ModuleEnvironment, ModuleMiddlewareChain};
use wasmer_engine::{Artifact, DeserializeError, SerializeError};
#[cfg(feature = "compiler")]
use wasmer_engine::{Engine, Tunables};
use wasmer_types::entity::PrimaryMap;
use wasmer_types::{MemoryIndex, ModuleInfo, OwnedDataInitializer, TableIndex};
use wasmer_vm::{MemoryStyle, TableStyle};

const SERIALIZED_METADATA_LENGTH_OFFSET: usize = 22;
const SERIALIZED_METADATA_CONTENT_OFFSET: usize = 32;

/// A compiled wasm module, ready to be instantiated.
#[derive(MemoryUsage)]
pub struct UniversalArtifact {
    pub(crate) serializable: SerializableModule,
}

impl UniversalArtifact {
    const MAGIC_HEADER: &'static [u8; 22] = b"\0wasmer-universal\0\0\0\0\0";

    /// Check if the provided bytes look like a serialized `UniversalArtifact`.
    pub fn is_deserializable(bytes: &[u8]) -> bool {
        bytes.starts_with(Self::MAGIC_HEADER)
    }

    /// Compile a data buffer into a `UniversalArtifact`, which may then be instantiated.
    #[cfg(feature = "compiler")]
    pub fn new(
        engine: &UniversalEngine,
        data: &[u8],
        tunables: &dyn Tunables,
    ) -> Result<Self, CompileError> {
        let environ = ModuleEnvironment::new();
        let inner_engine = engine.inner_mut();
        let features = inner_engine.features();

        let translation = environ.translate(data).map_err(CompileError::Wasm)?;

        let compiler = inner_engine.compiler()?;

        // We try to apply the middleware first
        let mut module = translation.module;
        let middlewares = compiler.get_middlewares();
        middlewares.apply_on_module_info(&mut module);

        let memory_styles: PrimaryMap<MemoryIndex, MemoryStyle> = module
            .memories
            .values()
            .map(|memory_type| tunables.memory_style(memory_type))
            .collect();
        let table_styles: PrimaryMap<TableIndex, TableStyle> = module
            .tables
            .values()
            .map(|table_type| tunables.table_style(table_type))
            .collect();

        let compile_info = CompileModuleInfo {
            module: Arc::new(module),
            features: features.clone(),
            memory_styles,
            table_styles,
        };

        // Compile the Module
        let compilation = compiler.compile_module(
            &engine.target(),
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
            .map(OwnedDataInitializer::new)
            .collect::<Vec<_>>()
            .into_boxed_slice();

        let frame_infos = compilation.get_frame_info();

        let serializable_compilation = SerializableCompilation {
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
        };
        let serializable = SerializableModule {
            compilation: serializable_compilation,
            compile_info,
            data_initializers,
            cpu_features: engine.target().cpu_features().as_u64(),
        };
        Ok(Self { serializable })
    }

    /// Compile a data buffer into a `UniversalArtifact`, which may then be instantiated.
    #[cfg(not(feature = "compiler"))]
    pub fn new(_data: &[u8]) -> Result<Self, CompileError> {
        Err(CompileError::Codegen(
            "Compilation is not enabled in the engine".to_string(),
        ))
    }

    /// Deserialize a UniversalArtifact
    ///
    /// # Safety
    /// This function is unsafe because rkyv reads directly without validating
    /// the data.
    pub unsafe fn deserialize(bytes: &[u8]) -> Result<Self, DeserializeError> {
        if !Self::is_deserializable(bytes) {
            return Err(DeserializeError::Incompatible(
                "The provided bytes are not wasmer-universal".to_string(),
            ));
        }

        let mut inner_bytes = &bytes[SERIALIZED_METADATA_LENGTH_OFFSET..];

        let metadata_len = leb128::read::unsigned(&mut inner_bytes).map_err(|_e| {
            DeserializeError::CorruptedBinary("Can't read metadata size".to_string())
        })?;
        let metadata_slice: &[u8] = std::slice::from_raw_parts(
            &bytes[SERIALIZED_METADATA_CONTENT_OFFSET] as *const u8,
            metadata_len as usize,
        );

        let serializable = SerializableModule::deserialize(metadata_slice)?;
        Ok(Self { serializable })
    }

    /// Get the default extension when serializing this artifact
    pub fn get_default_extension(_triple: &Triple) -> &'static str {
        // `.wasmu` is the default extension for all the triples. It
        // stands for “Wasm Universal”.
        "wasmu"
    }
}

impl Artifact for UniversalArtifact {
    fn module(&self) -> Arc<ModuleInfo> {
        self.serializable.compile_info.module.clone()
    }

    fn module_ref(&self) -> &ModuleInfo {
        &self.serializable.compile_info.module
    }

    fn module_mut(&mut self) -> Option<&mut ModuleInfo> {
        Arc::get_mut(&mut self.serializable.compile_info.module)
    }

    fn features(&self) -> &Features {
        &self.serializable.compile_info.features
    }

    fn cpu_features(&self) -> EnumSet<CpuFeature> {
        EnumSet::from_u64(self.serializable.cpu_features)
    }

    fn data_initializers(&self) -> &[OwnedDataInitializer] {
        &*self.serializable.data_initializers
    }

    fn memory_styles(&self) -> &PrimaryMap<MemoryIndex, MemoryStyle> {
        &self.serializable.compile_info.memory_styles
    }

    fn table_styles(&self) -> &PrimaryMap<TableIndex, TableStyle> {
        &self.serializable.compile_info.table_styles
    }

    fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        // Prepend the header.
        let mut serialized = Self::MAGIC_HEADER.to_vec();

        serialized.resize(SERIALIZED_METADATA_CONTENT_OFFSET, 0);
        let mut writable_leb = &mut serialized[SERIALIZED_METADATA_LENGTH_OFFSET..];
        let serialized_data = self.serializable.serialize()?;
        let length = serialized_data.len();
        leb128::write::unsigned(&mut writable_leb, length as u64).expect("Should write number");

        let offset = pad_and_extend::<SerializableModule>(&mut serialized, &serialized_data);
        assert_eq!(offset, SERIALIZED_METADATA_CONTENT_OFFSET);

        Ok(serialized)
    }
}

/// It pads the data with the desired alignment
pub fn pad_and_extend<T>(prev_data: &mut Vec<u8>, data: &[u8]) -> usize {
    let align = std::mem::align_of::<T>();

    let mut offset = prev_data.len();
    if offset & (align - 1) != 0 {
        offset += align - (offset & (align - 1));
        prev_data.resize(offset, 0);
    }
    prev_data.extend(data);
    offset
}

#[cfg(test)]
mod tests {
    use super::pad_and_extend;

    #[test]
    fn test_pad_and_extend() {
        let mut data: Vec<u8> = vec![];
        let offset = pad_and_extend::<i64>(&mut data, &[1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(offset, 0);
        let offset = pad_and_extend::<i32>(&mut data, &[2, 0, 0, 0]);
        assert_eq!(offset, 8);
        let offset = pad_and_extend::<i64>(&mut data, &[3, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(offset, 16);
        assert_eq!(
            data,
            &[1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0]
        );
    }
}
