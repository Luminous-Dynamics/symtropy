// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Test-only SQLite VFS fault injector.
//!
//! This module wraps SQLite's default VFS and injects one selected I/O failure
//! at a deterministic operation ordinal. It is compiled only for tests and is
//! never selected by production storage code.

use std::{
    cell::RefCell,
    ffi::{c_char, c_int, c_void, CString},
    mem,
    ptr,
    sync::{Mutex, MutexGuard, OnceLock},
};

use rusqlite::ffi;

pub const NAME: &str = "symtropy_fault_vfs";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultOperation {
    Write,
    Sync,
    Truncate,
}

#[derive(Clone, Copy, Debug)]
struct FaultPlan {
    operation: FaultOperation,
    ordinal: usize,
    seen: usize,
    fired: bool,
    wal_only: bool,
}

static PLAN: Mutex<Option<FaultPlan>> = Mutex::new(None);
static SERIAL: Mutex<()> = Mutex::new(());
static VFS: OnceLock<usize> = OnceLock::new();

thread_local! {
    static ACTIVE_NAME: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub struct FaultGuard {
    _serial: MutexGuard<'static, ()>,
}

impl Drop for FaultGuard {
    fn drop(&mut self) {
        *PLAN.lock().expect("fault plan mutex") = None;
    }
}

pub struct ActivationGuard {
    previous: Option<String>,
}

impl Drop for ActivationGuard {
    fn drop(&mut self) {
        ACTIVE_NAME.with(|name| {
            *name.borrow_mut() = self.previous.take();
        });
    }
}

#[repr(C)]
struct ProxyVfsState {
    base_vfs: *mut ffi::sqlite3_vfs,
    tail_offset: usize,
    proxy_size: usize,
}

#[repr(C)]
struct FileTail {
    original_methods: *const ffi::sqlite3_io_methods,
    state: *const ProxyVfsState,
    open_flags: c_int,
    proxy_methods: ffi::sqlite3_io_methods,
}

// SQLite owns the VFS callback thread. The process-leaked default VFS pointer
// and immutable proxy state remain valid for the lifetime of the test process.
unsafe impl Send for ProxyVfsState {}
unsafe impl Sync for ProxyVfsState {}

fn align_up(value: usize, alignment: usize) -> usize {
    debug_assert!(alignment.is_power_of_two());
    (value + alignment - 1) & !(alignment - 1)
}

pub fn install() {
    VFS.get_or_init(|| unsafe {
        let base_vfs = ffi::sqlite3_vfs_find(ptr::null());
        assert!(!base_vfs.is_null(), "SQLite default VFS must exist");

        let base_size = usize::try_from((*base_vfs).szOsFile)
            .expect("SQLite VFS file size must be non-negative");
        let tail_offset = align_up(base_size, mem::align_of::<FileTail>());
        let proxy_size = tail_offset
            .checked_add(mem::size_of::<FileTail>())
            .expect("SQLite proxy file size overflow");

        let state = Box::new(ProxyVfsState {
            base_vfs,
            tail_offset,
            proxy_size,
        });
        let state_ptr = Box::into_raw(state);

        let name = Box::leak(Box::new(
            CString::new(NAME).expect("static VFS name cannot contain NUL"),
        ));

        let mut vfs = Box::new(mem::zeroed::<ffi::sqlite3_vfs>());
        vfs.iVersion = (*base_vfs).iVersion;
        vfs.szOsFile = c_int::try_from(proxy_size).expect("SQLite VFS file size fits c_int");
        vfs.mxPathname = (*base_vfs).mxPathname;
        vfs.zName = name.as_ptr();
        vfs.pAppData = state_ptr.cast();

        vfs.xOpen = Some(vfs_x_open);
        vfs.xDelete = Some(vfs_x_delete);
        vfs.xAccess = Some(vfs_x_access);
        vfs.xFullPathname = Some(vfs_x_full_pathname);
        vfs.xDlOpen = None;
        vfs.xDlError = None;
        vfs.xDlSym = None;
        vfs.xDlClose = None;
        vfs.xRandomness = Some(vfs_x_randomness);
        vfs.xSleep = Some(vfs_x_sleep);
        vfs.xCurrentTime = Some(vfs_x_current_time);
        vfs.xGetLastError = Some(vfs_x_get_last_error);

        if (*base_vfs).iVersion >= 2 {
            vfs.xCurrentTimeInt64 = Some(vfs_x_current_time_int64);
        }
        if (*base_vfs).iVersion >= 3 {
            vfs.xSetSystemCall = (*base_vfs).xSetSystemCall.map(|_| vfs_x_set_system_call);
            vfs.xGetSystemCall = (*base_vfs).xGetSystemCall.map(|_| vfs_x_get_system_call);
            vfs.xNextSystemCall =
                (*base_vfs).xNextSystemCall.map(|_| vfs_x_next_system_call);
        }

        let vfs_ptr = Box::into_raw(vfs);
        let rc = ffi::sqlite3_vfs_register(vfs_ptr, 0);
        assert_eq!(rc, ffi::SQLITE_OK, "test VFS registration failed");

        vfs_ptr as usize
    });
}

pub fn activate_current_thread() -> ActivationGuard {
    install();
    ACTIVE_NAME.with(|name| ActivationGuard {
        previous: name.borrow_mut().replace(NAME.to_owned()),
    })
}

pub fn active_name() -> Option<String> {
    ACTIVE_NAME.with(|name| name.borrow().clone())
}

pub fn arm(operation: FaultOperation, ordinal: usize) -> FaultGuard {
    arm_inner(operation, ordinal, false)
}

pub fn arm_wal(operation: FaultOperation, ordinal: usize) -> FaultGuard {
    arm_inner(operation, ordinal, true)
}

fn arm_inner(operation: FaultOperation, ordinal: usize, wal_only: bool) -> FaultGuard {
    assert!(ordinal > 0, "fault ordinal is one-based");
    let serial = SERIAL.lock().expect("fault serial mutex");
    *PLAN.lock().expect("fault plan mutex") = Some(FaultPlan {
        operation,
        ordinal,
        seen: 0,
        fired: false,
        wal_only,
    });
    FaultGuard { _serial: serial }
}

pub fn fired() -> bool {
    PLAN.lock()
        .expect("fault plan mutex")
        .is_some_and(|plan| plan.fired)
}

fn maybe_fail(operation: FaultOperation, open_flags: c_int) -> Option<c_int> {
    let mut plan = PLAN.lock().expect("fault plan mutex");
    let Some(plan) = plan.as_mut() else {
        return None;
    };

    if plan.operation != operation
        || (plan.wal_only && (open_flags & ffi::SQLITE_OPEN_WAL) == 0)
    {
        return None;
    }

    plan.seen += 1;
    if plan.seen != plan.ordinal {
        return None;
    }

    plan.fired = true;
    Some(match operation {
        FaultOperation::Write => ffi::SQLITE_IOERR_WRITE,
        FaultOperation::Sync => ffi::SQLITE_IOERR_FSYNC,
        FaultOperation::Truncate => ffi::SQLITE_IOERR_TRUNCATE,
    })
}

unsafe fn tail_from_file(file: *mut ffi::sqlite3_file) -> &'static mut FileTail {
    let proxy_methods = unsafe { (*file).pMethods };
    assert!(!proxy_methods.is_null(), "proxy SQLite methods must exist");

    unsafe {
        &mut *(
            proxy_methods
                .cast_mut()
                .cast::<u8>()
                .sub(mem::offset_of!(FileTail, proxy_methods))
                .cast::<FileTail>()
        )
    }
}

