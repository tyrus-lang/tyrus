use crate::tyrus_panic;

fn out_of_bounds_range() -> ! {
    unsafe { tyrus_panic(c"tyrus runtime error: SparseTable range is out of bounds".as_ptr()) }
}

fn allocation_failed() -> ! {
    unsafe { tyrus_panic(c"tyrus runtime error: SparseTable allocation failed".as_ptr()) }
}

pub struct TyrusSparseTable {
    data: Vec<i32>,
    n: usize,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sparse_table_new(
    data: *const i32,
    length: usize,
) -> *mut TyrusSparseTable {
    unsafe {
        let n = length;
        let levels = if n == 0 {
            1
        } else {
            (n as u64).ilog2() as usize + 1
        };
        let total = levels.checked_mul(n).unwrap_or_else(|| allocation_failed());
        let mut table = vec![0i32; total];
        if n > 0 {
            std::ptr::copy_nonoverlapping(data, table.as_mut_ptr(), n);
        }
        for j in 1..levels {
            let step = 1usize << (j - 1);
            let span = 1usize << j;
            let count = n + 1 - span;
            for i in 0..count {
                let a = table[(j - 1) * n + i];
                let b = table[(j - 1) * n + i + step];
                table[j * n + i] = a.min(b);
            }
        }
        let st = Box::new(TyrusSparseTable { data: table, n });
        Box::into_raw(st)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sparse_table_query(
    st: *const TyrusSparseTable,
    from: usize,
    to: usize,
) -> i32 {
    unsafe {
        let st = &*st;
        if from >= to || to > st.n {
            out_of_bounds_range();
        }
        let len = to - from;
        let j = (len as u64).ilog2() as usize;
        let table = &st.data;
        let a = table[j * st.n + from];
        let b = table[j * st.n + (to - (1usize << j))];
        a.min(b)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sparse_table_len(st: *const TyrusSparseTable) -> usize {
    unsafe { (*st).n }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sparse_table_is_empty(st: *const TyrusSparseTable) -> bool {
    unsafe { (*st).n == 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sparse_table_free(st: *mut TyrusSparseTable) {
    unsafe {
        if !st.is_null() {
            drop(Box::from_raw(st));
        }
    }
}
