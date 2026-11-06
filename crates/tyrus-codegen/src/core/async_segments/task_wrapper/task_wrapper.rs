use super::*;

/// What the caller-side wrapper of an async function needs to allocate the
/// frame, build the task and hand back the promise.
#[derive(Clone, Copy)]
pub(super) struct AsyncTaskWrapper<'a, 'ctx> {
    pub(super) name: &'a str,
    pub(super) guards_throws: bool,
    pub(super) poll: FunctionValue<'ctx>,
    pub(super) frame_type: Option<inkwell::types::StructType<'ctx>>,
    pub(super) pointer: inkwell::types::PointerType<'ctx>,
    pub(super) frame_params: &'a [AsyncFrameParam],
    pub(super) frame_locals: &'a [(String, Type, AsyncLocalStorage)],
    pub(super) expression_temporaries: &'a [(Span, Type)],
    pub(super) for_of_await_count: usize,
}

impl<'ctx> Codegen<'ctx> {
    /// Emits the body of the async function itself: it allocates the frame,
    /// schedules the task and returns the promise.
    pub(super) fn emit_async_task_wrapper(
        &mut self,
        context: AsyncTaskWrapper<'_, 'ctx>,
    ) -> Result<(), String> {
        let AsyncTaskWrapper {
            name,
            guards_throws,
            poll,
            frame_type,
            pointer,
            frame_params,
            frame_locals,
            expression_temporaries,
            for_of_await_count,
        } = context;
        let guarded = guards_throws
            .then(|| self.end_async_throw_regions())
            .flatten();
        let task_poll = match (guarded, frame_type) {
            (Some(guarded), Some(frame_type)) => {
                self.emit_async_guarded_poll(name, poll, frame_type, &guarded)?
            }
            _ => poll,
        };

        let function = *self
            .fns
            .get(name)
            .ok_or_else(|| format!("internal: async function `{name}` was not forward-declared"))?;
        let entry = self
            .main_continuation(function)
            .filter(|_| name == "main")
            .filter(|block| block.get_terminator().is_none())
            .unwrap_or_else(|| self.context.append_basic_block(function, "entry"));
        self.builder.position_at_end(entry);
        if name == "main" {
            self.emit_env_startup()?;
        }
        let pending = self.declare_libc_fn("tyrus_promise_pending", &[], Some(pointer.into()));
        let promise = self
            .builder
            .build_call(pending, &[], "async.main.promise")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();
        let task_new = self.declare_libc_fn(
            "tyrus_async_task_new",
            &[
                pointer.into(),
                pointer.into(),
                pointer.into(),
                pointer.into(),
            ],
            Some(pointer.into()),
        );
        let frame = if let Some(frame_type) = frame_type {
            let (calloc, nmemb, size) = self.declare_calloc(
                self.context.i64_type().const_int(1, false),
                frame_type.size_of().unwrap(),
            );
            let frame = self
                .builder
                .build_call(calloc, &[nmemb.into(), size.into()], "async.frame")
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap()
                .into_pointer_value();
            let state_pointer = self
                .builder
                .build_struct_gep(frame_type, frame, 0, "async.frame.state")
                .unwrap();
            self.builder
                .build_store(state_pointer, self.context.i32_type().const_zero())
                .unwrap();
            for (index, parameter) in frame_params.iter().enumerate() {
                let slot = self
                    .builder
                    .build_struct_gep(
                        frame_type,
                        frame,
                        (index + 2) as u32,
                        "async.frame.param.store",
                    )
                    .unwrap();
                let value = function.get_nth_param(index as u32).unwrap();
                self.store_async_frame_parameter(function, parameter, slot, value)?;
            }
            frame
        } else {
            pointer.const_null()
        };
        let drop_frame = if let Some(frame_type) = frame_type {
            let wrapper_block = self.builder.get_insert_block().unwrap();
            let drop = self.emit_async_frame_drop(
                name,
                frame_type,
                frame_params,
                frame_locals,
                expression_temporaries,
                for_of_await_count * 3,
            );
            self.builder.position_at_end(wrapper_block);
            drop.as_global_value().as_pointer_value()
        } else {
            pointer.const_null()
        };
        let task = self
            .builder
            .build_call(
                task_new,
                &[
                    task_poll.as_global_value().as_pointer_value().into(),
                    drop_frame.into(),
                    frame.into(),
                    promise.into(),
                ],
                "async.main.task",
            )
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();
        let schedule = self.declare_libc_fn("tyrus_async_task_schedule", &[pointer.into()], None);
        self.builder
            .build_call(schedule, &[task.into()], "")
            .unwrap();
        let block_on = self.declare_libc_fn(
            "tyrus_async_block_on",
            &[pointer.into()],
            Some(self.context.i32_type().into()),
        );
        let task_free = self.declare_libc_fn("tyrus_async_task_free", &[pointer.into()], None);
        self.builder
            .build_call(task_free, &[task.into()], "")
            .unwrap();
        if name == "main" {
            let status = self
                .builder
                .build_call(block_on, &[promise.into()], "async.main.status")
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap();
            let promise_free = self.declare_libc_fn("tyrus_promise_free", &[pointer.into()], None);
            self.builder
                .build_call(promise_free, &[promise.into()], "")
                .unwrap();
            self.emit_runtime_shutdown();
            self.builder.build_return(Some(&status)).unwrap();
        } else {
            self.builder.build_return(Some(&promise)).unwrap();
        }
        Ok(())
    }
}
