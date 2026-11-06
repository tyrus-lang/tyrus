use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_read_file_sync(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!(
                "error: `fs.readFileSync` expects 1 argument (path), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.readFileSync` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        let result_type = Type::Result {
            success: Box::new(Type::Str),
            error: Box::new(Type::Named("FsError".to_string())),
        };
        let result_llvm_type = self.llvm_type(&result_type);
        let result_slot = self
            .builder
            .build_alloca(result_llvm_type, "fs.readFileSync.abi")
            .map_err(|error| error.to_string())?;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let fn_val = self.declare_libc_fn(
            "tyrus_fs_read_file_sync_result",
            &[self.string_abi_type(), pointer.into()],
            None,
        );
        self.builder
            .build_call(fn_val, &[path.into(), result_slot.into()], "")
            .map_err(|error| error.to_string())?;
        let result = self
            .builder
            .build_load(result_llvm_type, result_slot, "fs.readFileSync.result")
            .map_err(|error| error.to_string())?;
        self.record_owned_native_result(result, result_type);
        Ok(result)
    }

    pub(super) fn emit_write_file_sync(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 2 {
            return Err(format!(
                "error: `fs.writeFileSync` expects 2 arguments (path, data), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.writeFileSync` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        let data_value = self.emit_expr(&args[1])?;
        if !data_value.is_struct_value() {
            return Err("error: `fs.writeFileSync` data must be a string".to_string());
        }
        let data = data_value.into_struct_value();
        let result_type = Type::Result {
            success: Box::new(Type::Void),
            error: Box::new(Type::Named("FsError".to_string())),
        };
        let result_llvm_type = self.llvm_type(&result_type);
        let result_slot = self
            .builder
            .build_alloca(result_llvm_type, "fs.writeFileSync.abi")
            .map_err(|error| error.to_string())?;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let fn_val = self.declare_libc_fn(
            "tyrus_fs_write_file_sync_result",
            &[
                self.string_abi_type(),
                self.string_abi_type(),
                pointer.into(),
            ],
            None,
        );
        self.builder
            .build_call(fn_val, &[path.into(), data.into(), result_slot.into()], "")
            .map_err(|error| error.to_string())?;
        let result = self
            .builder
            .build_load(result_llvm_type, result_slot, "fs.writeFileSync.result")
            .map_err(|error| error.to_string())?;
        self.record_owned_native_result(result, result_type);
        Ok(result)
    }

    pub(super) fn emit_fs_exists(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!(
                "error: `fs.exists` expects 1 argument (path), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.exists` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        let fn_val = self.declare_libc_fn(
            "tyrus_fs_exists",
            &[self.string_abi_type()],
            Some(self.context.bool_type().into()),
        );
        Ok(self
            .builder
            .build_call(fn_val, &[path.into()], "fs.exists.result")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap())
    }

    pub(super) fn emit_fs_read_file(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!(
                "error: `fs.readFile` expects 1 argument (path), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.readFile` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let fn_val = self.declare_libc_fn(
            "tyrus_fs_read_file",
            &[self.string_abi_type()],
            Some(ptr_type.into()),
        );
        let promise = self
            .builder
            .build_call(fn_val, &[path.into()], "fs.readFile.promise")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();
        self.record_owned_native_result(
            promise.into(),
            Type::Apply {
                name: "Promise".to_string(),
                args: vec![Type::Result {
                    success: Box::new(Type::Str),
                    error: Box::new(Type::Named("FsError".to_string())),
                }],
            },
        );
        Ok(promise.into())
    }

    pub(super) fn emit_fs_write_file(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 2 {
            return Err(format!(
                "error: `fs.writeFile` expects 2 arguments (path, data), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.writeFile` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        let data_value = self.emit_expr(&args[1])?;
        if !data_value.is_struct_value() {
            return Err("error: `fs.writeFile` data must be a string".to_string());
        }
        let data = data_value.into_struct_value();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let fn_val = self.declare_libc_fn(
            "tyrus_fs_write_file",
            &[self.string_abi_type(), self.string_abi_type()],
            Some(ptr_type.into()),
        );
        let promise = self
            .builder
            .build_call(fn_val, &[path.into(), data.into()], "fs.writeFile.promise")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();
        self.record_owned_native_result(
            promise.into(),
            Type::Apply {
                name: "Promise".to_string(),
                args: vec![Type::Result {
                    success: Box::new(Type::Void),
                    error: Box::new(Type::Named("FsError".to_string())),
                }],
            },
        );
        Ok(promise.into())
    }

    pub(super) fn emit_fs_file_stream(
        &mut self,
        args: &[Expr],
        readable: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let function = if readable {
            "readFileStream"
        } else {
            "writeFileStream"
        };
        if args.len() != 1 {
            return Err(format!(
                "error: `fs.{function}` expects 1 argument (path), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err(format!("error: `fs.{function}` path must be a string"));
        }
        let stream_type = Type::Apply {
            name: if readable {
                "ReadableStream".to_string()
            } else {
                "WritableStream".to_string()
            },
            args: vec![Type::Array(Box::new(Type::U8))],
        };
        let result_type = Type::Result {
            success: Box::new(stream_type),
            error: Box::new(Type::Named("FsError".to_string())),
        };
        let symbol = if readable {
            "tyrus_fs_read_file_stream_result"
        } else {
            "tyrus_fs_write_file_stream_result"
        };
        self.emit_fs_result_call(
            symbol,
            &[self.string_abi_type()],
            &[path_value.into_struct_value().into()],
            result_type,
            "fs.file_stream",
        )
    }

    pub(super) fn emit_fs_append_file(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 2 {
            return Err(format!(
                "error: `fs.appendFile` expects 2 arguments (path, data), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.appendFile` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        let data_value = self.emit_expr(&args[1])?;
        if !data_value.is_struct_value() {
            return Err("error: `fs.appendFile` data must be a string".to_string());
        }
        let data = data_value.into_struct_value();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let fn_val = self.declare_libc_fn(
            "tyrus_fs_append_file",
            &[self.string_abi_type(), self.string_abi_type()],
            Some(ptr_type.into()),
        );
        let promise = self
            .builder
            .build_call(fn_val, &[path.into(), data.into()], "fs.appendFile.promise")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();
        self.record_owned_native_result(
            promise.into(),
            Type::Apply {
                name: "Promise".to_string(),
                args: vec![Type::Result {
                    success: Box::new(Type::Void),
                    error: Box::new(Type::Named("FsError".to_string())),
                }],
            },
        );
        Ok(promise.into())
    }

    pub(super) fn emit_fs_readdir(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.is_empty() || args.len() > 2 {
            return Err(format!(
                "error: `fs.readdir` expects 1 or 2 arguments (path, options?), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.readdir` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());

        let options_ptr = if args.len() == 2 {
            let options_type = Type::Named("ReaddirOptions".to_string());
            let options = self.emit_expr_as(&args[1], &options_type)?;
            let options_slot = self
                .builder
                .build_alloca(self.llvm_type(&options_type), "fs.readdir.options")
                .unwrap();
            self.builder.build_store(options_slot, options).unwrap();
            Some(options_slot)
        } else {
            None
        };

        let options_arg = match &options_ptr {
            Some(slot) => (*slot).into(),
            None => self
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .const_null()
                .into(),
        };
        let result_type = Type::Result {
            success: Box::new(Type::Array(Box::new(Type::Named("DirEntry".to_string())))),
            error: Box::new(Type::Named("FsError".to_string())),
        };
        self.emit_fs_result_call(
            "tyrus_fs_readdir_result",
            &[self.string_abi_type(), ptr_type.into()],
            &[path.into(), options_arg],
            result_type,
            "fs.readdir.result",
        )
    }

    pub(super) fn emit_fs_mkdir(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!(
                "error: `fs.mkdir` expects 1 argument (path), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.mkdir` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        let result_type = Type::Result {
            success: Box::new(Type::Void),
            error: Box::new(Type::Named("FsError".to_string())),
        };
        let result_llvm_type = self.llvm_type(&result_type);
        let result_slot = self
            .builder
            .build_alloca(result_llvm_type, "fs.mkdir.abi")
            .map_err(|error| error.to_string())?;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let fn_val = self.declare_libc_fn(
            "tyrus_fs_mkdir_result",
            &[self.string_abi_type(), pointer.into()],
            None,
        );
        self.builder
            .build_call(fn_val, &[path.into(), result_slot.into()], "")
            .map_err(|error| error.to_string())?;
        let result = self
            .builder
            .build_load(result_llvm_type, result_slot, "fs.mkdir.result")
            .map_err(|error| error.to_string())?;
        self.record_owned_native_result(result, result_type);
        Ok(result)
    }

    pub(super) fn emit_fs_rm(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.is_empty() || args.len() > 2 {
            return Err(format!(
                "error: `fs.rm` expects 1 or 2 arguments (path, options?), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.rm` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());

        let options_ptr = if args.len() == 2 {
            let options_type = Type::Named("RmOptions".to_string());
            let options = self.emit_expr_as(&args[1], &options_type)?;
            let options_slot = self
                .builder
                .build_alloca(self.llvm_type(&options_type), "fs.rm.options")
                .unwrap();
            self.builder.build_store(options_slot, options).unwrap();
            Some(options_slot)
        } else {
            None
        };

        let options_arg = match &options_ptr {
            Some(slot) => (*slot).into(),
            None => ptr_type.const_null().into(),
        };
        self.emit_fs_void_result_call(
            "tyrus_fs_rm_result",
            &[self.string_abi_type(), ptr_type.into()],
            &[path.into(), options_arg],
            "fs.rm",
        )
    }

    pub(super) fn emit_fs_cp(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() < 2 || args.len() > 3 {
            return Err(format!(
                "error: `fs.cp` expects 2 or 3 arguments (src, dest, options?), found {}",
                args.len()
            ));
        }
        let src_value = self.emit_expr(&args[0])?;
        if !src_value.is_struct_value() {
            return Err("error: `fs.cp` src must be a string".to_string());
        }
        let src = src_value.into_struct_value();
        let dest_value = self.emit_expr(&args[1])?;
        if !dest_value.is_struct_value() {
            return Err("error: `fs.cp` dest must be a string".to_string());
        }
        let dest = dest_value.into_struct_value();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());

        let options_ptr = if args.len() == 3 {
            let options_type = Type::Named("CpOptions".to_string());
            let options = self.emit_expr_as(&args[2], &options_type)?;
            let options_slot = self
                .builder
                .build_alloca(self.llvm_type(&options_type), "fs.cp.options")
                .unwrap();
            self.builder.build_store(options_slot, options).unwrap();
            Some(options_slot)
        } else {
            None
        };

        let options_arg = match &options_ptr {
            Some(slot) => (*slot).into(),
            None => ptr_type.const_null().into(),
        };
        self.emit_fs_void_result_call(
            "tyrus_fs_cp_result",
            &[
                self.string_abi_type(),
                self.string_abi_type(),
                ptr_type.into(),
            ],
            &[src.into(), dest.into(), options_arg],
            "fs.cp",
        )
    }

    pub(super) fn emit_fs_mv(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 2 {
            return Err(format!(
                "error: `fs.mv` expects 2 arguments (src, dest), found {}",
                args.len()
            ));
        }
        let src_value = self.emit_expr(&args[0])?;
        if !src_value.is_struct_value() {
            return Err("error: `fs.mv` src must be a string".to_string());
        }
        let src = src_value.into_struct_value();
        let dest_value = self.emit_expr(&args[1])?;
        if !dest_value.is_struct_value() {
            return Err("error: `fs.mv` dest must be a string".to_string());
        }
        let dest = dest_value.into_struct_value();
        self.emit_fs_void_result_call(
            "tyrus_fs_mv_result",
            &[self.string_abi_type(), self.string_abi_type()],
            &[src.into(), dest.into()],
            "fs.mv",
        )
    }

    pub(super) fn emit_fs_remove_all(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!(
                "error: `fs.removeAll` expects 1 argument (path), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.removeAll` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        self.emit_fs_void_result_call(
            "tyrus_fs_remove_all_result",
            &[self.string_abi_type()],
            &[path.into()],
            "fs.removeAll",
        )
    }

    pub(super) fn emit_fs_stat(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!(
                "error: `fs.stat` expects 1 argument (path), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.stat` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        self.emit_fs_result_call(
            "tyrus_fs_stat_result",
            &[self.string_abi_type()],
            &[path.into()],
            Type::Result {
                success: Box::new(Type::Named("FileStats".to_string())),
                error: Box::new(Type::Named("FsError".to_string())),
            },
            "fs.stat.result",
        )
    }

    pub(super) fn emit_fs_metadata(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!(
                "error: `fs.metadata` expects 1 argument (path), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.metadata` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        self.emit_fs_result_call(
            "tyrus_fs_metadata_result",
            &[self.string_abi_type()],
            &[path.into()],
            Type::Result {
                success: Box::new(Type::Named("Metadata".to_string())),
                error: Box::new(Type::Named("FsError".to_string())),
            },
            "fs.metadata.result",
        )
    }

    pub(super) fn emit_fs_disk_usage(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!(
                "error: `fs.diskUsage` expects 1 argument (path), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.diskUsage` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        let result_type = Type::Result {
            success: Box::new(Type::Named("DiskUsage".to_string())),
            error: Box::new(Type::Named("FsError".to_string())),
        };
        self.emit_fs_result_call(
            "tyrus_fs_disk_usage_result",
            &[self.string_abi_type()],
            &[path.into()],
            result_type,
            "fs.diskUsage.result",
        )
    }

    pub(super) fn emit_fs_mime(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!(
                "error: `fs.mime` expects 1 argument (path), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.mime` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        let fn_val = self.declare_libc_fn(
            "tyrus_fs_mime",
            &[self.string_abi_type()],
            Some(
                self.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
            ),
        );
        let result = self
            .builder
            .build_call(fn_val, &[path.into()], "fs.mime.result")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap();
        self.record_owned_native_result(result, Type::Optional(Box::new(Type::Str)));
        Ok(result)
    }

    pub(super) fn emit_fs_chmod(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 2 {
            return Err(format!(
                "error: `fs.chmod` expects 2 arguments (path, mode), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.chmod` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        let mode_value = self.emit_expr_as(&args[1], &Type::Named("FileMode".to_string()))?;
        let mode = mode_value.into_int_value();
        self.emit_fs_void_result_call(
            "tyrus_fs_chmod_result",
            &[self.string_abi_type(), self.context.i32_type().into()],
            &[path.into(), mode.into()],
            "fs.chmod.result",
        )
    }

    pub(super) fn emit_fs_chown(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 3 {
            return Err(format!(
                "error: `fs.chown` expects 3 arguments (path, uid, gid), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.chown` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        let uid_value = self.emit_expr_as(&args[1], &Type::U32)?;
        let uid = uid_value.into_int_value();
        let gid_value = self.emit_expr_as(&args[2], &Type::U32)?;
        let gid = gid_value.into_int_value();
        self.emit_fs_void_result_call(
            "tyrus_fs_chown_result",
            &[
                self.string_abi_type(),
                self.context.i32_type().into(),
                self.context.i32_type().into(),
            ],
            &[path.into(), uid.into(), gid.into()],
            "fs.chown.result",
        )
    }

    pub(super) fn emit_fs_readlink(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!(
                "error: `fs.readlink` expects 1 argument (path), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.readlink` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        self.emit_fs_result_call(
            "tyrus_fs_readlink_result",
            &[self.string_abi_type()],
            &[path.into()],
            Type::Result {
                success: Box::new(Type::Str),
                error: Box::new(Type::Named("FsError".to_string())),
            },
            "fs.readlink.result",
        )
    }

    pub(super) fn emit_fs_symlink(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 2 {
            return Err(format!(
                "error: `fs.symlink` expects 2 arguments (original, link), found {}",
                args.len()
            ));
        }
        let original = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
        let link = self.emit_expr_as(&args[1], &Type::Str)?.into_struct_value();
        self.emit_fs_void_result_call(
            "tyrus_fs_symlink_result",
            &[self.string_abi_type(), self.string_abi_type()],
            &[original.into(), link.into()],
            "fs.symlink.result",
        )
    }

    pub(super) fn emit_fs_symlink_dir(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 2 {
            return Err(format!(
                "error: `fs.symlinkDir` expects 2 arguments (original, link), found {}",
                args.len()
            ));
        }
        let original = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
        let link = self.emit_expr_as(&args[1], &Type::Str)?.into_struct_value();
        self.emit_fs_void_result_call(
            "tyrus_fs_symlink_dir_result",
            &[self.string_abi_type(), self.string_abi_type()],
            &[original.into(), link.into()],
            "fs.symlink_dir.result",
        )
    }

    pub(super) fn emit_fs_hard_link(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 2 {
            return Err(format!(
                "error: `fs.hardLink` expects 2 arguments (original, link), found {}",
                args.len()
            ));
        }
        let original = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
        let link = self.emit_expr_as(&args[1], &Type::Str)?.into_struct_value();
        self.emit_fs_void_result_call(
            "tyrus_fs_hard_link_result",
            &[self.string_abi_type(), self.string_abi_type()],
            &[original.into(), link.into()],
            "fs.hardLink.result",
        )
    }

    pub(super) fn emit_fs_canonicalize(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!(
                "error: `fs.canonicalize` expects 1 argument (path), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.canonicalize` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        self.emit_fs_result_call(
            "tyrus_fs_canonicalize_result",
            &[self.string_abi_type()],
            &[path.into()],
            Type::Result {
                success: Box::new(Type::Str),
                error: Box::new(Type::Named("FsError".to_string())),
            },
            "fs.canonicalize.result",
        )
    }

    pub(super) fn emit_fs_glob(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!(
                "error: `fs.glob` expects 1 argument (pattern), found {}",
                args.len()
            ));
        }
        let pattern_value = self.emit_expr(&args[0])?;
        if !pattern_value.is_struct_value() {
            return Err("error: `fs.glob` pattern must be a string".to_string());
        }
        let pattern = pattern_value.into_struct_value();
        self.emit_fs_result_call(
            "tyrus_fs_glob_result",
            &[self.string_abi_type()],
            &[pattern.into()],
            Type::Result {
                success: Box::new(Type::Array(Box::new(Type::Str))),
                error: Box::new(Type::Named("FsError".to_string())),
            },
            "fs.glob.result",
        )
    }

    pub(super) fn emit_fs_search(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.is_empty() || args.len() > 2 {
            return Err(format!(
                "error: `fs.search` expects 1 or 2 arguments (pattern, options?), found {}",
                args.len()
            ));
        }
        let pattern_value = self.emit_expr(&args[0])?;
        if !pattern_value.is_struct_value() {
            return Err("error: `fs.search` pattern must be a string".to_string());
        }
        let pattern = pattern_value.into_struct_value();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());

        let options_ptr = if args.len() == 2 {
            let options_type = Type::Named("SearchOptions".to_string());
            let options = self.emit_expr_as(&args[1], &options_type)?;
            let options_slot = self
                .builder
                .build_alloca(self.llvm_type(&options_type), "fs.search.options")
                .unwrap();
            self.builder.build_store(options_slot, options).unwrap();
            Some(options_slot)
        } else {
            None
        };

        let options_arg = match &options_ptr {
            Some(slot) => (*slot).into(),
            None => ptr_type.const_null().into(),
        };
        self.emit_fs_result_call(
            "tyrus_fs_search_result",
            &[self.string_abi_type(), ptr_type.into()],
            &[pattern.into(), options_arg],
            Type::Result {
                success: Box::new(Type::Array(Box::new(Type::Str))),
                error: Box::new(Type::Named("FsError".to_string())),
            },
            "fs.search.result",
        )
    }

    pub(super) fn emit_fs_diff(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 2 {
            return Err(format!(
                "error: `fs.diff` expects 2 arguments (path1, path2), found {}",
                args.len()
            ));
        }
        let path1_value = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
        let path2_value = self.emit_expr_as(&args[1], &Type::Str)?.into_struct_value();
        self.emit_fs_result_call(
            "tyrus_fs_diff_result",
            &[self.string_abi_type(), self.string_abi_type()],
            &[path1_value.into(), path2_value.into()],
            Type::Result {
                success: Box::new(Type::Named("DiffResult".to_string())),
                error: Box::new(Type::Named("FsError".to_string())),
            },
            "fs.diff.result",
        )
    }

    pub(super) fn emit_fs_atomic_write(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 2 {
            return Err(format!(
                "error: `fs.atomicWrite` expects 2 arguments (path, data), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.atomicWrite` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        let data_value = self.emit_expr(&args[1])?;
        if !data_value.is_struct_value() {
            return Err("error: `fs.atomicWrite` data must be a string".to_string());
        }
        let data = data_value.into_struct_value();
        self.emit_fs_void_result_call(
            "tyrus_fs_atomic_write_result",
            &[self.string_abi_type(), self.string_abi_type()],
            &[path.into(), data.into()],
            "fs.atomicWrite.result",
        )
    }

    pub(super) fn emit_fs_temp_dir(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if !args.is_empty() {
            return Err(format!(
                "error: `fs.tempDir` expects 0 arguments, found {}",
                args.len()
            ));
        }
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let temp_dir_ty = self.llvm_type(&Type::Named("TempDir".to_string()));
        let out_slot = self
            .builder
            .build_alloca(temp_dir_ty, "fs.tempDir.out")
            .unwrap();
        let fn_val = self.declare_libc_fn("tyrus_fs_temp_dir", &[ptr_type.into()], None);
        self.builder
            .build_call(fn_val, &[out_slot.into()], "")
            .unwrap();
        let result = self
            .builder
            .build_load(temp_dir_ty, out_slot, "fs.tempDir.result")
            .unwrap();
        self.record_owned_native_result(result, Type::Named("TempDir".to_string()));
        Ok(result)
    }

    pub(super) fn emit_fs_temp_file(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if !args.is_empty() {
            return Err(format!(
                "error: `fs.tempFile` expects 0 arguments, found {}",
                args.len()
            ));
        }
        self.emit_fs_result_call(
            "tyrus_fs_temp_file_result",
            &[],
            &[],
            Type::Result {
                success: Box::new(Type::Str),
                error: Box::new(Type::Named("FsError".to_string())),
            },
            "fs.tempFile.result",
        )
    }

    pub(super) fn emit_fs_watch(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!(
                "error: `fs.watch` expects 1 argument (path), found {}",
                args.len()
            ));
        }
        let path_value = self.emit_expr(&args[0])?;
        if !path_value.is_struct_value() {
            return Err("error: `fs.watch` path must be a string".to_string());
        }
        let path = path_value.into_struct_value();
        self.emit_fs_result_call(
            "tyrus_fs_watch_result",
            &[self.string_abi_type()],
            &[path.into()],
            Type::Result {
                success: Box::new(Type::Named("FSWatcher".to_string())),
                error: Box::new(Type::Named("FsError".to_string())),
            },
            "fs.watch.result",
        )
    }

    fn emit_fs_void_result_call(
        &mut self,
        symbol: &str,
        parameter_types: &[inkwell::types::BasicTypeEnum<'ctx>],
        arguments: &[inkwell::values::BasicMetadataValueEnum<'ctx>],
        result_name: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let result_type = Type::Result {
            success: Box::new(Type::Void),
            error: Box::new(Type::Named("FsError".to_string())),
        };
        self.emit_fs_result_call(symbol, parameter_types, arguments, result_type, result_name)
    }

    fn emit_fs_result_call(
        &mut self,
        symbol: &str,
        parameter_types: &[inkwell::types::BasicTypeEnum<'ctx>],
        arguments: &[inkwell::values::BasicMetadataValueEnum<'ctx>],
        result_type: Type,
        result_name: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let result_llvm_type = self.llvm_type(&result_type);
        let result_slot = self
            .builder
            .build_alloca(result_llvm_type, format!("{result_name}.abi").as_str())
            .map_err(|error| error.to_string())?;
        let mut call_types = parameter_types.to_vec();
        call_types.push(
            self.context
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
        );
        let function = self.declare_libc_fn(symbol, &call_types, None);
        let mut call_arguments = arguments.to_vec();
        call_arguments.push(result_slot.into());
        self.builder
            .build_call(function, &call_arguments, "")
            .map_err(|error| error.to_string())?;
        let result = self
            .builder
            .build_load(result_llvm_type, result_slot, result_name)
            .map_err(|error| error.to_string())?;
        self.record_owned_native_result(result, result_type);
        Ok(result)
    }
}
