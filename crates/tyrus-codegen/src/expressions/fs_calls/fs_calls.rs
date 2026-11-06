use super::*;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_fs_fn_call(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        match callee {
            "__tyrus_fs_readFileSync" => {
                let result = self.emit_read_file_sync(args)?;
                return Ok(Some(result));
            }
            "__tyrus_fs_writeFileSync" => return (self.emit_write_file_sync(args)).map(Some),
            "__tyrus_fs_readFile" => {
                let result = self.emit_fs_read_file(args)?;
                return Ok(Some(result));
            }
            "__tyrus_fs_readFileStream" => {
                return (self.emit_fs_file_stream(args, true)).map(Some);
            }
            "__tyrus_fs_writeFile" => {
                let result = self.emit_fs_write_file(args)?;
                return Ok(Some(result));
            }
            "__tyrus_fs_writeFileStream" => {
                return (self.emit_fs_file_stream(args, false)).map(Some);
            }
            "__tyrus_fs_exists" => return (self.emit_fs_exists(args)).map(Some),
            "__tyrus_fs_appendFile" => {
                let result = self.emit_fs_append_file(args)?;
                return Ok(Some(result));
            }
            "__tyrus_fs_readdir" => {
                let result = self.emit_fs_readdir(args)?;
                return Ok(Some(result));
            }
            "__tyrus_fs_mkdir" => {
                return (self.emit_fs_mkdir(args)).map(Some);
            }
            "__tyrus_fs_rm" => {
                return (self.emit_fs_rm(args)).map(Some);
            }
            "__tyrus_fs_cp" => {
                return (self.emit_fs_cp(args)).map(Some);
            }
            "__tyrus_fs_mv" => {
                return (self.emit_fs_mv(args)).map(Some);
            }
            "__tyrus_fs_removeAll" => {
                return (self.emit_fs_remove_all(args)).map(Some);
            }
            "__tyrus_fs_stat" => {
                let result = self.emit_fs_stat(args)?;
                return Ok(Some(result));
            }
            "__tyrus_fs_metadata" => {
                let result = self.emit_fs_metadata(args)?;
                return Ok(Some(result));
            }
            "__tyrus_fs_disk_usage" => {
                let result = self.emit_fs_disk_usage(args)?;
                return Ok(Some(result));
            }
            "__tyrus_fs_mime" => {
                let result = self.emit_fs_mime(args)?;
                return Ok(Some(result));
            }
            "__tyrus_fs_chmod" => {
                return (self.emit_fs_chmod(args)).map(Some);
            }
            "__tyrus_fs_chown" => {
                return (self.emit_fs_chown(args)).map(Some);
            }
            "__tyrus_fs_readlink" => {
                let result = self.emit_fs_readlink(args)?;
                return Ok(Some(result));
            }
            "__tyrus_fs_symlink" => {
                return (self.emit_fs_symlink(args)).map(Some);
            }
            "__tyrus_fs_symlink_dir" => {
                return (self.emit_fs_symlink_dir(args)).map(Some);
            }
            "__tyrus_fs_hard_link" => {
                return (self.emit_fs_hard_link(args)).map(Some);
            }
            "__tyrus_fs_canonicalize" => {
                let result = self.emit_fs_canonicalize(args)?;
                return Ok(Some(result));
            }
            "__tyrus_fs_glob" => {
                let result = self.emit_fs_glob(args)?;
                return Ok(Some(result));
            }
            "__tyrus_fs_search" => {
                let result = self.emit_fs_search(args)?;
                return Ok(Some(result));
            }
            "__tyrus_fs_diff" => {
                let result = self.emit_fs_diff(args)?;
                return Ok(Some(result));
            }
            "__tyrus_fs_atomic_write" => {
                return (self.emit_fs_atomic_write(args)).map(Some);
            }
            "__tyrus_fs_temp_dir" => {
                let result = self.emit_fs_temp_dir(args)?;
                return Ok(Some(result));
            }
            "__tyrus_fs_temp_file" => {
                let result = self.emit_fs_temp_file(args)?;
                return Ok(Some(result));
            }
            "__tyrus_fs_watch" => {
                let result = self.emit_fs_watch(args)?;
                return Ok(Some(result));
            }
            _ => {}
        }
        Ok(None)
    }
}
