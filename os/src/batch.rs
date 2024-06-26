//! batch subsystem

use crate::sbi::shutdown;
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use core::arch::asm;
use lazy_static::*;

const USER_STACK_SIZE: usize = 4096 * 2;
const KERNEL_STACK_SIZE: usize = 4096 * 2;
const MAX_APP_NUM: usize = 16;
const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x20000;
// ch2 add begin
const SYSCALL_NUM: usize = 2;
// ch2 add end
#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: KernelStack = KernelStack {
    data: [0; KERNEL_STACK_SIZE],
};
static USER_STACK: UserStack = UserStack {
    data: [0; USER_STACK_SIZE],
};

impl KernelStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    pub fn push_context(&self, cx: TrapContext) -> &'static mut TrapContext {
        let cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *cx_ptr = cx;
        }
        unsafe { cx_ptr.as_mut().unwrap() }
    }
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}
/**
 * AppManager
 */
struct AppManager {
    num_app: usize,
    current_app: usize,
    app_start: [usize; MAX_APP_NUM + 1],
    // ch2 add begin
    app_runtime: [u64; MAX_APP_NUM],
    // ch2 add end
}
// ch2 add begin
/**
 * syscall num
 */
pub struct SyscallNum {
    num: [usize; SYSCALL_NUM],
}

impl SyscallNum {
    /**
     * get syscall num
     */
    pub fn get_syscall_num(&self, syscall_id: usize) -> usize {
        self.num[syscall_id]
    }
    /**
     * inc syscall num
     */
    pub fn inc_syscall_num(&mut self, syscall_id: usize) {
        self.num[syscall_id] += 1;
    }
}

/**
 * Reliable address of program
 */
pub struct ReliableAddr {
    start: usize,
    end: usize,
}

impl ReliableAddr {
    /**
     * get reliable start addr
     */
    pub fn get_reliable_start(&self) -> usize {
        self.start
    }
    /**
     * get reliable end addr
     */
    pub fn get_reliable_end(&self) -> usize {
        self.end
    }
}
// ch2 add end
impl AppManager {
    /**
     * print app info
     */
    pub fn print_app_info(&self) {
        println!("[kernel] num_app = {}", self.num_app);
        for i in 0..self.num_app {
            println!(
                "[kernel] app_{} [{:#x}, {:#x})",
                i,
                self.app_start[i],
                self.app_start[i + 1]
            );
        }
    }

    unsafe fn load_app(&self, app_id: usize) {
        if app_id >= self.num_app {
            println!("All applications completed!");
            shutdown(false);
        }
        println!("[kernel] Loading app_{}", app_id);
        // clear app area
        core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, APP_SIZE_LIMIT).fill(0);
        let app_src = core::slice::from_raw_parts(
            self.app_start[app_id] as *const u8,
            self.app_start[app_id + 1] - self.app_start[app_id],
        );
        let app_dst = core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, app_src.len());
        app_dst.copy_from_slice(app_src);
        // Memory fence about fetching the instruction memory
        // It is guaranteed that a subsequent instruction fetch must
        // observes all previous writes to the instruction memory.
        // Therefore, fence.i must be executed after we have loaded
        // the code of the next app into the instruction memory.
        // See also: riscv non-priv spec chapter 3, 'Zifencei' extension.
        asm!("fence.i");
    }
    /**
     * get current app
     */
    pub fn get_current_app(&self) -> usize {
        self.current_app
    }
    /**
     * move to next app
     */
    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }
    /**
     * time reg set app runtime
     */
    pub fn set_app_runtime(&mut self, app_id: usize, runtime: u64) {
        self.app_runtime[app_id] = runtime;
    }
    /**
     * get app runtime
     */
    pub fn get_app_runtime(&self, app_id: usize) -> u64 {
        self.app_runtime[app_id]
    }
}

lazy_static! {
    /**
     * syscall use app manager
     */
    static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe {
        UPSafeCell::new({
            extern "C" {
                fn _num_app();
            }
            let num_app_ptr = _num_app as usize as *const usize;
            let num_app = num_app_ptr.read_volatile();
            let mut app_start: [usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM + 1];
            let app_start_raw: &[usize] =
                core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1);
            app_start[..=num_app].copy_from_slice(app_start_raw);
            AppManager {
                num_app,
                current_app: 0,
                app_start,
                // ch2 add begin
                app_runtime: [0; MAX_APP_NUM],
                // ch2 add end
            }
        })
    };

    // ch2 add begin
    /**
     * syscall use syscall num
     */
    pub static ref NUM: UPSafeCell<SyscallNum> = unsafe {
        UPSafeCell::new({
            SyscallNum {
                num: [0; SYSCALL_NUM],
            }
        })
    };

    /**
     * reliable addr init
     */
    pub static ref RE_ADDR:UPSafeCell<ReliableAddr> = unsafe {
        UPSafeCell::new({
            ReliableAddr {
                start: APP_BASE_ADDRESS,
                end: APP_BASE_ADDRESS + APP_SIZE_LIMIT,
            }
        })
    };
    // ch2 add end
}

/// init batch subsystem
pub fn init() {
    print_app_info();
}

/// print apps info
pub fn print_app_info() {
    APP_MANAGER.exclusive_access().print_app_info();
}

/// run next app
pub fn run_next_app() -> ! {
    let mut app_manager = APP_MANAGER.exclusive_access();
    let current_app = app_manager.get_current_app();
    // ch2 add begin
    let mut reliable_addr = RE_ADDR.exclusive_access();
    reliable_addr.start = app_manager.app_start[current_app].clone();
    reliable_addr.end = app_manager.app_start[current_app + 1].clone();
    drop(reliable_addr);
    // ch2 add end
    unsafe {
        app_manager.load_app(current_app);
    }
    app_manager.move_to_next_app();
    drop(app_manager);
    // before this we have to drop local variables related to resources manually
    // and release the resources
    extern "C" {
        fn __restore(cx_addr: usize);
    }
    // ch2 add begin

    let time: u64;
    unsafe {
        asm!("rdtime {0}", out(reg) time);
    }
    if current_app > 0 {
        let runtime = time - APP_MANAGER.exclusive_access().app_runtime[current_app - 1];
        APP_MANAGER
            .exclusive_access()
            .set_app_runtime(current_app - 1, runtime);
        println!(
            "[kernel] app_{} runs {} cycles",
            current_app - 1,
            APP_MANAGER
                .exclusive_access()
                .get_app_runtime(current_app - 1)
        );
    }
    if current_app == APP_MANAGER.exclusive_access().num_app - 1 || current_app == 0 {
        println!(
            "sys_write num: {}",
            NUM.exclusive_access().get_syscall_num(0)
        );
        println!(
            "sys_exit num: {}",
            NUM.exclusive_access().get_syscall_num(1)
        );
    }
    // ch2 add end
    unsafe {
        __restore(KERNEL_STACK.push_context(TrapContext::app_init_context(
            APP_BASE_ADDRESS,
            USER_STACK.get_sp(),
        )) as *const _ as usize);
    }
    panic!("Unreachable in batch::run_current_app!");
}
