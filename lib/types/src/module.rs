// This file contains code from external sources.
// Attributions: https://github.com/wasmerio/wasmer/blob/master/ATTRIBUTIONS.md

//! Data structure for representing WebAssembly modules in a
//! `wasmer::Module`.

use crate::entity::{EntityRef, PrimaryMap};
#[cfg(feature = "enable-rkyv")]
use crate::ArchivableIndexMap;
use crate::{
    CustomSectionIndex, DataIndex, ElemIndex, ExportIndex, ExportType, ExternType, FunctionIndex,
    FunctionType, GlobalIndex, GlobalInit, GlobalType, Import, ImportIndex, LocalFunctionIndex,
    LocalGlobalIndex, LocalMemoryIndex, LocalTableIndex, MemoryIndex, MemoryType, SignatureIndex,
    TableIndex, OwnedTableInitializer, TableType,
};
use indexmap::IndexMap;
use loupe::MemoryUsage;
#[cfg(feature = "enable-rkyv")]
use rkyv::{
    de::SharedDeserializeRegistry, ser::ScratchSpace, ser::Serializer,
    ser::SharedSerializeRegistry, Archive, Archived, Deserialize as RkyvDeserialize, Fallible,
    Serialize as RkyvSerialize,
};
#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "enable-rkyv")]
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt;
use std::iter::ExactSizeIterator;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::Arc;

#[derive(Debug, Clone, MemoryUsage)]
#[cfg_attr(
    feature = "enable-rkyv",
    derive(RkyvSerialize, RkyvDeserialize, Archive)
)]
pub struct ModuleId {
    id: usize,
}

impl ModuleId {
    pub fn id(&self) -> String {
        format!("{}", &self.id)
    }
}

impl Default for ModuleId {
    fn default() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        Self {
            id: NEXT_ID.fetch_add(1, SeqCst),
        }
    }
}

/// The counts of imported entities in a WebAssembly module.
#[derive(
    Debug,
    Copy,
    Clone,
    Default,
    PartialEq,
    Eq,
    loupe::MemoryUsage,
    rkyv::Serialize,
    rkyv::Deserialize,
    rkyv::Archive,
)]
#[cfg_attr(feature = "enable-serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityCounts {
    /// Number of imported functions in the module.
    pub functions: usize,

    /// Number of imported tables in the module.
    pub tables: usize,

    /// Number of imported memories in the module.
    pub memories: usize,

    /// Number of imported globals in the module.
    pub globals: usize,
}

/// A translated WebAssembly module, excluding the function bodies and
/// memory initializers.
#[derive(Debug, Clone, Default, MemoryUsage)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct ModuleInfo {
    /// A unique identifier (within this process) for this module.
    ///
    /// We skip serialization/deserialization of this field, as it
    /// should be computed by the process.
    /// It's not skipped in rkyv, but that is okay, because even though it's skipped in bincode/serde
    /// it's still deserialized back as a garbage number, and later override from computed by the process
    #[cfg_attr(feature = "enable-serde", serde(skip_serializing, skip_deserializing))]
    pub id: ModuleId,

    /// The name of this wasm module, often found in the wasm file.
    pub name: Option<String>,

    /// Imported entities with the (module, field, index_of_the_import)
    ///
    /// Keeping the `index_of_the_import` is important, as there can be
    /// two same references to the same import, and we don't want to confuse
    /// them.
    pub imports: IndexMap<(String, String, u32), ImportIndex>,

    /// Exported entities.
    pub exports: IndexMap<String, ExportIndex>,

    /// The module "start" function, if present.
    pub start_function: Option<FunctionIndex>,

    /// WebAssembly table initializers.
    pub table_initializers: Vec<OwnedTableInitializer>,

    /// WebAssembly passive elements.
    #[loupe(skip)] // TODO(0-copy): don't skip loupe
    pub passive_elements: BTreeMap<ElemIndex, Box<[FunctionIndex]>>,

    /// WebAssembly passive data segments.
    #[loupe(skip)] // TODO(0-copy): don't skip loupe
    pub passive_data: BTreeMap<DataIndex, Arc<[u8]>>,

    /// WebAssembly global initializers.
    pub global_initializers: PrimaryMap<LocalGlobalIndex, GlobalInit>,

    /// WebAssembly function names.
    pub function_names: HashMap<FunctionIndex, String>,

    /// WebAssembly function signatures.
    pub signatures: PrimaryMap<SignatureIndex, FunctionType>,

    /// WebAssembly functions (imported and local).
    pub functions: PrimaryMap<FunctionIndex, SignatureIndex>,

    /// WebAssembly tables (imported and local).
    pub tables: PrimaryMap<TableIndex, TableType>,

    /// WebAssembly linear memories (imported and local).
    pub memories: PrimaryMap<MemoryIndex, MemoryType>,

    /// WebAssembly global variables (imported and local).
    pub globals: PrimaryMap<GlobalIndex, GlobalType>,

    /// Custom sections in the module.
    pub custom_sections: IndexMap<String, CustomSectionIndex>,

    /// The data for each CustomSection in the module.
    pub custom_sections_data: PrimaryMap<CustomSectionIndex, Arc<[u8]>>,

    /// The counts of imported entities.
    pub import_counts: EntityCounts,
}

