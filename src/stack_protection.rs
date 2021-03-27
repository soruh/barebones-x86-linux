use crate::{syscalls::*, PAGESIZE};
use core::ptr::null;
use core::ptr::null_mut;

const SIG_STACK_SIZE: usize = 60 * 1024;
const GUARD_SIZE: usize = 2; // size of the stack guard in pages

pub unsafe fn get_alt_stack() -> SyscallResult<SignalStack> {
    let mut signal_stack = SignalStack {
        stack_pointer: null_mut(),
        flags: SignalStackFlags::empty(),
        size: 0,
    };

    sigaltstack(null(), &mut signal_stack as *mut SignalStack)?;

    Ok(signal_stack)
}

pub unsafe fn setup_alt_stack() -> SyscallResult<()> {
    let stack = mmap(
        null_mut(),
        SIG_STACK_SIZE,
        MProt::READ | MProt::WRITE,
        MMapFlags::PRIVATE | MMapFlags::ANONYMOUS,
        -1,
        0,
    )?;

    let new_signal_stack = SignalStack {
        stack_pointer: stack,
        flags: SignalStackFlags::empty(),
        size: SIG_STACK_SIZE,
    };
    let mut old_signal_stack = SignalStack {
        stack_pointer: null_mut(),
        flags: SignalStackFlags::empty(),
        size: 0,
    };

    sigaltstack(
        &new_signal_stack as *const SignalStack,
        &mut old_signal_stack as *mut SignalStack,
    )?;

    trace!(
        "allocated signal stack at {:?}",
        new_signal_stack.stack_pointer
    );

    /*
    assert_eq!(
        old_signal_stack.flags & SignalStackFlags::DISABLE,
        SignalStackFlags::DISABLE,
        "there was already a registered signal stack"
    );
    */

    Ok(())
}

pub unsafe fn teardown_alt_stack() -> SyscallResult<()> {
    let new_signal_stack = SignalStack {
        stack_pointer: null_mut(),
        flags: SignalStackFlags::DISABLE,
        size: 0,
    };

    let mut old_signal_stack = SignalStack {
        stack_pointer: null_mut(),
        flags: SignalStackFlags::empty(),
        size: 0,
    };

    sigaltstack(
        &new_signal_stack as *const SignalStack,
        &mut old_signal_stack as *mut SignalStack,
    )?;

    trace!(
        "freeing signal stack at {:?}",
        old_signal_stack.stack_pointer
    );

    munmap(old_signal_stack.stack_pointer, old_signal_stack.size)?;

    Ok(())
}

type Page = [u8; PAGESIZE];

unsafe fn calc_guard_location(stack_base: *const u8, stack_size: usize) -> *mut u8 {
    let stack_end = stack_base.sub(stack_size);

    trace!("stack ends at {:?}", stack_end);

    let stack_end_page = stack_end.sub(PAGESIZE - stack_end.align_offset(PAGESIZE)) as *mut Page;

    #[allow(clippy::let_and_return)]
    let alloc_start = stack_end_page.sub(GUARD_SIZE - 1) as *mut u8;

    alloc_start
}

pub unsafe fn create_guard_for_stack(
    stack_base: *const u8,
    stack_size: usize,
) -> SyscallResult<()> {
    let alloc_start = calc_guard_location(stack_base, stack_size);

    trace!(
        "allocating {} guard pages from {:?} to {:?}",
        GUARD_SIZE,
        alloc_start,
        alloc_start.add(GUARD_SIZE * PAGESIZE - 1)
    );

    let allocation = mmap(
        alloc_start,
        GUARD_SIZE * PAGESIZE,
        MProt::NONE,
        MMapFlags::ANONYMOUS | MMapFlags::PRIVATE | MMapFlags::FIXED_NOREPLACE,
        -1,
        0,
    )?;

    assert_eq!(allocation, alloc_start);

    Ok(())
}

pub unsafe fn free_guard_for_stack(stack_base: *const u8, stack_size: usize) -> SyscallResult<()> {
    let alloc_start = calc_guard_location(stack_base, stack_size);

    trace!(
        "freeing {} guard pages from {:?} to {:?}",
        GUARD_SIZE,
        alloc_start,
        alloc_start.add(GUARD_SIZE * PAGESIZE - 1)
    );

    munmap(alloc_start, GUARD_SIZE * PAGESIZE)?;

    Ok(())
}

unsafe extern "C" fn segv_handler(signal: Signal, signal_info: *mut SignalInfo, _unused: *mut ()) {
    trace!("entered SEGV handler");

    match signal.kind() {
        SignalKind::SEGV => {
            let seg_fault_addr = (*signal_info).inner.sig_fault.addr as *mut u8;

            if (*signal_info).code.segv() == SegvCode::ACCERR {
                let tls = &*crate::tls::get_tls_ptr().expect("Failed to get tls pointer");

                let stack_end = tls.stack_base.sub(tls.stack_limit);

                let seg_fault_addr_page = seg_fault_addr.sub(
                    (crate::PAGESIZE - seg_fault_addr.align_offset(crate::PAGESIZE))
                        % crate::PAGESIZE,
                );

                let n_pages_overshot =
                    stack_end.offset_from(seg_fault_addr_page) / crate::PAGESIZE as isize;

                if (1..=GUARD_SIZE as isize).contains(&n_pages_overshot) {
                    panic!(
                        "Stack Overflow! Overflowed {} byte stack by {:?} bytes (hit guard page {})",
                        tls.stack_limit,
                        stack_end.offset_from(seg_fault_addr),
                        n_pages_overshot
                    );
                }
            }

            let fault_kind = match (*signal_info).code.segv() {
                SegvCode::MAPERR => "memory is not mapped",
                SegvCode::ACCERR => "not allowed",
                SegvCode::Unknown => "unknown cause",
            };

            panic!(
                "Segmentation Fault ({}) at {:?}",
                fault_kind, seg_fault_addr
            );
        }

        _ => {
            panic!("Handler found unexpected event")
        }
    }
}

unsafe extern "C" fn segv_restorer(signal: Signal, signal_info: *mut SignalInfo, _unused: *mut ()) {
    trace!("signal restorer");

    dbg!(signal.kind());
    dbg_p!(*signal_info);

    unreachable!()
}

pub fn setup_segv_handler() -> SyscallResult<()> {
    let action = Sigaction {
        handler: SignalHandler::handler(segv_handler),
        flags: SigactionFlags::ONSTACK | SigactionFlags::SIGINFO | SigactionFlags::RESTORER,
        restorer: SignalHandler::handler(segv_restorer),
        mask: 0,
    };

    unsafe {
        rt_sigaction(
            SignalKind::SEGV.into(),
            &action as *const Sigaction,
            null_mut(),
            core::mem::size_of::<usize>(),
        )
    }
}

pub fn teardown_segv_handler() -> SyscallResult<()> {
    let action = Sigaction::default();

    unsafe {
        rt_sigaction(
            SignalKind::SEGV.into(),
            &action as *const Sigaction,
            null_mut(),
            core::mem::size_of::<usize>(),
        )
    }
}
