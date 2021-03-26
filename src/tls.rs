use core::ptr::null_mut;

use alloc::boxed::Box;

use crate::syscalls::{arch_prctl, PrctlCode, SyscallResult};

unsafe fn get_gs() -> SyscallResult<u64> {
    let mut gs: u64 = 0;

    arch_prctl(PrctlCode::GET_GS, &mut gs as *mut _)?;

    Ok(gs)
}

unsafe fn set_gs(gs: u64) -> SyscallResult<()> {
    arch_prctl(PrctlCode::SET_GS, gs as *mut _)?;

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub struct Tls {
    pub stack_base: *mut u8,
    pub stack_limit: usize,
}

// TODO: mmap a page here to that TLS can be used inside of the allocator?
pub unsafe fn setup_tls(tls: Tls) -> SyscallResult<()> {
    set_gs(Box::into_raw(Box::new(tls)) as usize as u64)
}

pub unsafe fn get_tls_ptr() -> SyscallResult<*mut Tls> {
    get_gs().map(|gs| gs as *mut Tls)
}

pub unsafe fn teardown_tls() -> SyscallResult<Tls> {
    let tls = get_tls_ptr()?;

    // clear the gs register to that all (invalid) attempts to access TLS
    // SegFault instead of accessing random memory
    set_gs(0)?;

    let tls = Box::from_raw(tls);

    Ok(*tls)
}
