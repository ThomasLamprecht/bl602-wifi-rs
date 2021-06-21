use core::mem::MaybeUninit;

use bl602_hal as hal;
use bl602_hal::timer::ClockSource;
use embedded_time::duration::Milliseconds;
use hal::interrupts::TrapFrame;
use hal::prelude::Extensions;
use hal::timer::{ConfiguredTimerChannel0, TimerChannel0};

use crate::compat::set_time_source;
use crate::preemt::{task_create, task_switch};
use crate::wifi::{wifi_worker_task1, wifi_worker_task2};

static mut CH0: MaybeUninit<ConfiguredTimerChannel0> = MaybeUninit::uninit();

static mut TIME_MS: u32 = 0;

pub fn wifi_timer_init(channel0: TimerChannel0) {
    let ch0 = channel0.set_clock_source(ClockSource::Clock1Khz, 1_000u32.Hz());
    ch0.enable_match0_interrupt();
    ch0.set_preload_value(Milliseconds::new(0));
    ch0.set_preload(hal::timer::Preload::PreloadMatchComparator0);
    ch0.set_match0(Milliseconds::new(10u32));

    hal::interrupts::enable_interrupt(hal::interrupts::Interrupt::TimerCh0);
    unsafe {
        *(CH0.as_mut_ptr()) = ch0;
    }

    set_time_source(get_time);

    task_create(wifi_worker_task1);
    task_create(wifi_worker_task2);

    get_ch0().enable(); // start timer for tasks

    unsafe {
        riscv::interrupt::enable();
    }
}

pub fn timestamp() -> smoltcp::time::Instant {
    smoltcp::time::Instant::from_millis(get_time().0)
}

#[allow(non_snake_case)]
#[no_mangle]
fn TimerCh0(trap_frame: &mut TrapFrame) {
    unsafe {
        TIME_MS += 20; // the counter is already reset - all we need is a monotonic timer
    }

    get_ch0().clear_match0_interrupt();
    task_switch(trap_frame);
}

fn get_ch0() -> &'static mut ConfiguredTimerChannel0 {
    unsafe { &mut *CH0.as_mut_ptr() }
}

fn get_time() -> Milliseconds {
    unsafe { riscv::interrupt::free(|_| get_ch0().current_time() + Milliseconds(TIME_MS as u32)) }
}
