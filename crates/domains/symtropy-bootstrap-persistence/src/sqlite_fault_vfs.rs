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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FaultScope {
    Wal,
    MainDatabase,
}

#[derive(Clone, Copy, Debug)]
struct FaultPlan {
    operation: FaultOperation,
    ordinal: usize,
    seen: usize,
    fired: bool,
    scope: FaultScope,
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
}

#[repr(C)]
struct FileTail {
    original_methods: *const ffi::sqlite3_io_methods,
    open_flags: c_int,
    proxy_methods: ffi::sqlite3_io_methods,
}

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

pub fn arm_wal(operation: FaultOperation, ordinal: usize) -> FaultGuard {
    arm_inner(operation, ordinal, FaultScope::Wal)
}

/// Arm a one-shot fault against the main database file, excluding WAL I/O.
pub fn arm_main(operation: FaultOperation, ordinal: usize) -> FaultGuard {
    arm_inner(operation, ordinal, FaultScope::MainDatabase)
}

fn arm_inner(operation: FaultOperation, ordinal: usize, scope: FaultScope) -> FaultGuard {
    assert!(ordinal > 0, "fault ordinal is one-based");
    let serial = SERIAL.lock().expect("fault serial mutex");
    *PLAN.lock().expect("fault plan mutex") = Some(FaultPlan {
        operation,
        ordinal,
        seen: 0,
        fired: false,
        scope,
    });
    FaultGuard { _serial: serial }
}

pub fn fired() -> bool {
    PLAN.lock()
        .expect("fault plan mutex")
        .is_some_and(|plan| plan.fired)
}

/// Return the number of matching I/O calls observed by the currently armed plan.
pub fn observed() -> usize {
    PLAN.lock()
        .expect("fault plan mutex")
        .map_or(0, |plan| plan.seen)
}

fn maybe_fail(operation: FaultOperation, open_flags: c_int) -> Option<c_int> {
    let mut plan = PLAN.lock().expect("fault plan mutex");
    let Some(plan) = plan.as_mut() else {
        return None;
    };
    if plan.fired {
        return None;
    }

    if plan.operation != operation {
        return None;
    }

    let is_wal = (open_flags & ffi::SQLITE_OPEN_WAL) != 0;
    let scope_matches = match plan.scope {
        FaultScope::Wal => is_wal,
        FaultScope::MainDatabase => !is_wal,
    };
    if !scope_matches {
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

    unsafe {
        (*file).pMethods = ptr::null();
    }

    let rc = unsafe {