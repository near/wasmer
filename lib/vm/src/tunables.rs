use crate::MemoryError;
use crate::{Memory, Table};
use crate::{MemoryStyle, TableStyle};
use crate::{VMMemoryDefinition, VMTableDefinition};
use std::ptr::NonNull;
use std::sync::Arc;
use wasmer_types::{MemoryType, TableType};

/// An engine delegates the creation of memories, tables, and globals
/// to a foreign implementor of this trait.
pub trait Tunables {
    /// Construct a `MemoryStyle` for the provided `MemoryType`
    fn memory_style(&self, memory: &MemoryType) -> MemoryStyle;

    /// Construct a `TableStyle` for the provided `TableType`
    fn table_style(&self, table: &TableType) -> TableStyle;

    /// Create a memory owned by the host given a [`MemoryType`] and a [`MemoryStyle`].
    fn create_host_memory(
        &self,
        ty: &MemoryType,
        style: &MemoryStyle,
    ) -> Result<Arc<dyn Memory>, MemoryError>;

    /// Create a memory owned by the VM given a [`MemoryType`] and a [`MemoryStyle`].
    ///
    /// # Safety
    /// - `vm_definition_location` must point to a valid location in VM memory.
    unsafe fn create_vm_memory(
        &self,
        ty: &MemoryType,
        style: &MemoryStyle,
        vm_definition_location: NonNull<VMMemoryDefinition>,
    ) -> Result<Arc<dyn Memory>, MemoryError>;

    /// Create a table owned by the host given a [`TableType`] and a [`TableStyle`].
    fn create_host_table(
        &self,
        ty: &TableType,
        style: &TableStyle,
    ) -> Result<Arc<dyn Table>, String>;

    /// Create a table owned by the VM given a [`TableType`] and a [`TableStyle`].
    ///
    /// # Safety
    /// - `vm_definition_location` must point to a valid location in VM memory.
    unsafe fn create_vm_table(
        &self,
        ty: &TableType,
        style: &TableStyle,
        vm_definition_location: NonNull<VMTableDefinition>,
    ) -> Result<Arc<dyn Table>, String>;
}