unsafe fn original_methods(file: *mut ffi::sqlite3_file) -> *const ffi::sqlite3_io_methods {
    unsafe { tail_from_file(file).original_methods }
}

unsafe fn open_flags(file: *mut ffi::sqlite3_file) -> c_int {
    unsafe { tail_from_file(file).open_flags }
}

unsafe extern "C" fn vfs_x_open(
    vfs: *mut ffi::sqlite3_vfs,
    z_name: ffi::sqlite3_filename,
    file: *mut ffi::sqlite3_file,
    flags: c_int,
    out_flags: *mut c_int,
) -> c_int {
    let state = unsafe { &*((*vfs).pAppData.cast::<ProxyVfsState>()) };
    let base = state.base_vfs;

    let rc = unsafe {
        ((*base).xOpen.expect("default SQLite VFS xOpen"))(
            base, z_name, file, flags, out_flags,
        )
    };
    if rc != ffi::SQLITE_OK {
        return rc;
    }

    let original_methods = unsafe { (*file).pMethods };
    if original_methods.is_null() {
        return ffi::SQLITE_CANTOPEN;
    }

    let tail = unsafe { tail_from_file_with_offset(file, state.tail_offset) };
    tail.original_methods = original_methods;
    tail.state = state as *const _;
    tail.open_flags = flags;
    tail.proxy_methods = unsafe { *original_methods };
    tail.proxy_methods.xClose = Some(io_x_close);
    tail.proxy_methods.xRead = Some(io_x_read);
    tail.proxy_methods.xWrite = Some(io_x_write);
    tail.proxy_methods.xTruncate = Some(io_x_truncate);
    tail.proxy_methods.xSync = Some(io_x_sync);
    tail.proxy_methods.xFileSize = Some(io_x_file_size);
    tail.proxy_methods.xLock = Some(io_x_lock);
    tail.proxy_methods.xUnlock = Some(io_x_unlock);
    tail.proxy_methods.xCheckReservedLock = Some(io_x_check_reserved_lock);
    tail.proxy_methods.xFileControl = Some(io_x_file_control);
    tail.proxy_methods.xSectorSize = Some(io_x_sector_size);
    tail.proxy_methods.xDeviceCharacteristics = Some(io_x_device_characteristics);

    if tail.proxy_methods.iVersion >= 2 {
        tail.proxy_methods.xShmMap = Some(io_x_shm_map);
        tail.proxy_methods.xShmLock = Some(io_x_shm_lock);
        tail.proxy_methods.xShmBarrier = Some(io_x_shm_barrier);
        tail.proxy_methods.xShmUnmap = Some(io_x_shm_unmap);
    }
    if tail.proxy_methods.iVersion >= 3 {
        tail.proxy_methods.xFetch = Some(io_x_fetch);
        tail.proxy_methods.xUnfetch = Some(io_x_unfetch);
    }

    unsafe {
        (*file).pMethods = &tail.proxy_methods;
    }
    ffi::SQLITE_OK
}

