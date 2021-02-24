(function() {var implementors = {};
implementors["wasmer"] = [{"text":"impl&lt;T:&nbsp;Clone&gt; Clone for LazyInit&lt;T&gt;","synthetic":false,"types":[]},{"text":"impl Clone for Exports","synthetic":false,"types":[]},{"text":"impl Clone for WasmFunctionDefinition","synthetic":false,"types":[]},{"text":"impl Clone for HostFunctionDefinition","synthetic":false,"types":[]},{"text":"impl Clone for FunctionDefinition","synthetic":false,"types":[]},{"text":"impl Clone for Function","synthetic":false,"types":[]},{"text":"impl Clone for DynamicFunctionWithoutEnv","synthetic":false,"types":[]},{"text":"impl&lt;Env:&nbsp;Sized + Clone + 'static + Send + Sync&gt; Clone for DynamicFunctionWithEnv&lt;Env&gt;","synthetic":false,"types":[]},{"text":"impl&lt;Args:&nbsp;Clone, Rets:&nbsp;Clone&gt; Clone for Function&lt;Args, Rets&gt;","synthetic":false,"types":[]},{"text":"impl Clone for Global","synthetic":false,"types":[]},{"text":"impl Clone for Memory","synthetic":false,"types":[]},{"text":"impl Clone for Table","synthetic":false,"types":[]},{"text":"impl Clone for Extern","synthetic":false,"types":[]},{"text":"impl Clone for ImportObject","synthetic":false,"types":[]},{"text":"impl Clone for Instance","synthetic":false,"types":[]},{"text":"impl Clone for Module","synthetic":false,"types":[]},{"text":"impl&lt;Args:&nbsp;Clone, Rets:&nbsp;Clone&gt; Clone for NativeFunc&lt;Args, Rets&gt;","synthetic":false,"types":[]},{"text":"impl&lt;T:&nbsp;Copy, Ty&gt; Clone for WasmPtr&lt;T, Ty&gt;","synthetic":false,"types":[]},{"text":"impl Clone for Store","synthetic":false,"types":[]},{"text":"impl Clone for BaseTunables","synthetic":false,"types":[]}];
implementors["wasmer_c_api"] = [{"text":"impl Clone for wasmer_export_t","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_export_func_t","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_exports_t","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_export_descriptor_t","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_export_descriptors_t","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_import_export_value","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_import_export_kind","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_global_descriptor_t","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_global_t","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_import_func_t","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_import_descriptor_t","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_import_descriptors_t","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_import_object_iter_t","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_memory_t","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_table_t","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_value_tag","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_value","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_value_t","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_compiler_t","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_engine_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_extern_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_extern_vec_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_exporttype_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_exporttype_vec_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_externtype_t","synthetic":false,"types":[]},{"text":"impl Clone for ExternTypeConversionError","synthetic":false,"types":[]},{"text":"impl Clone for wasm_frame_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_frame_vec_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_functype_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_functype_vec_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_globaltype_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_globaltype_vec_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_importtype_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_importtype_vec_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_memorytype_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_memorytype_vec_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_limits_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_mutability_enum","synthetic":false,"types":[]},{"text":"impl Clone for wasm_tabletype_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_tabletype_vec_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_valkind_enum","synthetic":false,"types":[]},{"text":"impl Clone for wasm_valtype_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_valtype_vec_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_byte_vec_t","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_named_extern_t","synthetic":false,"types":[]},{"text":"impl Clone for wasmer_named_extern_vec_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_val_inner","synthetic":false,"types":[]},{"text":"impl Clone for wasm_val_vec_t","synthetic":false,"types":[]},{"text":"impl Clone for wasm_val_t","synthetic":false,"types":[]},{"text":"impl Clone for wasi_version_t","synthetic":false,"types":[]}];
implementors["wasmer_cache"] = [{"text":"impl Clone for Hash","synthetic":false,"types":[]}];
implementors["wasmer_cli"] = [{"text":"impl Clone for Wasi","synthetic":false,"types":[]},{"text":"impl Clone for Run","synthetic":false,"types":[]},{"text":"impl Clone for WasmFeatures","synthetic":false,"types":[]},{"text":"impl Clone for CType","synthetic":false,"types":[]},{"text":"impl Clone for CStatement","synthetic":false,"types":[]},{"text":"impl Clone for StoreOptions","synthetic":false,"types":[]},{"text":"impl Clone for CompilerOptions","synthetic":false,"types":[]},{"text":"impl Clone for EngineType","synthetic":false,"types":[]}];
implementors["wasmer_compiler"] = [{"text":"impl Clone for InstructionAddressMap","synthetic":false,"types":[]},{"text":"impl Clone for FunctionAddressMap","synthetic":false,"types":[]},{"text":"impl Clone for Symbol","synthetic":false,"types":[]},{"text":"impl Clone for CompiledFunctionFrameInfo","synthetic":false,"types":[]},{"text":"impl Clone for FunctionBody","synthetic":false,"types":[]},{"text":"impl Clone for CompiledFunction","synthetic":false,"types":[]},{"text":"impl Clone for Dwarf","synthetic":false,"types":[]},{"text":"impl Clone for JumpTable","synthetic":false,"types":[]},{"text":"impl Clone for RelocationKind","synthetic":false,"types":[]},{"text":"impl Clone for Relocation","synthetic":false,"types":[]},{"text":"impl Clone for RelocationTarget","synthetic":false,"types":[]},{"text":"impl Clone for CpuFeature","synthetic":false,"types":[]},{"text":"impl Clone for Target","synthetic":false,"types":[]},{"text":"impl Clone for TrapInformation","synthetic":false,"types":[]},{"text":"impl Clone for CompiledFunctionUnwindInfo","synthetic":false,"types":[]},{"text":"impl Clone for SectionIndex","synthetic":false,"types":[]},{"text":"impl Clone for CustomSectionProtection","synthetic":false,"types":[]},{"text":"impl Clone for CustomSection","synthetic":false,"types":[]},{"text":"impl Clone for SectionBody","synthetic":false,"types":[]},{"text":"impl Clone for SourceLoc","synthetic":false,"types":[]}];
implementors["wasmer_compiler_cranelift"] = [{"text":"impl Clone for CraneliftOptLevel","synthetic":false,"types":[]},{"text":"impl Clone for Cranelift","synthetic":false,"types":[]},{"text":"impl Clone for ModuleInfoMemoryOffset","synthetic":false,"types":[]},{"text":"impl Clone for ModuleInfoVmctxInfo","synthetic":false,"types":[]},{"text":"impl Clone for WriterRelocate","synthetic":false,"types":[]},{"text":"impl Clone for GlobalVariable","synthetic":false,"types":[]},{"text":"impl Clone for ReturnMode","synthetic":false,"types":[]}];
implementors["wasmer_compiler_llvm"] = [{"text":"impl Clone for CompiledKind","synthetic":false,"types":[]},{"text":"impl Clone for LLVM","synthetic":false,"types":[]},{"text":"impl Clone for ElfSectionIndex","synthetic":false,"types":[]},{"text":"impl&lt;'ctx&gt; Clone for MemoryCache&lt;'ctx&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'ctx&gt; Clone for GlobalCache&lt;'ctx&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'ctx&gt; Clone for FunctionCache&lt;'ctx&gt;","synthetic":false,"types":[]},{"text":"impl Clone for ExtraInfo","synthetic":false,"types":[]}];
implementors["wasmer_compiler_singlepass"] = [{"text":"impl Clone for TrapTable","synthetic":false,"types":[]},{"text":"impl Clone for FloatValue","synthetic":false,"types":[]},{"text":"impl Clone for CanonicalizeType","synthetic":false,"types":[]},{"text":"impl Clone for IfElseState","synthetic":false,"types":[]},{"text":"impl Clone for RegisterIndex","synthetic":false,"types":[]},{"text":"impl Clone for WasmAbstractValue","synthetic":false,"types":[]},{"text":"impl Clone for MachineState","synthetic":false,"types":[]},{"text":"impl Clone for MachineStateDiff","synthetic":false,"types":[]},{"text":"impl Clone for MachineValue","synthetic":false,"types":[]},{"text":"impl Clone for FunctionStateMap","synthetic":false,"types":[]},{"text":"impl Clone for SuspendOffset","synthetic":false,"types":[]},{"text":"impl Clone for OffsetInfo","synthetic":false,"types":[]},{"text":"impl Clone for Singlepass","synthetic":false,"types":[]},{"text":"impl Clone for Location","synthetic":false,"types":[]},{"text":"impl Clone for Condition","synthetic":false,"types":[]},{"text":"impl Clone for Size","synthetic":false,"types":[]},{"text":"impl Clone for XMMOrMemory","synthetic":false,"types":[]},{"text":"impl Clone for GPROrMemory","synthetic":false,"types":[]},{"text":"impl Clone for GPR","synthetic":false,"types":[]},{"text":"impl Clone for XMM","synthetic":false,"types":[]},{"text":"impl Clone for X64Register","synthetic":false,"types":[]}];
implementors["wasmer_emscripten"] = [{"text":"impl Clone for EmAddrInfo","synthetic":false,"types":[]},{"text":"impl Clone for EmSockAddr","synthetic":false,"types":[]},{"text":"impl Clone for LongJumpRet","synthetic":false,"types":[]},{"text":"impl&lt;T:&nbsp;Copy, Ty&gt; Clone for WasmPtr&lt;T, Ty&gt;","synthetic":false,"types":[]},{"text":"impl Clone for EmPollFd","synthetic":false,"types":[]},{"text":"impl Clone for VarArgs","synthetic":false,"types":[]},{"text":"impl Clone for EmEnv","synthetic":false,"types":[]},{"text":"impl Clone for LibcDirWrapper","synthetic":false,"types":[]},{"text":"impl Clone for EmscriptenData","synthetic":false,"types":[]},{"text":"impl Clone for EmscriptenGlobalsData","synthetic":false,"types":[]}];
implementors["wasmer_engine"] = [{"text":"impl Clone for EngineId","synthetic":false,"types":[]},{"text":"impl Clone for Export","synthetic":false,"types":[]},{"text":"impl Clone for ExportFunction","synthetic":false,"types":[]},{"text":"impl Clone for ExportTable","synthetic":false,"types":[]},{"text":"impl Clone for ExportMemory","synthetic":false,"types":[]},{"text":"impl Clone for ExportGlobal","synthetic":false,"types":[]},{"text":"impl&lt;A, B&gt; Clone for NamedResolverChain&lt;A, B&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;A: NamedResolver + Clone,<br>&nbsp;&nbsp;&nbsp;&nbsp;B: NamedResolver + Clone,&nbsp;</span>","synthetic":false,"types":[]},{"text":"impl Clone for UnprocessedFunctionFrameInfo","synthetic":false,"types":[]},{"text":"impl Clone for SerializableFunctionFrameInfo","synthetic":false,"types":[]},{"text":"impl Clone for RuntimeError","synthetic":false,"types":[]},{"text":"impl Clone for FrameInfo","synthetic":false,"types":[]}];
implementors["wasmer_engine_dummy"] = [{"text":"impl Clone for DummyEngine","synthetic":false,"types":[]}];
implementors["wasmer_engine_jit"] = [{"text":"impl Clone for JITEngine","synthetic":false,"types":[]}];
implementors["wasmer_engine_native"] = [{"text":"impl Clone for NativeEngine","synthetic":false,"types":[]},{"text":"impl Clone for Linker","synthetic":false,"types":[]}];
implementors["wasmer_engine_object_file"] = [{"text":"impl Clone for ObjectFileEngine","synthetic":false,"types":[]}];
implementors["wasmer_integration_tests_cli"] = [{"text":"impl Clone for Compiler","synthetic":false,"types":[]},{"text":"impl Clone for Engine","synthetic":false,"types":[]}];
implementors["wasmer_middlewares"] = [{"text":"impl Clone for MeteringGlobalIndexes","synthetic":false,"types":[]}];
implementors["wasmer_vm"] = [{"text":"impl Clone for VMExportFunction","synthetic":false,"types":[]},{"text":"impl Clone for VMExportTable","synthetic":false,"types":[]},{"text":"impl Clone for VMExportMemory","synthetic":false,"types":[]},{"text":"impl Clone for VMExportGlobal","synthetic":false,"types":[]},{"text":"impl Clone for GlobalError","synthetic":false,"types":[]},{"text":"impl Clone for Imports","synthetic":false,"types":[]},{"text":"impl Clone for InstanceRef","synthetic":false,"types":[]},{"text":"impl Clone for ImportFunctionEnv","synthetic":false,"types":[]},{"text":"impl Clone for MemoryError","synthetic":false,"types":[]},{"text":"impl Clone for MemoryStyle","synthetic":false,"types":[]},{"text":"impl Clone for ModuleId","synthetic":false,"types":[]},{"text":"impl Clone for ModuleInfo","synthetic":false,"types":[]},{"text":"impl Clone for TableStyle","synthetic":false,"types":[]},{"text":"impl Clone for TrapCode","synthetic":false,"types":[]},{"text":"impl Clone for VMFunctionEnvironment","synthetic":false,"types":[]},{"text":"impl Clone for VMFunctionImport","synthetic":false,"types":[]},{"text":"impl&lt;T:&nbsp;Sized + Clone + Send + Sync&gt; Clone for VMDynamicFunctionContext&lt;T&gt;","synthetic":false,"types":[]},{"text":"impl Clone for VMFunctionKind","synthetic":false,"types":[]},{"text":"impl Clone for VMTableImport","synthetic":false,"types":[]},{"text":"impl Clone for VMMemoryImport","synthetic":false,"types":[]},{"text":"impl Clone for VMGlobalImport","synthetic":false,"types":[]},{"text":"impl Clone for VMMemoryDefinition","synthetic":false,"types":[]},{"text":"impl Clone for VMTableDefinition","synthetic":false,"types":[]},{"text":"impl Clone for VMGlobalDefinitionStorage","synthetic":false,"types":[]},{"text":"impl Clone for VMGlobalDefinition","synthetic":false,"types":[]},{"text":"impl Clone for VMSharedSignatureIndex","synthetic":false,"types":[]},{"text":"impl Clone for VMCallerCheckedAnyfunc","synthetic":false,"types":[]},{"text":"impl Clone for VMBuiltinFunctionIndex","synthetic":false,"types":[]},{"text":"impl Clone for VMOffsets","synthetic":false,"types":[]},{"text":"impl Clone for TargetSharedSignatureIndex","synthetic":false,"types":[]},{"text":"impl Clone for LibCall","synthetic":false,"types":[]},{"text":"impl Clone for FunctionBodyPtr","synthetic":false,"types":[]},{"text":"impl Clone for SectionBodyPtr","synthetic":false,"types":[]}];
implementors["wasmer_wasi"] = [{"text":"impl&lt;T:&nbsp;Copy, Ty&gt; Clone for WasmPtr&lt;T, Ty&gt;","synthetic":false,"types":[]},{"text":"impl Clone for WasiFsError","synthetic":false,"types":[]},{"text":"impl Clone for PollEvent","synthetic":false,"types":[]},{"text":"impl Clone for PollEventBuilder","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_ciovec_t","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_dirent_t","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_event_fd_readwrite_t","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_event_u","synthetic":false,"types":[]},{"text":"impl Clone for EventEnum","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_event_t","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_prestat_u_dir_t","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_prestat_u","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_prestat_t","synthetic":false,"types":[]},{"text":"impl Clone for PrestatEnum","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_fdstat_t","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_filestat_t","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_iovec_t","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_subscription_clock_t","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_subscription_fs_readwrite_t","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_subscription_u","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_subscription_t","synthetic":false,"types":[]},{"text":"impl Clone for EventType","synthetic":false,"types":[]},{"text":"impl Clone for WasiSubscription","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_subscription_clock_t","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_subscription_u","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_subscription_t","synthetic":false,"types":[]},{"text":"impl Clone for __wasi_filestat_t","synthetic":false,"types":[]},{"text":"impl Clone for WasiVersion","synthetic":false,"types":[]},{"text":"impl Clone for WasiEnv","synthetic":false,"types":[]}];
implementors["wasmer_wasi_experimental_io_devices"] = [{"text":"impl Clone for InputEvent","synthetic":false,"types":[]}];
implementors["wasmer_wast"] = [{"text":"impl&lt;'a&gt; Clone for WasiTest&lt;'a&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Clone for Envs&lt;'a&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Clone for Args&lt;'a&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Clone for Preopens&lt;'a&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Clone for MapDirs&lt;'a&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Clone for TempDirs&lt;'a&gt;","synthetic":false,"types":[]},{"text":"impl Clone for AssertReturn","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Clone for Stdin&lt;'a&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Clone for AssertStdout&lt;'a&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Clone for AssertStderr&lt;'a&gt;","synthetic":false,"types":[]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()