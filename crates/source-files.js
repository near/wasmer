var N = null;var sourcesIndex = {};
sourcesIndex["test_generator"] = {"name":"","files":["lib.rs","processors.rs"]};
sourcesIndex["wasmer"] = {"name":"","dirs":[{"name":"externals","files":["function.rs","global.rs","memory.rs","mod.rs","table.rs"]}],"files":["env.rs","exports.rs","import_object.rs","instance.rs","lib.rs","module.rs","native.rs","ptr.rs","store.rs","tunables.rs","types.rs","utils.rs"]};
sourcesIndex["wasmer_c_api"] = {"name":"","dirs":[{"name":"deprecated","dirs":[{"name":"import","files":["mod.rs","wasi.rs"]}],"files":["export.rs","global.rs","instance.rs","memory.rs","mod.rs","module.rs","table.rs","value.rs"]},{"name":"wasm_c_api","dirs":[{"name":"externals","files":["function.rs","global.rs","memory.rs","mod.rs","table.rs"]},{"name":"types","files":["export.rs","extern_.rs","frame.rs","function.rs","global.rs","import.rs","memory.rs","mod.rs","mutability.rs","table.rs","value.rs"]},{"name":"unstable","files":["engine.rs","mod.rs","module.rs","target_lexicon.rs"]},{"name":"wasi","files":["capture_files.rs","mod.rs"]}],"files":["engine.rs","instance.rs","macros.rs","mod.rs","module.rs","store.rs","trap.rs","value.rs","version.rs","wat.rs"]}],"files":["error.rs","lib.rs","ordered_resolver.rs"]};
sourcesIndex["wasmer_cache"] = {"name":"","files":["cache.rs","filesystem.rs","hash.rs","lib.rs"]};
sourcesIndex["wasmer_cli"] = {"name":"","dirs":[{"name":"c_gen","files":["mod.rs","object_file_header.rs"]},{"name":"commands","dirs":[{"name":"run","files":["wasi.rs"]}],"files":["cache.rs","config.rs","inspect.rs","run.rs","self_update.rs","validate.rs","wast.rs"]}],"files":["cli.rs","commands.rs","common.rs","error.rs","lib.rs","store.rs","suggestions.rs","utils.rs"]};
sourcesIndex["wasmer_compiler"] = {"name":"","dirs":[{"name":"translator","files":["environ.rs","error.rs","middleware.rs","mod.rs","module.rs","sections.rs","state.rs"]}],"files":["address_map.rs","compiler.rs","error.rs","function.rs","jump_table.rs","lib.rs","module.rs","relocation.rs","section.rs","sourceloc.rs","target.rs","trap.rs","unwind.rs"]};
sourcesIndex["wasmer_compiler_cranelift"] = {"name":"","dirs":[{"name":"debug","files":["address_map.rs","mod.rs"]},{"name":"trampoline","files":["dynamic_function.rs","function_call.rs","mod.rs"]},{"name":"translator","files":["code_translator.rs","func_environ.rs","func_state.rs","func_translator.rs","mod.rs","translation_utils.rs","unwind.rs"]}],"files":["address_map.rs","compiler.rs","config.rs","dwarf.rs","func_environ.rs","lib.rs","sink.rs"]};
sourcesIndex["wasmer_compiler_llvm"] = {"name":"","dirs":[{"name":"abi","files":["aarch64_systemv.rs","mod.rs","x86_64_systemv.rs"]},{"name":"trampoline","files":["mod.rs","wasm.rs"]},{"name":"translator","files":["code.rs","intrinsics.rs","mod.rs","state.rs"]}],"files":["compiler.rs","config.rs","lib.rs","object_file.rs"]};
sourcesIndex["wasmer_compiler_singlepass"] = {"name":"","files":["address_map.rs","codegen_x64.rs","common_decl.rs","compiler.rs","config.rs","emitter_x64.rs","lib.rs","machine.rs","x64_decl.rs"]};
sourcesIndex["wasmer_derive"] = {"name":"","files":["lib.rs","parse.rs"]};
sourcesIndex["wasmer_emscripten"] = {"name":"","dirs":[{"name":"env","dirs":[{"name":"unix","files":["mod.rs"]}],"files":["mod.rs"]},{"name":"io","files":["mod.rs","unix.rs"]},{"name":"syscalls","files":["mod.rs","unix.rs"]}],"files":["bitwise.rs","emscripten_target.rs","errno.rs","exception.rs","exec.rs","exit.rs","inet.rs","jmp.rs","lib.rs","libc.rs","linking.rs","lock.rs","macros.rs","math.rs","memory.rs","process.rs","pthread.rs","ptr.rs","signal.rs","storage.rs","time.rs","ucontext.rs","unistd.rs","utils.rs","varargs.rs"]};
sourcesIndex["wasmer_engine"] = {"name":"","dirs":[{"name":"trap","files":["error.rs","frame_info.rs","mod.rs"]}],"files":["artifact.rs","engine.rs","error.rs","export.rs","lib.rs","resolver.rs","serialize.rs","tunables.rs"]};
sourcesIndex["wasmer_engine_dummy"] = {"name":"","files":["artifact.rs","engine.rs","lib.rs"]};
sourcesIndex["wasmer_engine_jit"] = {"name":"","dirs":[{"name":"unwind","files":["mod.rs","systemv.rs"]}],"files":["artifact.rs","builder.rs","code_memory.rs","engine.rs","lib.rs","link.rs","serialize.rs"]};
sourcesIndex["wasmer_engine_native"] = {"name":"","files":["artifact.rs","builder.rs","engine.rs","lib.rs","serialize.rs"]};
sourcesIndex["wasmer_engine_object_file"] = {"name":"","files":["artifact.rs","builder.rs","engine.rs","lib.rs","serialize.rs"]};
sourcesIndex["wasmer_integration_tests_cli"] = {"name":"","files":["assets.rs","lib.rs","link_code.rs","util.rs"]};
sourcesIndex["wasmer_middlewares"] = {"name":"","files":["lib.rs","metering.rs"]};
sourcesIndex["wasmer_object"] = {"name":"","files":["error.rs","lib.rs","module.rs"]};
sourcesIndex["wasmer_types"] = {"name":"","files":["features.rs","indexes.rs","initializers.rs","lib.rs","memory_view.rs","native.rs","ref.rs","types.rs","units.rs","values.rs"]};
sourcesIndex["wasmer_vm"] = {"name":"","dirs":[{"name":"instance","files":["allocator.rs","mod.rs","ref.rs"]},{"name":"trap","files":["mod.rs","trapcode.rs","traphandlers.rs"]}],"files":["export.rs","global.rs","imports.rs","lib.rs","libcalls.rs","memory.rs","mmap.rs","module.rs","probestack.rs","sig_registry.rs","table.rs","vmcontext.rs","vmoffsets.rs"]};
sourcesIndex["wasmer_wasi"] = {"name":"","dirs":[{"name":"state","files":["builder.rs","mod.rs","types.rs"]},{"name":"syscalls","dirs":[{"name":"legacy","files":["mod.rs","snapshot0.rs"]},{"name":"unix","files":["mod.rs"]}],"files":["mod.rs","types.rs"]}],"files":["lib.rs","macros.rs","ptr.rs","utils.rs"]};
sourcesIndex["wasmer_wasi_experimental_io_devices"] = {"name":"","files":["lib.rs","util.rs"]};
sourcesIndex["wasmer_wast"] = {"name":"","files":["error.rs","lib.rs","spectest.rs","wasi_wast.rs","wast.rs"]};
createSourceSidebar();