/// Mirror version of ModuleInfo that can derive rkyv traits
#[cfg(feature = "enable-rkyv")]
#[derive(RkyvSerialize, RkyvDeserialize, Archive)]
pub struct ArchivableModuleInfo {
    pub name: Option<String>,
    pub imports: ArchivableIndexMap<(String, String, u32), ImportIndex>,
    pub exports: ArchivableIndexMap<String, ExportIndex>,
    pub start_function: Option<FunctionIndex>,
    pub table_initializers: Vec<OwnedTableInitializer>,
    pub passive_elements: BTreeMap<ElemIndex, Box<[FunctionIndex]>>,
    pub passive_data: BTreeMap<DataIndex, Arc<[u8]>>,
    pub global_initializers: PrimaryMap<LocalGlobalIndex, GlobalInit>,
    pub function_names: BTreeMap<FunctionIndex, String>,
    pub signatures: PrimaryMap<SignatureIndex, FunctionType>,
    pub functions: PrimaryMap<FunctionIndex, SignatureIndex>,
    pub tables: PrimaryMap<TableIndex, TableType>,
    pub memories: PrimaryMap<MemoryIndex, MemoryType>,
    pub globals: PrimaryMap<GlobalIndex, GlobalType>,
    pub custom_sections: ArchivableIndexMap<String, CustomSectionIndex>,
    pub custom_sections_data: PrimaryMap<CustomSectionIndex, Arc<[u8]>>,
    pub import_counts: EntityCounts,
}

#[cfg(feature = "enable-rkyv")]
impl From<ModuleInfo> for ArchivableModuleInfo {
    fn from(it: ModuleInfo) -> ArchivableModuleInfo {
        ArchivableModuleInfo {
            name: it.name,
            imports: ArchivableIndexMap::from(it.imports),
            exports: ArchivableIndexMap::from(it.exports),
            start_function: it.start_function,
            table_initializers: it.table_initializers,
            passive_elements: it.passive_elements.into_iter().collect(),
            passive_data: it.passive_data.into_iter().collect(),
            global_initializers: it.global_initializers,
            function_names: it.function_names.into_iter().collect(),
            signatures: it.signatures,
            functions: it.functions,
            tables: it.tables,
            memories: it.memories,
            globals: it.globals,
            custom_sections: ArchivableIndexMap::from(it.custom_sections),
            custom_sections_data: it.custom_sections_data,
            import_counts: it.import_counts,
        }
    }
}

