use borsh::{BorshDeserialize, BorshSerialize};
use serde::de::{Deserializer, Visitor};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use std::fmt;
use wasmer_compiler::CompiledFunctionFrameInfo;

#[derive(Clone, Deserialize, Serialize, BorshDeserialize, BorshSerialize)]
/// Just a CompiledFunctionFrameInfo, used in serialization
pub struct SerializableFunctionFrameInfo {
    /// frame info for serialization
    pub frame_info: CompiledFunctionFrameInfo
}