unsafe fn tail_from_file_with_offset(
    file: *mut ffi::sqlite3_file,
    offset: usize,
) -> &'static mut FileTail {
    unsafe { &mut *((file.cast::<u8>()).add(offset).cast::<FileTail>()) }
}

unsafe extern "C" fn io_x_close(file: *mut ffi::sqlite3_file) -> c_int {
    let tail = unsafe { tail_from_file(file) };
    let original = tail.original_methods;
    unsafe {
        (*file).pMethods = original;
        ((*original).xClose.expect("SQLite xClose"))(file)
    }
}

unsafe extern "C" fn io_x_read(
    file: *mut ffi::sqlite3_file,
    buffer: *mut c_void,
    amount: c_int,
    offset: ffi::sqlite3_int64,
) -> c_int {
    let original = unsafe { original_methods(file) };
    unsafe { ((*original).xRead.expect("SQLite xRead"))(file, buffer, amount, offset) }
}

unsafe extern "C" fn io_x_write(
    file: *mut ffi::sqlite3_file,
    buffer: *const c_void,
    amount: c_int,
    offset: ffi::sqlite3_int64,
) -> c_int {
    let flags = unsafe { open_flags(file) };
    if let Some(rc) = maybe_fail(FaultOperation::Write, flags) {
        return rc;
    }
    let original = unsafe { original_methods(file) };
    unsafe { ((*original).xWrite.expect("SQLite xWrite"))(file, buffer, amount, offset) }
}

unsafe extern "C" fn io_x_truncate(
    file: *mut ffi::sqlite3_file,
    size: ffi::sqlite3_int64,
) -> c_int {
    let flags = unsafe { open_flags(file) };
    if let Some(rc) = maybe_fail(FaultOperation::Truncate, flags) {
        return rc;
    }
    let original = unsafe { original_methods(file) };
    unsafe { ((*original).xTruncate.expect("SQLite xTruncate"))(file, size) }
}

unsafe extern "C" fn io_x_sync(file: *mut ffi::sqlite3_file, flags: c_int) -> c_int {
    let open_flags = unsafe { open_flags(file) };
    if let Some(rc) = maybe_fail(FaultOperation::Sync, open_flags) {
        return rc;
    }
    let original = unsafe { original_methods(file) };
    unsafe { ((*original).xSync.expect("SQLite xSync"))(file, flags) }
}

unsafe extern "C" fn io_x_file_size(
    file: *mut ffi::sqlite3_file,
    size: *mut ffi::sqlite3_int64,
) -> c_int {
    let original = unsafe { original_methods(file) };
    unsafe { ((*original).xFileSize.expect("SQLite xFileSize"))(file, size) }
}