#[cfg(feature = "enable-rkyv")]
impl From<ArchivableModuleInfo> for ModuleInfo {
    fn from(it: ArchivableModuleInfo) -> ModuleInfo {
        ModuleInfo {
            id: Default::default(),
            name: it.name,
            imports: it.imports.into(),
            exports: it.exports.into(),
            start_function: it.start_function,
            table_initializers: it.table_initializers,
            passive_elements: it.passive_elements.into_iter().collect(),
            passive_data: it.passive_data.into_iter().collect(),
            global_initializers: it.global_initializers,
            function_names: it.function_names.into_iter().collect(),
            signatures: it.signatures,
            functions: it.functions,
            tables: it.tables,
            memories: it.memories,
            globals: it.globals,
            custom_sections: it.custom_sections.into(),
            custom_sections_data: it.custom_sections_data,
            import_counts: it.import_counts,
        }
    }
}

#[cfg(feature = "enable-rkyv")]
impl From<&ModuleInfo> for ArchivableModuleInfo {
    fn from(it: &ModuleInfo) -> ArchivableModuleInfo {
        ArchivableModuleInfo::from(it.clone())
    }
}

#[cfg(feature = "enable-rkyv")]
impl Archive for ModuleInfo {
    type Archived = <ArchivableModuleInfo as Archive>::Archived;
    type Resolver = <ArchivableModuleInfo as Archive>::Resolver;

    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
        ArchivableModuleInfo::from(self).resolve(pos, resolver, out)
    }
}

#[cfg(feature = "enable-rkyv")]
impl<S: Serializer + SharedSerializeRegistry + ScratchSpace + ?Sized> RkyvSerialize<S>
    for ModuleInfo
{
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivableModuleInfo::from(self).serialize(serializer)
    }
}

#[cfg(feature = "enable-rkyv")]
impl<D: Fallible + ?Sized + SharedDeserializeRegistry> RkyvDeserialize<ModuleInfo, D>
    for Archived<ModuleInfo>
{
    fn deserialize(&self, deserializer: &mut D) -> Result<ModuleInfo, D::Error> {
        let r: ArchivableModuleInfo =
            RkyvDeserialize::<ArchivableModuleInfo, D>::deserialize(self, deserializer)?;
        Ok(ModuleInfo::from(r))
    }
}

// For test serialization correctness, everything except module id should be same
impl PartialEq for ModuleInfo {
    fn eq(&self, other: &ModuleInfo) -> bool {
        self.name == other.name
            && self.imports == other.imports
            && self.exports == other.exports
            && self.start_function == other.start_function
            && self.table_initializers == other.table_initializers
            && self.passive_elements == other.passive_elements
            && self.passive_data == other.passive_data
            && self.global_initializers == other.global_initializers
            && self.function_names == other.function_names
            && self.signatures == other.signatures
            && self.functions == other.functions
            && self.tables == other.tables
            && self.memories == other.memories
            && self.globals == other.globals
            && self.custom_sections == other.custom_sections
            && self.custom_sections_data == other.custom_sections_data
            && self.import_counts == other.import_counts
    }
}

impl Eq for ModuleInfo {}

impl ModuleInfo {
    /// Allocates the module data structures.
    pub fn new() -> Self {
        Default::default()
    }

    /// Get the given passive element, if it exists.
    pub fn get_passive_element(&self, index: ElemIndex) -> Option<&[FunctionIndex]> {
        self.passive_elements.get(&index).map(|es| &**es)
    }

    /// Get the exported signatures of the module
    pub fn exported_signatures(&self) -> Vec<FunctionType> {
        self.exports
            .iter()
            .filter_map(|(_name, export_index)| match export_index {
                ExportIndex::Function(i) => {
                    let signature = self.functions.get(*i).unwrap();
                    let func_type = self.signatures.get(*signature).unwrap();
                    Some(func_type.clone())
                }
                _ => None,
            })
            .collect::<Vec<FunctionType>>()
    }

