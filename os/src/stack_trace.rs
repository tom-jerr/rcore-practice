/**
 * stack_trace.rs
 * 打印栈回溯
 */
use core::{arch::asm, ptr};

pub unsafe fn print_stack_trace() -> () {
    let mut fp: *const usize;
    asm!("mv {}, fp", out(reg) fp);

    println!("== Begin Stack Trace ==");
    while fp != ptr::null() {
        let ra = *fp.sub(1);
        let last_fp = *fp.sub(2);

        println!("fp = 0x{:016x}, ra = 0x{:016x}", fp as usize, ra);
        fp = last_fp as *const usize;
    }
    println!("== End Stack Trace ==");
}
