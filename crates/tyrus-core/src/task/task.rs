use crate::ast::{Type, TypeParam};

pub const CANCEL_TARGET: &str = "__tyrus_task_cancel";
pub const SLEEP_TARGET: &str = "__tyrus_task_sleep";
pub const SLEEP_ASYNC_TARGET: &str = "__tyrus_task_sleepAsync";
pub const SPAWN_TARGET: &str = "__tyrus_task_spawn";
pub const CHANNEL_TARGET: &str = "__tyrus_task_channel";
pub const MUTEX_TARGET: &str = "__tyrus_task_mutex_new";
pub const RWLOCK_TARGET: &str = "__tyrus_task_rwlock_new";
pub const SEMAPHORE_TARGET: &str = "__tyrus_task_semaphore_new";
pub const JOIN_ALL_TARGET: &str = "__tyrus_task_joinAll";
pub const RACE_TARGET: &str = "__tyrus_task_race";
pub const TIMEOUT_TARGET: &str = "__tyrus_task_timeout";
pub const IS_CANCELLED_TARGET: &str = "__tyrus_task_isCancelled";

pub fn cancel_type() -> Type {
    Type::GenericFunction {
        type_params: vec![TypeParam {
            name: "T".to_string(),
            constraint: None,
        }],
        params: vec![
            Type::Apply {
                name: "Promise".to_string(),
                args: vec![Type::Generic("T".to_string())],
            },
            Type::Str,
            Type::Optional(Box::new(Type::U64)),
        ],
        ret: Box::new(Type::Bool),
    }
}

pub fn is_cancelled_type() -> Type {
    Type::Function {
        params: Vec::new(),
        ret: Box::new(Type::Bool),
    }
}

pub fn sleep_type() -> Type {
    Type::Function {
        params: vec![Type::U64],
        ret: Box::new(Type::Void),
    }
}

pub fn sleep_async_type() -> Type {
    Type::Function {
        params: vec![Type::U64],
        ret: Box::new(Type::Apply {
            name: "Promise".to_string(),
            args: vec![Type::Void],
        }),
    }
}

pub fn spawn_type() -> Type {
    Type::GenericFunction {
        type_params: vec![TypeParam {
            name: "T".to_string(),
            constraint: None,
        }],
        params: vec![Type::Function {
            params: vec![],
            ret: Box::new(Type::Apply {
                name: "Promise".to_string(),
                args: vec![Type::Generic("T".to_string())],
            }),
        }],
        ret: Box::new(Type::Apply {
            name: "JoinHandle".to_string(),
            args: vec![Type::Generic("T".to_string())],
        }),
    }
}

pub fn join_all_type() -> Type {
    Type::GenericFunction {
        type_params: vec![TypeParam {
            name: "T".to_string(),
            constraint: None,
        }],
        params: vec![Type::Array(Box::new(Type::Apply {
            name: "JoinHandle".to_string(),
            args: vec![Type::Generic("T".to_string())],
        }))],
        ret: Box::new(Type::Apply {
            name: "Promise".to_string(),
            args: vec![Type::Array(Box::new(Type::Generic("T".to_string())))],
        }),
    }
}

pub fn race_type() -> Type {
    Type::GenericFunction {
        type_params: vec![TypeParam {
            name: "T".to_string(),
            constraint: None,
        }],
        params: vec![Type::Array(Box::new(Type::Apply {
            name: "JoinHandle".to_string(),
            args: vec![Type::Generic("T".to_string())],
        }))],
        ret: Box::new(Type::Apply {
            name: "Promise".to_string(),
            args: vec![Type::Generic("T".to_string())],
        }),
    }
}

pub fn timeout_type() -> Type {
    Type::GenericFunction {
        type_params: vec![TypeParam {
            name: "T".to_string(),
            constraint: None,
        }],
        params: vec![
            Type::U64,
            Type::Apply {
                name: "Promise".to_string(),
                args: vec![Type::Generic("T".to_string())],
            },
        ],
        ret: Box::new(Type::Apply {
            name: "Promise".to_string(),
            args: vec![Type::Generic("T".to_string())],
        }),
    }
}

pub fn mutex_type() -> Type {
    Type::GenericFunction {
        type_params: vec![TypeParam {
            name: "T".to_string(),
            constraint: None,
        }],
        params: vec![Type::Generic("T".to_string())],
        ret: Box::new(Type::Apply {
            name: "Mutex".to_string(),
            args: vec![Type::Generic("T".to_string())],
        }),
    }
}

pub fn rwlock_type() -> Type {
    Type::GenericFunction {
        type_params: vec![TypeParam {
            name: "T".to_string(),
            constraint: None,
        }],
        params: vec![Type::Generic("T".to_string())],
        ret: Box::new(Type::Apply {
            name: "RwLock".to_string(),
            args: vec![Type::Generic("T".to_string())],
        }),
    }
}

pub fn semaphore_type() -> Type {
    Type::Function {
        params: vec![Type::Usize],
        ret: Box::new(Type::Named("Semaphore".to_string())),
    }
}

pub fn channel_type() -> Type {
    Type::GenericFunction {
        type_params: vec![TypeParam {
            name: "T".to_string(),
            constraint: None,
        }],
        params: vec![Type::Optional(Box::new(Type::Usize))],
        ret: Box::new(Type::Tuple(vec![
            Type::Apply {
                name: "Sender".to_string(),
                args: vec![Type::Generic("T".to_string())],
            },
            Type::Apply {
                name: "Receiver".to_string(),
                args: vec![Type::Generic("T".to_string())],
            },
        ])),
    }
}