    /// Get the export types of the module
    pub fn exports<'a>(&'a self) -> ExportsIterator<impl Iterator<Item = ExportType> + 'a> {
        let iter = self.exports.iter().map(move |(name, export_index)| {
            let extern_type = match export_index {
                ExportIndex::Function(i) => {
                    let signature = self.functions.get(*i).unwrap();
                    let func_type = self.signatures.get(*signature).unwrap();
                    ExternType::Function(func_type.clone())
                }
                ExportIndex::Table(i) => {
                    let table_type = self.tables.get(*i).unwrap();
                    ExternType::Table(*table_type)
                }
                ExportIndex::Memory(i) => {
                    let memory_type = self.memories.get(*i).unwrap();
                    ExternType::Memory(*memory_type)
                }
                ExportIndex::Global(i) => {
                    let global_type = self.globals.get(*i).unwrap();
                    ExternType::Global(*global_type)
                }
            };
            ExportType::new(name, extern_type)
        });
        ExportsIterator::new(iter, self.exports.len())
    }

    // /// Get the import types of the module
    // pub fn imports<'a>(&'a self) -> ImportsIterator<'a> {
    //     let iter = self.imports.iter().map(move |(&(ref m, ref f, i), index)| {
    //         let extern_type = match index {
    //             ImportIndex::Function(i) => {
    //                 let signature = self.functions.get(*i).unwrap();
    //                 let func_type = self.signatures.get(*signature).unwrap();
    //                 ExternType::Function(func_type.clone())
    //             }
    //             ImportIndex::Table(i) => {
    //                 let table_type = self.tables.get(*i).unwrap();
    //                 ExternType::Table(*table_type)
    //             }
    //             ImportIndex::Memory(i) => {
    //                 let memory_type = self.memories.get(*i).unwrap();
    //                 ExternType::Memory(*memory_type)
    //             }
    //             ImportIndex::Global(i) => {
    //                 let global_type = self.globals.get(*i).unwrap();
    //                 ExternType::Global(*global_type)
    //             }
    //         };
    //         (index.clone(), Import::new(m.as_str(), f.as_str(), i, extern_type))
    //     });
    //     ImportsIterator {
    //         iter: Box::new(iter),
    //     }
    // }

    /// Get the custom sections of the module given a `name`.
    pub fn custom_sections<'a>(&'a self, name: &'a str) -> impl Iterator<Item = Arc<[u8]>> + 'a {
        self.custom_sections
            .iter()
            .filter_map(move |(section_name, section_index)| {
                if name != section_name {
                    return None;
                }
                Some(self.custom_sections_data[*section_index].clone())
            })
    }

    /// Convert a `LocalFunctionIndex` into a `FunctionIndex`.
    pub fn func_index(&self, local_func: LocalFunctionIndex) -> FunctionIndex {
        FunctionIndex::new(self.import_counts.functions + local_func.index())
    }

    /// Convert a `FunctionIndex` into a `LocalFunctionIndex`. Returns None if the
    /// index is an imported function.
    pub fn local_func_index(&self, func: FunctionIndex) -> Option<LocalFunctionIndex> {
        func.index()
            .checked_sub(self.import_counts.functions)
            .map(LocalFunctionIndex::new)
    }

    /// Test whether the given function index is for an imported function.
    pub fn is_imported_function(&self, index: FunctionIndex) -> bool {
        index.index() < self.import_counts.functions
    }

    /// Convert a `LocalTableIndex` into a `TableIndex`.
    pub fn table_index(&self, local_table: LocalTableIndex) -> TableIndex {
        TableIndex::new(self.import_counts.tables + local_table.index())
    }

    /// Convert a `TableIndex` into a `LocalTableIndex`. Returns None if the
    /// index is an imported table.
    pub fn local_table_index(&self, table: TableIndex) -> Option<LocalTableIndex> {
        table
            .index()
            .checked_sub(self.import_counts.tables)
            .map(LocalTableIndex::new)
    }

    /// Test whether the given table index is for an imported table.
    pub fn is_imported_table(&self, index: TableIndex) -> bool {
        index.index() < self.import_counts.tables
    }

    /// Convert a `LocalMemoryIndex` into a `MemoryIndex`.
    pub fn memory_index(&self, local_memory: LocalMemoryIndex) -> MemoryIndex {
        MemoryIndex::new(self.import_counts.memories + local_memory.index())
    }

    /// Convert a `MemoryIndex` into a `LocalMemoryIndex`. Returns None if the
    /// index is an imported memory.
    pub fn local_memory_index(&self, memory: MemoryIndex) -> Option<LocalMemoryIndex> {
        memory
            .index()
            .checked_sub(self.import_counts.memories)
            .map(LocalMemoryIndex::new)
    }

    /// Test whether the given memory index is for an imported memory.
    pub fn is_imported_memory(&self, index: MemoryIndex) -> bool {
        index.index() < self.import_counts.memories
    }

    /// Convert a `LocalGlobalIndex` into a `GlobalIndex`.
    pub fn global_index(&self, local_global: LocalGlobalIndex) -> GlobalIndex {
        GlobalIndex::new(self.import_counts.globals + local_global.index())
    }

    /// Convert a `GlobalIndex` into a `LocalGlobalIndex`. Returns None if the
    /// index is an imported global.
    pub fn local_global_index(&self, global: GlobalIndex) -> Option<LocalGlobalIndex> {
        global
            .index()
            .checked_sub(self.import_counts.globals)
            .map(LocalGlobalIndex::new)
    }

    /// Test whether the given global index is for an imported global.
    pub fn is_imported_global(&self, index: GlobalIndex) -> bool {
        index.index() < self.import_counts.globals
    }

    /// Get the Module name
    pub fn name(&self) -> String {
        match self.name {
            Some(ref name) => name.to_string(),
            None => "<module>".to_string(),
        }
    }

    /// Get the imported function types of the module.
    pub fn imported_function_types<'a>(&'a self) -> impl Iterator<Item = FunctionType> + 'a {
        self.functions
            .values()
            .take(self.import_counts.functions)
            .map(move |sig_index| self.signatures[*sig_index].clone())
    }
}