unsafe extern "C" fn io_x_lock(file: *mut ffi::sqlite3_file, level: c_int) -> c_int {
    let original = unsafe { original_methods(file) };
    unsafe { ((*original).xLock.expect("SQLite xLock"))(file, level) }
}

unsafe extern "C" fn io_x_unlock(file: *mut ffi::sqlite3_file, level: c_int) -> c_int {
    let original = unsafe { original_methods(file) };
    unsafe { ((*original).xUnlock.expect("SQLite xUnlock"))(file, level) }
}

unsafe extern "C" fn io_x_check_reserved_lock(
    file: *mut ffi::sqlite3_file,
    result: *mut c_int,
) -> c_int {
    let original = unsafe { original_methods(file) };
    unsafe {
        ((*original)
            .xCheckReservedLock
            .expect("SQLite xCheckReservedLock"))(file, result)
    }
}

unsafe extern "C" fn io_x_file_control(
    file: *mut ffi::sqlite3_file,
    op: c_int,
    arg: *mut c_void,
) -> c_int {
    let original = unsafe { original_methods(file) };
    unsafe { ((*original).xFileControl.expect("SQLite xFileControl"))(file, op, arg) }
}

unsafe extern "C" fn io_x_sector_size(file: *mut ffi::sqlite3_file) -> c_int {
    let original = unsafe { original_methods(file) };
    unsafe { ((*original).xSectorSize.expect("SQLite xSectorSize"))(file) }
}

unsafe extern "C" fn io_x_device_characteristics(file: *mut ffi::sqlite3_file) -> c_int {
    let original = unsafe { original_methods(file) };
    unsafe {
        ((*original)
            .xDeviceCharacteristics
            .expect("SQLite xDeviceCharacteristics"))(file)
    }
}

unsafe extern "C" fn io_x_shm_map(
    file: *mut ffi::sqlite3_file,
    page: c_int,
    page_size: c_int,
    extend: c_int,
    result: *mut *mut c_void,
) -> c_int {
    let original = unsafe { original_methods(file) };
    unsafe { ((*original).xShmMap.expect("SQLite xShmMap"))(file, page, page_size, extend, result) }
}

unsafe extern "C" fn io_x_shm_lock(
    file: *mut ffi::sqlite3_file,
    offset: c_int,
    number: c_int,
    flags: c_int,
) -> c_int {
    let original = unsafe { original_methods(file) };
    unsafe { ((*original).xShmLock.expect("SQLite xShmLock"))(file, offset, number, flags) }
}

unsafe extern "C" fn io_x_shm_barrier(file: *mut ffi::sqlite3_file) {
    let original = unsafe { original_methods(file) };
    if let Some(method) = unsafe { (*original).xShmBarrier } {
        unsafe { method(file) };
    }
}

unsafe extern "C" fn io_x_shm_unmap(
    file: *mut ffi::sqlite3_file,
    delete_flag: c_int,
) -> c_int {
    let original = unsafe { original_methods(file) };
    unsafe { ((*original).xShmUnmap.expect("SQLite xShmUnmap"))(file, delete_flag) }
}

unsafe extern "C" fn io_x_fetch(
    file: *mut ffi::sqlite3_file,
    offset: ffi::sqlite3_int64,
    amount: c_int,
    result: *mut *mut c_void,
) -> c_int {
    let original = unsafe { original_methods(file) };
    unsafe { ((*original).xFetch.expect("SQLite xFetch"))(file, offset, amount, result) }
}

unsafe extern "C" fn io_x_unfetch(
    file: *mut ffi::sqlite3_file,
    offset: ffi::sqlite3_int64,
    ptr: *mut c_void,
) -> c_int {
    let original = unsafe { original_methods(file) };
    unsafe { ((*original).xUnfetch.expect("SQLite xUnfetch"))(file, offset, ptr) }
}

unsafe extern "C" fn vfs_x_delete(
    vfs: *mut ffi::sqlite3_vfs,
    z_name: *const c_char,
    sync_dir: c_int,
) -> c_int {
    let base = unsafe { &*((*vfs).pAppData.cast::<ProxyVfsState>()) }.base_vfs;
    unsafe { ((*base).xDelete.expect("default SQLite VFS xDelete"))(base, z_name, sync_dir) }
}

