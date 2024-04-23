//! Implementation of syscalls
//!
//! The single entry point to all system calls, [`syscall()`], is called
//! whenever userspace wishes to perform a system call using the `ecall`
//! instruction. In this case, the processor raises an 'Environment call from
//! U-mode' exception, which is handled as one of the cases in
//! [`crate::trap::trap_handler`].
//!
//! For clarity, each single syscall is implemented as its own function, named
//! `sys_` then the name of the syscall. You can find functions like this in
//! submodules, and you should also implement syscalls this way.

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;

mod fs;
mod process;

use crate::batch::NUM;
use crate::batch::RE_ADDR;
use fs::*;
use process::*;

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => {
            let start_addr = RE_ADDR.exclusive_access().get_reliable_start();
            let end_addr = RE_ADDR.exclusive_access().get_reliable_end();
            if args[1] < start_addr {
                return -1;
            }
            if args[1] >= end_addr {
                return -1;
            }
            if args[1] + args[2] >= end_addr {
                return -1;
            }
            let ret = sys_write(args[0], args[1] as *const u8, args[2]);
            NUM.exclusive_access().inc_syscall_num(0);
            ret
        }
        SYSCALL_EXIT => {
            NUM.exclusive_access().inc_syscall_num(1);
            sys_exit(args[0] as i32)
        }
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