// impl ArchivedArchivableModuleInfo {
//     /// Get the import types of the module
//     pub fn imports<'a>(&'a self) -> ImportsIterator<'a> {
//         let iter = self.imports.iter().map(move |((m, f, i), index)| {
//             let extern_type = match index {
//                 rkyv::Archived::<ImportIndex>::Function(i) => {
//                     let signature = &self.functions[i];
//                     let func_type: FunctionTypeRef<'_> = self.signatures[signature].into();
//                     ExternType::Function(func_type)
//                 }
//                 rkyv::Archived::<ImportIndex>::Table(i) => {
//                     let table_type = unrkyv(&self.tables[i]);
//                     ExternType::Table(table_type)
//                 }
//                 rkyv::Archived::<ImportIndex>::Memory(i) => {
//                     let memory_type = unrkyv(&self.memories[i]);
//                     ExternType::Memory(memory_type)
//                 }
//                 rkyv::Archived::<ImportIndex>::Global(i) => {
//                     let global_type = unrkyv(&self.globals[i]);
//                     ExternType::Global(global_type)
//                 }
//             };
//             (unrkyv(index), Import::new(m.as_str(), f.as_str(), *i, extern_type))
//         });
//         ImportsIterator {
//             iter: Box::new(iter),
//         }
//     }
// }

impl fmt::Display for ModuleInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// Code inspired from
// https://www.reddit.com/r/rust/comments/9vspv4/extending_iterators_ergonomically/

/// This iterator allows us to iterate over the exports
/// and offer nice API ergonomics over it.
pub struct ExportsIterator<I: Iterator<Item = ExportType> + Sized> {
    iter: I,
    size: usize,
}

impl<I: Iterator<Item = ExportType> + Sized> ExportsIterator<I> {
    /// Create a new `ExportsIterator` for a given iterator and size
    pub fn new(iter: I, size: usize) -> Self {
        Self { iter, size }
    }
}