unsafe extern "C" fn vfs_x_access(
    vfs: *mut ffi::sqlite3_vfs,
    z_name: *const c_char,
    flags: c_int,
    result: *mut c_int,
) -> c_int {
    let base = unsafe { &*((*vfs).pAppData.cast::<ProxyVfsState>()) }.base_vfs;
    unsafe { ((*base).xAccess.expect("default SQLite VFS xAccess"))(base, z_name, flags, result) }
}

unsafe extern "C" fn vfs_x_full_pathname(
    vfs: *mut ffi::sqlite3_vfs,
    z_name: *const c_char,
    n_out: c_int,
    z_out: *mut c_char,
) -> c_int {
    let base = unsafe { &*((*vfs).pAppData.cast::<ProxyVfsState>()) }.base_vfs;
    unsafe {
        ((*base).xFullPathname.expect("default SQLite VFS xFullPathname"))(
            base, z_name, n_out, z_out,
        )
    }
}

unsafe extern "C" fn vfs_x_randomness(
    vfs: *mut ffi::sqlite3_vfs,
    n_byte: c_int,
    z_out: *mut c_char,
) -> c_int {
    let base = unsafe { &*((*vfs).pAppData.cast::<ProxyVfsState>()) }.base_vfs;
    unsafe { ((*base).xRandomness.expect("default SQLite VFS xRandomness"))(base, n_byte, z_out) }
}

unsafe extern "C" fn vfs_x_sleep(vfs: *mut ffi::sqlite3_vfs, microseconds: c_int) -> c_int {
    let base = unsafe { &*((*vfs).pAppData.cast::<ProxyVfsState>()) }.base_vfs;
    unsafe { ((*base).xSleep.expect("default SQLite VFS xSleep"))(base, microseconds) }
}

unsafe extern "C" fn vfs_x_current_time(
    vfs: *mut ffi::sqlite3_vfs,
    result: *mut f64,
) -> c_int {
    let base = unsafe { &*((*vfs).pAppData.cast::<ProxyVfsState>()) }.base_vfs;
    unsafe { ((*base).xCurrentTime.expect("default SQLite VFS xCurrentTime"))(base, result) }
}

unsafe extern "C" fn vfs_x_get_last_error(
    vfs: *mut ffi::sqlite3_vfs,
    n_byte: c_int,
    z_err_msg: *mut c_char,
) -> c_int {
    let base = unsafe { &*((*vfs).pAppData.cast::<ProxyVfsState>()) }.base_vfs;
    unsafe {
        ((*base).xGetLastError.expect("default SQLite VFS xGetLastError"))(
            base, n_byte, z_err_msg,
        )
    }
}

unsafe extern "C" fn vfs_x_current_time_int64(
    vfs: *mut ffi::sqlite3_vfs,
    result: *mut ffi::sqlite3_int64,
) -> c_int {
    let base = unsafe { &*((*vfs).pAppData.cast::<ProxyVfsState>()) }.base_vfs;
    unsafe {
        ((*base)
            .xCurrentTimeInt64
            .expect("default SQLite VFS xCurrentTimeInt64"))(base, result)
    }
}

unsafe extern "C" fn vfs_x_set_system_call(
    vfs: *mut ffi::sqlite3_vfs,
    z_name: *const c_char,
    call: ffi::sqlite3_syscall_ptr,
) -> c_int {
    let base = unsafe { &*((*vfs).pAppData.cast::<ProxyVfsState>()) }.base_vfs;
    unsafe {
        ((*base).xSetSystemCall.expect("default SQLite VFS xSetSystemCall"))(
            base, z_name, call,
        )
    }
}

unsafe extern "C" fn vfs_x_get_system_call(
    vfs: *mut ffi::sqlite3_vfs,
    z_name: *const c_char,
) -> ffi::sqlite3_syscall_ptr {
    let base = unsafe { &*((*vfs).pAppData.cast::<ProxyVfsState>()) }.base_vfs;
    unsafe { ((*base).xGetSystemCall.expect("default SQLite VFS xGetSystemCall"))(base, z_name) }
}

unsafe extern "C" fn vfs_x_next_system_call(
    vfs: *mut ffi::sqlite3_vfs,
    z_name: *const c_char,
) -> *const c_char {
    let base = unsafe { &*((*vfs).pAppData.cast::<ProxyVfsState>()) }.base_vfs;
    unsafe { ((*base).xNextSystemCall.expect("default SQLite VFS xNextSystemCall"))(base, z_name) }
}
