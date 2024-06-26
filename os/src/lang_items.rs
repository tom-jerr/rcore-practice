//! The panic handler

use crate::sbi::shutdown;
use core::panic::PanicInfo;
//ch2
use crate::stack_trace::print_stack_trace;
use log::*;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        error!(
            "[kernel] Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        error!("[kernel] Panicked: {}", info.message().unwrap());
    }
    unsafe {
        print_stack_trace();
    }
    shutdown(true)
}