impl<I: Iterator<Item = ExportType> + Sized> ExactSizeIterator for ExportsIterator<I> {
    // We can easily calculate the remaining number of iterations.
    fn len(&self) -> usize {
        self.size
    }
}

impl<I: Iterator<Item = ExportType> + Sized> ExportsIterator<I> {
    /// Get only the functions
    pub fn functions(self) -> impl Iterator<Item = ExportType<FunctionType>> + Sized {
        self.iter.filter_map(|extern_| match extern_.ty() {
            ExternType::Function(ty) => Some(ExportType::new(extern_.name(), ty.clone())),
            _ => None,
        })
    }
    /// Get only the memories
    pub fn memories(self) -> impl Iterator<Item = ExportType<MemoryType>> + Sized {
        self.iter.filter_map(|extern_| match extern_.ty() {
            ExternType::Memory(ty) => Some(ExportType::new(extern_.name(), *ty)),
            _ => None,
        })
    }
    /// Get only the tables
    pub fn tables(self) -> impl Iterator<Item = ExportType<TableType>> + Sized {
        self.iter.filter_map(|extern_| match extern_.ty() {
            ExternType::Table(ty) => Some(ExportType::new(extern_.name(), *ty)),
            _ => None,
        })
    }
    /// Get only the globals
    pub fn globals(self) -> impl Iterator<Item = ExportType<GlobalType>> + Sized {
        self.iter.filter_map(|extern_| match extern_.ty() {
            ExternType::Global(ty) => Some(ExportType::new(extern_.name(), *ty)),
            _ => None,
        })
    }
}

impl<I: Iterator<Item = ExportType> + Sized> Iterator for ExportsIterator<I> {
    type Item = ExportType;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

/// This iterator allows us to iterate over the imports
/// and offer nice API ergonomics over it.
pub struct ImportsIterator<'a> {
    iter: Box<dyn ExactSizeIterator<Item = (ImportIndex, Import<&'a str, ExternType>)> + 'a>,
}

impl<'a> ExactSizeIterator for ImportsIterator<'a> {
    // We can easily calculate the remaining number of iterations.
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a> ImportsIterator<'a> {
    /// Get only the functions
    pub fn functions(self) -> impl Iterator<Item = Import<&'a str, FunctionType>> + Sized {
        std::iter::empty()
        // self.iter.filter_map(|extern_| match extern_.ty() {
        //     ExternType::Function(ty) => Some(Import::new(
        //         extern_.module(),
        //         extern_.name(),
        //         ty.clone(),
        //     )),
        //     _ => None,
        // })
    }
    /// Get only the memories
    pub fn memories(self) -> impl Iterator<Item = Import<&'a str, MemoryType>> + Sized {
        std::iter::empty()
        // self.iter.filter_map(|extern_| match extern_.ty() {
        //     ExternType::Memory(ty) => Some(Import::new(extern_.module(), extern_.name(), *ty)),
        //     _ => None,
        // })
    }
    /// Get only the tables
    pub fn tables(self) -> impl Iterator<Item = Import<&'a str, TableType>> + Sized {
        std::iter::empty()
        // self.iter.filter_map(|extern_| match extern_.ty() {
        //     ExternType::Table(ty) => Some(Import::new(extern_.module(), extern_.name(), *ty)),
        //     _ => None,
        // })
    }
    /// Get only the globals
    pub fn globals(self) -> impl Iterator<Item = Import<&'a str, GlobalType>> + Sized {
        std::iter::empty()
        // self.iter.filter_map(|extern_| match extern_.ty() {
        //     ExternType::Global(ty) => Some(Import::new(extern_.module(), extern_.name(), *ty)),
        //     _ => None,
        // })
    }
}

impl<'a> Iterator for ImportsIterator<'a> {
    type Item = (ImportIndex, Import<&'a str, ExternType>);
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}
