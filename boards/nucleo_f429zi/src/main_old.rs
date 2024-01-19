//! Board file for Nucleo-F429ZI development board
//!
//! - <https://www.st.com/en/evaluation-tools/nucleo-f429zi.html>

#![no_std]
// Disable this attribute when documenting, as a workaround for
// https://github.com/rust-lang/rust/issues/62184.
#![cfg_attr(not(doc), no_main)]
#![deny(missing_docs)]
#![feature(core_intrinsics)]

use kernel::hil::gpio::FloatingState;
use kernel::hil::led::LedLow;
use capsules_core::virtualizers::virtual_alarm::VirtualMuxAlarm;
use components::gpio::GpioComponent;
use kernel::capabilities;
use kernel::component::Component;
use kernel::deferred_call::{DeferredCall, DeferredCallClient};
use kernel::platform::{KernelResources, SyscallDriverLookup};
use kernel::scheduler::round_robin::RoundRobinSched;
use kernel::{create_capability, debug, static_init};

use capsules_core::virtualizers::virtual_i2c::MuxI2C;
use kernel::hil::i2c::I2CMaster;
use stm32f429zi::gpio::{AlternateFunction, Mode, PinId, PortId};
use stm32f429zi::interrupt_service::Stm32f429ziDefaultPeripherals;
use stm32f429zi::{rcc, rcc::Rcc};
use stm32f4xx::{flash, i2c, pwr, utils};
use stm32f429zi::pwr;

use core::intrinsics::breakpoint;
/// Support routines for debugging I/O.
pub mod io;

// Number of concurrent processes this platform supports.
const NUM_PROCS: usize = 4;

// Actual memory for holding the active process structures.
static mut PROCESSES: [Option<&'static dyn kernel::process::Process>; NUM_PROCS] =
    [None, None, None, None];

static mut CHIP: Option<&'static stm32f429zi::chip::Stm32f4xx<Stm32f429ziDefaultPeripherals>> =
    None;
static mut PROCESS_PRINTER: Option<&'static kernel::process::ProcessPrinterText> = None;

// How should the kernel respond when a process faults.
const FAULT_RESPONSE: kernel::process::PanicFaultPolicy = kernel::process::PanicFaultPolicy {};

/// Dummy buffer that causes the linker to reserve enough space for the stack.
#[no_mangle]
#[link_section = ".stack_buffer"]
pub static mut STACK_MEMORY: [u8; 0x2000] = [0; 0x2000];

/// A structure representing this platform that holds references to all
/// capsules for this platform.
struct NucleoF429ZI {
    console: &'static capsules_core::console::Console<'static>,
    ipc: kernel::ipc::IPC<{ NUM_PROCS as u8 }>,
    led: &'static capsules_core::led::LedDriver<
        'static,
        LedLow<'static, stm32f429zi::gpio::Pin<'static>>,
        3,
    >,
    button: &'static capsules_core::button::Button<'static, stm32f429zi::gpio::Pin<'static>>,
    adc: &'static capsules_core::adc::AdcVirtualized<'static>,
    alarm: &'static capsules_core::alarm::AlarmDriver<
        'static,
        VirtualMuxAlarm<'static, stm32f429zi::tim2::Tim2<'static>>,
    >,
    temperature: &'static capsules_extra::temperature::TemperatureSensor<'static>,
    gpio: &'static capsules_core::gpio::GPIO<'static, stm32f429zi::gpio::Pin<'static>>,
    rng: &'static capsules_core::rng::RngDriver<'static>,

    scheduler: &'static RoundRobinSched<'static>,
    systick: cortexm4::systick::SysTick,

    display: &'static capsules_extra::graphic_display::GraphicDisplay<'static>,
    ipc_comm: &'static kernel::ipc::IPC<00000>,
    pwm: &'static capsules_extra::pwm::Pwm<'static, 1>,
    measurement: &'static capsules_extra::measurement::MeasurementCapsule< //todo implemente capsule
        'static,
        stm32f4xx::tim3::Tim2_5<'static>, //todo check
    >,
    can: &'static capsules_extra::can::CanCapsule<'static, stm32f429zi::can::Can<'static>>,
}

/// Mapping of integer syscalls to objects that implement syscalls.
impl SyscallDriverLookup for NucleoF429ZI {
    fn with_driver<F, R>(&self, driver_num: usize, f: F) -> R
    where
        F: FnOnce(Option<&dyn kernel::syscall::SyscallDriver>) -> R,
    {
        match driver_num {
            capsules_core::console::DRIVER_NUM => f(Some(self.console)),
            capsules_core::led::DRIVER_NUM => f(Some(self.led)),
            capsules_core::button::DRIVER_NUM => f(Some(self.button)),
            capsules_core::adc::DRIVER_NUM => f(Some(self.adc)),
            capsules_core::alarm::DRIVER_NUM => f(Some(self.alarm)),
            capsules_extra::temperature::DRIVER_NUM => f(Some(self.temperature)),
            kernel::ipc::DRIVER_NUM => f(Some(&self.ipc)),
            capsules_core::gpio::DRIVER_NUM => f(Some(self.gpio)),
            capsules_core::rng::DRIVER_NUM => f(Some(self.rng)),
            capsules_extra::graphic_display::DRIVER_NUM => f(Some(self.display)),
            kernel::ipc::DRIVER_NUM => f(Some(self.ipc_comm)),
            capsules_extra::pwm::DRIVER_NUM => f(Some(self.pwm)),
            capsules_extra::measurement::DRIVER_NUM => f(Some(self.measurement)), // impl measurement
            capsules_extra::can::DRIVER_NUM => f(Some(self.can)),
            _ => f(None),
        }
    }
}

impl
    KernelResources<
        stm32f429zi::chip::Stm32f4xx<
            'static,
            stm32f429zi::interrupt_service::Stm32f429ziDefaultPeripherals<'static>,
        >,
    > for NucleoF429ZI
{
    type SyscallDriverLookup = Self;
    type SyscallFilter = ();
    type ProcessFault = ();
    type CredentialsCheckingPolicy = ();
    type Scheduler = RoundRobinSched<'static>;
    type SchedulerTimer = cortexm4::systick::SysTick;
    type WatchDog = ();
    type ContextSwitchCallback = ();

    fn syscall_driver_lookup(&self) -> &Self::SyscallDriverLookup {
        self
    }
    fn syscall_filter(&self) -> &Self::SyscallFilter {
        &()
    }
    fn process_fault(&self) -> &Self::ProcessFault {
        &()
    }
    fn credentials_checking_policy(&self) -> &'static Self::CredentialsCheckingPolicy {
        &()
    }
    fn scheduler(&self) -> &Self::Scheduler {
        self.scheduler
    }
    fn scheduler_timer(&self) -> &Self::SchedulerTimer {
        &self.systick
    }
    fn watchdog(&self) -> &Self::WatchDog {
        &()
    }
    fn context_switch_callback(&self) -> &Self::ContextSwitchCallback {
        &()
    }
}

/// Helper function called during bring-up that configures DMA.
unsafe fn setup_dma(
    dma: &stm32f429zi::dma::Dma1,
    dma_streams: &'static [stm32f429zi::dma::Stream<stm32f429zi::dma::Dma1>; 8],
    usart3: &'static stm32f429zi::usart::Usart<stm32f429zi::dma::Dma1>,
) {
    use stm32f429zi::dma::Dma1Peripheral;
    use stm32f429zi::usart;

    dma.enable_clock();

    let usart3_tx_stream = &dma_streams[Dma1Peripheral::USART3_TX.get_stream_idx()];
    let usart3_rx_stream = &dma_streams[Dma1Peripheral::USART3_RX.get_stream_idx()];

    usart3.set_dma(
        usart::TxDMA(usart3_tx_stream),
        usart::RxDMA(usart3_rx_stream),
    );

    usart3_tx_stream.set_client(usart3);
    usart3_rx_stream.set_client(usart3);

    usart3_tx_stream.setup(Dma1Peripheral::USART3_TX);
    usart3_rx_stream.setup(Dma1Peripheral::USART3_RX);

    cortexm4::nvic::Nvic::new(Dma1Peripheral::USART3_TX.get_stream_irqn()).enable();
    cortexm4::nvic::Nvic::new(Dma1Peripheral::USART3_RX.get_stream_irqn()).enable();
}

/// Helper function called during bring-up that configures multiplexed I/O.
unsafe fn set_pin_primary_functions(
    syscfg: &stm32f429zi::syscfg::Syscfg,
    gpio_ports: &'static stm32f429zi::gpio::GpioPorts<'static>,
    i2c1: &stm32f4xx::i2c::I2C,
) {
    use apis::gpio::Configure;

    syscfg.enable_clock();

    gpio_ports.get_port_from_port_id(PortId::B).enable_clock();

    // User LD2 is connected to PB07. Configure PB07 as `debug_gpio!(0, ...)`
    if let Some(pin) = gpio_ports.get_pin(PinId::PB07) {
        pin.make_output();

        // Configure kernel debug gpios as early as possible
        kernel::debug::assign_gpios(Some(pin), None, None);
    };

    gpio_ports.get_port_from_port_id(PortId::D).enable_clock();

    // pd8 and pd9 (USART3) is connected to ST-LINK virtual COM port
    if let Some(pin) = gpio_ports.get_pin(PinId::PD08) {
        pin.set_mode(Mode::AlternateFunctionMode);
        // AF7 is USART2_TX
        pin.set_alternate_function(AlternateFunction::AF7);
    };
    if let Some(pin) = gpio_ports.get_pin(PinId::PD09) {
        pin.set_mode(Mode::AlternateFunctionMode);
        // AF7 is USART2_RX
        pin.set_alternate_function(AlternateFunction::AF7);
    };

    gpio_ports.get_port_from_port_id(PortId::C).enable_clock();

    // button is connected on pa6
    if let Some(pin) = gpio_ports.get_pin(PinId::PA06) {
        pin.set_mode(Mode::Input);
        pin.enable_interrupt();
    }

    // set interrupt for pin D0
    if let Some(pin) = gpio_ports.get_pin(PinId::PG09) {
        pin.enable_interrupt();
    }

    // Enable clocks for GPIO Ports
    // Disable some of them if you don't need some of the GPIOs
    gpio_ports.get_port_from_port_id(PortId::A).enable_clock();
    // Ports B, C and D are already enabled
    gpio_ports.get_port_from_port_id(PortId::E).enable_clock();
    gpio_ports.get_port_from_port_id(PortId::F).enable_clock();
    gpio_ports.get_port_from_port_id(PortId::G).enable_clock();
    gpio_ports.get_port_from_port_id(PortId::H).enable_clock();

    // Arduino A0
    if let Some(pin) = gpio_ports.get_pin(PinId::PC00) {
        pin.set_mode(stm32f429zi::gpio::Mode::AnalogMode);
    }

    // // Arduino A1
    // if let Some(pin) = gpio_ports.get_pin(PinId::PC00) {
    //     pin.set_mode(stm32f429zi::gpio::Mode::AnalogMode);
    // }

    // // Arduino A2
    // if let Some(pin) = gpio_ports.get_pin(PinId::PC03) {
    //     pin.set_mode(stm32f429zi::gpio::Mode::AnalogMode);
    // }

    // // Arduino A6
    // if let Some(pin) = gpio_ports.get_pin(PinId::PB01) {
    //     pin.set_mode(stm32f429zi::gpio::Mode::AnalogMode);
    // }

    // // Arduino A7
    // if let Some(pin) = gpio_ports.get_pin(PinId::PC02) {
    //     pin.set_mode(stm32f429zi::gpio::Mode::AnalogMode);
    // }

    if let Some(pin) = gpio_ports.get_pin(PinId::PB08) {
        pin.set_mode(Mode::AlternateFunctionMode);
        pin.set_floating_state(kernel::hil::gpio::FloatingState::PullNone);
        pin.set_mode_output_opendrain();
        // AF4 is I2C
        pin.set_speed();
        pin.set_alternate_function(AlternateFunction::AF4);
    };
    if let Some(pin) = gpio_ports.get_pin(PinId::PB09) {
        pin.make_output();
        pin.set_floating_state(kernel::hil::gpio::FloatingState::PullNone);
        pin.set_speed();
        pin.set_mode(Mode::AlternateFunctionMode);
        pin.set_mode_output_opendrain();
        // AF4 is I2C
        pin.set_alternate_function(AlternateFunction::AF4);
    };

    i2c1.enable_clock();
    i2c1.set_speed(i2c::I2CSpeed::Speed100k, 8);

    // PE11 is connected to motor driver IN2
    gpio_ports.get_pin(PinId::PE11).map(|pin| {
        pin.set_mode(Mode::AlternateFunctionMode);
        // AF1 is TIM1/2
        pin.set_alternate_function(AlternateFunction::AF1);
    });

    // PE13 is connected to motor driver IN1
    gpio_ports.get_pin(PinId::PE13).map(|pin| {
        pin.set_mode(Mode::GeneralPurposeOutputMode);
        // pin.set_mode(Mode::AlternateFunctionMode);
        // // AF1 is TIM1/2
        // pin.set_alternate_function(AlternateFunction::AF1);
    });

    // PA7 is Encoder pulse counting
    gpio_ports.get_pin(PinId::PA07).map(|pin| {
        pin.set_mode(Mode::AlternateFunctionMode);
        pin.set_floating_state(FloatingState::PullDown);
        //pin.set_mode(Mode::AlternateFunctionMode);
        pin.set_alternate_function(AlternateFunction::AF2);
    });

    // PA3 is Encoder pulse counting
    gpio_ports.get_pin(PinId::PA03).map(|pin| {
        pin.make_input();
        pin.set_floating_state(FloatingState::PullNone);
    });

    // PA08 is Clock Output MCO1
    gpio_ports.get_pin(PinId::PA08).map(|pin| {
        pin.set_mode(Mode::AlternateFunctionMode);
        // AF0 is Clock output (MCO1)
        pin.set_alternate_function(AlternateFunction::AF0);
    });

    // PC09 is ClockOutput MCO2
    gpio_ports.get_pin(PinId::PC09).map(|pin| {
        pin.set_mode(Mode::AlternateFunctionMode);
        pin.set_alternate_function(AlternateFunction::AF0);
    });

    gpio_ports.get_pin(PinId::PD00).map(|pin| {
        pin.set_mode(Mode::AlternateFunctionMode);
        // AF9 is CAN_RX
        pin.set_alternate_function(AlternateFunction::AF9);
        pin.set_floating_state(kernel::hil::gpio::FloatingState::PullNone);
    });
    gpio_ports.get_pin(PinId::PD01).map(|pin| {
        pin.set_mode(Mode::AlternateFunctionMode);
        // AF9 is CAN_TX
        pin.set_alternate_function(AlternateFunction::AF9);
    });

    gpio_ports.get_pin(PinId::PE08).map(|pin| {
        pin.make_output();
    });
}

/// Helper function for miscellaneous peripheral functions
unsafe fn setup_peripherals(
    tim2: &stm32f429zi::tim2::Tim2,
    trng: &stm32f429zi::trng::Trng,
    tim1: &stm32f4xx::tim1::Tim1_8,
    tim3: &stm32f4xx::tim3::Tim2_5,
    can1: &'static stm32f429zi::can::Can,
) {
    // USART3 IRQn is 39
    cortexm4::nvic::Nvic::new(stm32f429zi::nvic::USART3).enable();

    // TIM2 IRQn is 28
    tim2.enable_clock();
    tim2.start();
    cortexm4::nvic::Nvic::new(stm32f429zi::nvic::TIM2).enable();

    // RNG
    trng.enable_clock();

    tim1.enable_clock();
    tim1.start();
    // tim1.start_pwm(&stm32f4xx::tim1::TimOutputs::OutputCh2, 2000, 800);
    tim3.enable_clock();
    tim3.start();

    // CAN
    can1.enable_clock();
}

/// Statically initialize the core peripherals for the chip.
///
/// This is in a separate, inline(never) function so that its stack frame is
/// removed when this function returns. Otherwise, the stack space used for
/// these static_inits is wasted.
#[inline(never)]
unsafe fn get_peripherals() -> (
    &'static mut Stm32f429ziDefaultPeripherals<'static>,
    &'static stm32f429zi::syscfg::Syscfg<'static>,
    &'static stm32f429zi::dma::Dma1<'static>,
    &'static stm32f429zi::rcc::Rcc,
) {
    // We use the default HSI 16Mhz clock
    let rcc = static_init!(stm32f429zi::rcc::Rcc, stm32f429zi::rcc::Rcc::new());
    let syscfg = static_init!(
        stm32f429zi::syscfg::Syscfg,
        stm32f429zi::syscfg::Syscfg::new(rcc)
    );
    let exti = static_init!(
        stm32f429zi::exti::Exti,
        stm32f429zi::exti::Exti::new(syscfg)
    );
    let dma1 = static_init!(stm32f429zi::dma::Dma1, stm32f429zi::dma::Dma1::new(rcc));
    let dma2 = static_init!(stm32f429zi::dma::Dma2, stm32f429zi::dma::Dma2::new(rcc));

    let peripherals = static_init!(
        Stm32f429ziDefaultPeripherals,
        Stm32f429ziDefaultPeripherals::new(rcc, exti, dma1, dma2)
    );
    (peripherals, syscfg, dma1, rcc)
}

/// Initialize clock tree and related peripherals (flash, pwr)
///
/// In order to start the PLL, the pwr scaling needs to be configured. In order to increase the system clock to 180MHz, the flash latency needs to be increased and pwr overdrive needs to be enabled.
/// see: RM0090 ch. 5.1.4 pg 123
unsafe fn try_init_clocks(rcc: &Rcc) -> Result<(), ()> {
    // only PWR peripheral should be enabled, HSI -> SystemClock
    rcc::enable_boot_esential_clocks()?;
    pwr::configure_voltage_scaling(1)?;

    // uncomment these lines in order to measure internal clocks
    rcc.configure_prescaler(&rcc::RccClockDestination::MClockOutput1, 5);
    rcc.configure_prescaler(&rcc::RccClockDestination::MClockOutput2, 5);

    // HSI -> PLL, wait for HSI to be stable
    utils::wait_for_condition(
        1_000,
        || rcc.is_clock_stable(&rcc::RccClockSource::HsiRc),
        || (),
    )?;

    // HSI = 16MHz
    // PLL output = (16/8)*180/2 = 180 MHz
    rcc.configure_pll(8, 180, 2);
    rcc.enable(rcc::RccClockSource::Pll);

    pwr::enable_overdrive()?;
    flash::set_latency(5);

    let _ = utils::wait_for_condition(
        1_000,
        || rcc.is_clock_stable(&rcc::RccClockSource::Pll),
        || (),
    );
    rcc.configure_prescaler(&rcc::RccClockDestination::Apb1, 4);
    rcc.configure_prescaler(&rcc::RccClockDestination::Apb2, 4);
    rcc.route(
        &rcc::RccClockSource::Pll,
        &rcc::RccClockDestination::SystemClock,
    )?;

    Ok(())
}

/// Initialize clock tree, if it fails it reverts to default configuration (HSI-> SystemClcok)
pub unsafe fn init_clocks(rcc: &Rcc) {
    match try_init_clocks(&rcc) {
        Ok(_) => {}
        Err(_) => {
            let _ = rcc.route(
                &rcc::RccClockSource::HsiRc,
                &rcc::RccClockDestination::SystemClock,
            );
        }
    }
    if rcc.compute_nominal_frequency().is_err() {
        breakpoint();
    }
}

/// Main function.
///
/// # Safety
/// This function needs to be marked unsafe allows ...
///
/// This is called after RAM initialization is complete.
#[no_mangle]
pub unsafe fn main() {
    stm32f429zi::init();

    let (peripherals, syscfg, dma1, rcc) = get_peripherals();
    peripherals.init();

    init_clocks(rcc);
    let base_peripherals = &peripherals.stm32f4;

    setup_peripherals(
        &base_peripherals.tim2,
        &peripherals.trng,
        &base_peripherals.tim1,
        &base_peripherals.tim3,
        &peripherals.can1,
    );

    set_pin_primary_functions(
        syscfg,
        &base_peripherals.gpio_ports,
        &peripherals.stm32f4.i2c1,
    );

    setup_dma(
        dma1,
        &base_peripherals.dma1_streams,
        &base_peripherals.usart3,
    );

    let board_kernel = static_init!(kernel::Kernel, kernel::Kernel::new(&PROCESSES));

    let dynamic_deferred_call_clients =
        static_init!([DynamicDeferredCallClientState; 3], Default::default());
    let dynamic_deferred_caller = static_init!(
        DynamicDeferredCall,
        DynamicDeferredCall::new(dynamic_deferred_call_clients)
    );
    DynamicDeferredCall::set_global_instance(dynamic_deferred_caller);

    let chip = static_init!(
        stm32f429zi::chip::Stm32f4xx<Stm32f429ziDefaultPeripherals>,
        stm32f429zi::chip::Stm32f4xx::new(peripherals)
    );
    CHIP = Some(chip);

    // UART

    // Create a shared UART channel for kernel debug.
    base_peripherals.usart3.enable_clock();
    let uart_mux = components::console::UartMuxComponent::new(
        &base_peripherals.usart3,
        115200,
        dynamic_deferred_caller,
    )
    .finalize(components::uart_mux_component_static!());

    io::WRITER.set_initialized();

    // Create capabilities that the board needs to call certain protected kernel
    // functions.
    let memory_allocation_capability = create_capability!(capabilities::MemoryAllocationCapability);
    let main_loop_capability = create_capability!(capabilities::MainLoopCapability);
    let process_management_capability =
        create_capability!(capabilities::ProcessManagementCapability);

    // Setup the console.
    let console = components::console::ConsoleComponent::new(
        board_kernel,
        capsules::console::DRIVER_NUM,
        uart_mux,
    )
    .finalize(components::console_component_static!());
    // Create the debugger object that handles calls to `debug!()`.
    components::debug_writer::DebugWriterComponent::new(uart_mux)
        .finalize(components::debug_writer_component_static!());

    // LEDs

    // Clock to Port A is enabled in `set_pin_primary_functions()`
    let gpio_ports = &base_peripherals.gpio_ports;

    let led = components::led::LedsComponent::new().finalize(components::led_component_static!(
        LedLow<'static, stm32f429zi::gpio::Pin>,
        LedLow::new(gpio_ports.get_pin(stm32f429zi::gpio::PinId::PD15).unwrap()),
        LedLow::new(gpio_ports.get_pin(stm32f429zi::gpio::PinId::PD14).unwrap()),
        LedLow::new(gpio_ports.get_pin(stm32f429zi::gpio::PinId::PA05).unwrap()),
    ));

    // LedHigh::new(gpio_ports.get_pin(stm32f429zi::gpio::PinId::PB00).unwrap()),
    // LedHigh::new(gpio_ports.get_pin(stm32f429zi::gpio::PinId::PB07).unwrap()),
    // LedHigh::new(gpio_ports.get_pin(stm32f429zi::gpio::PinId::PB14).unwrap()),

    // BUTTONs
    let button = components::button::ButtonComponent::new(
        board_kernel,
        capsules::button::DRIVER_NUM,
        components::button_component_helper!(
            stm32f429zi::gpio::Pin,
            (
                gpio_ports.get_pin(stm32f429zi::gpio::PinId::PA06).unwrap(),
                apis::gpio::ActivationMode::ActiveHigh,
                apis::gpio::FloatingState::PullDown
            )
        ),
    )
    .finalize(components::button_component_static!(stm32f429zi::gpio::Pin));

    // ALARM

    let tim2 = &base_peripherals.tim2;
    let mux_alarm = components::alarm::AlarmMuxComponent::new(tim2).finalize(
        components::alarm_mux_component_static!(stm32f429zi::tim2::Tim2),
    );

    let alarm = components::alarm::AlarmDriverComponent::new(
        board_kernel,
        capsules::alarm::DRIVER_NUM,
        mux_alarm,
    )
    .finalize(components::alarm_component_static!(stm32f429zi::tim2::Tim2));

    // GPIO
    let gpio = GpioComponent::new(
        board_kernel,
        capsules::gpio::DRIVER_NUM,
        components::gpio_component_helper!(
            stm32f429zi::gpio::Pin,
            // Arduino like RX/TX
            0 => gpio_ports.get_pin(PinId::PG09).unwrap(), //D0
            1 => gpio_ports.pins[6][14].as_ref().unwrap(), //D1
            2 => gpio_ports.pins[5][15].as_ref().unwrap(), //D2
            3 => gpio_ports.pins[4][13].as_ref().unwrap(), //D3
            4 => gpio_ports.pins[5][14].as_ref().unwrap(), //D4
            // 5 => gpio_ports.pins[4][11].as_ref().unwrap(), //D5
            // 6 => gpio_ports.pins[4][9].as_ref().unwrap(), //D6
            7 => gpio_ports.pins[5][13].as_ref().unwrap(), //D7
            8 => gpio_ports.pins[5][12].as_ref().unwrap(), //D8
            // 9 => gpio_ports.pins[3][15].as_ref().unwrap(), //D9
            // SPI Pins
            // 10 => gpio_ports.pins[3][14].as_ref().unwrap(), //D10
            // 11 => gpio_ports.pins[0][7].as_ref().unwrap(),  //D11
            // 12 => gpio_ports.pins[0][6].as_ref().unwrap(),  //D12
            // 13 => gpio_ports.pins[0][5].as_ref().unwrap(),  //D13
            // I2C Pins
            14 => gpio_ports.pins[1][9].as_ref().unwrap(), //D14
            15 => gpio_ports.pins[1][8].as_ref().unwrap(), //D15
            16 => gpio_ports.pins[2][6].as_ref().unwrap(), //D16
            17 => gpio_ports.pins[1][15].as_ref().unwrap(), //D17
            18 => gpio_ports.pins[1][13].as_ref().unwrap(), //D18
            19 => gpio_ports.pins[1][12].as_ref().unwrap(), //D19
            20 => gpio_ports.pins[0][15].as_ref().unwrap(), //D20
            21 => gpio_ports.pins[2][7].as_ref().unwrap(), //D21
            // SPI B Pins
            // 22 => gpio_ports.pins[1][5].as_ref().unwrap(), //D22
            // 23 => gpio_ports.pins[1][3].as_ref().unwrap(), //D23
            // 24 => gpio_ports.pins[0][4].as_ref().unwrap(), //D24
            // 24 => gpio_ports.pins[1][4].as_ref().unwrap(), //D25
            // QSPI
            26 => gpio_ports.pins[1][6].as_ref().unwrap(), //D26
            27 => gpio_ports.pins[1][2].as_ref().unwrap(), //D27
            28 => gpio_ports.pins[3][13].as_ref().unwrap(), //D28
            29 => gpio_ports.pins[3][12].as_ref().unwrap(), //D29
            30 => gpio_ports.pins[3][11].as_ref().unwrap(), //D30
            31 => gpio_ports.pins[4][2].as_ref().unwrap(), //D31
            // Timer Pins
            32 => gpio_ports.pins[0][0].as_ref().unwrap(), //D32
            33 => gpio_ports.pins[1][0].as_ref().unwrap(), //D33
            34 => gpio_ports.pins[4][0].as_ref().unwrap(), //D34
            35 => gpio_ports.pins[1][11].as_ref().unwrap(), //D35
            36 => gpio_ports.pins[1][10].as_ref().unwrap(), //D36
            37 => gpio_ports.pins[4][15].as_ref().unwrap(), //D37
            38 => gpio_ports.pins[4][14].as_ref().unwrap(), //D38
            39 => gpio_ports.pins[4][12].as_ref().unwrap(), //D39
            40 => gpio_ports.pins[4][10].as_ref().unwrap(), //D40
            41 => gpio_ports.pins[4][7].as_ref().unwrap(), //D41
            42 => gpio_ports.pins[4][8].as_ref().unwrap(), //D42
            // SDMMC
            43 => gpio_ports.pins[2][8].as_ref().unwrap(), //D43
            44 => gpio_ports.pins[2][9].as_ref().unwrap(), //D44
            45 => gpio_ports.pins[2][10].as_ref().unwrap(), //D45
            46 => gpio_ports.pins[2][11].as_ref().unwrap(), //D46
            47 => gpio_ports.pins[2][12].as_ref().unwrap(), //D47
            48 => gpio_ports.pins[3][2].as_ref().unwrap(), //D48
            49 => gpio_ports.pins[6][2].as_ref().unwrap(), //D49
            50 => gpio_ports.pins[6][3].as_ref().unwrap(), //D50
            // USART
            51 => gpio_ports.pins[3][7].as_ref().unwrap(), //D51
            52 => gpio_ports.pins[3][6].as_ref().unwrap(), //D52
            53 => gpio_ports.pins[3][5].as_ref().unwrap(), //D53
            54 => gpio_ports.pins[3][4].as_ref().unwrap(), //D54
            55 => gpio_ports.pins[3][3].as_ref().unwrap(), //D55
            56 => gpio_ports.pins[4][2].as_ref().unwrap(), //D56
            57 => gpio_ports.pins[4][4].as_ref().unwrap(), //D57
            58 => gpio_ports.pins[4][5].as_ref().unwrap(), //D58
            59 => gpio_ports.pins[4][6].as_ref().unwrap(), //D59
            60 => gpio_ports.pins[4][3].as_ref().unwrap(), //D60
            61 => gpio_ports.pins[5][8].as_ref().unwrap(), //D61
            62 => gpio_ports.pins[5][7].as_ref().unwrap(), //D62
            63 => gpio_ports.pins[5][9].as_ref().unwrap(), //D63
            64 => gpio_ports.pins[6][1].as_ref().unwrap(), //D64
            65 => gpio_ports.pins[6][0].as_ref().unwrap(), //D65
            // 66 => gpio_ports.pins[3][1].as_ref().unwrap(), //D66
            // 67 => gpio_ports.pins[3][0].as_ref().unwrap(), //D67
            68 => gpio_ports.pins[5][0].as_ref().unwrap(), //D68
            69 => gpio_ports.pins[5][1].as_ref().unwrap(), //D69
            70 => gpio_ports.pins[5][2].as_ref().unwrap(), //D70
            71 => gpio_ports.pins[0][7].as_ref().unwrap(),  //D71

            // ADC Pins
            // Enable the to use the ADC pins as GPIO
            // 72 => gpio_ports.pins[0][3].as_ref().unwrap(), //A0
            // 73 => gpio_ports.pins[2][0].as_ref().unwrap(), //A1
            // 74 => gpio_ports.pins[2][3].as_ref().unwrap(), //A2
            // 75 => gpio_ports.pins[5][3].as_ref().unwrap(), //A3
            76 => gpio_ports.pins[5][5].as_ref().unwrap(), //A4
            77 => gpio_ports.pins[5][10].as_ref().unwrap(), //A5
            // 78 => gpio_ports.pins[1][1].as_ref().unwrap(), //A6
            // 79 => gpio_ports.pins[2][2].as_ref().unwrap(), //A7
            80 => gpio_ports.pins[5][4].as_ref().unwrap()  //A8
        ),
    )
    .finalize(components::gpio_component_static!(stm32f429zi::gpio::Pin));

    // ADC
    let adc_mux = components::adc::AdcMuxComponent::new(&base_peripherals.adc1)
        .finalize(components::adc_mux_component_static!(stm32f429zi::adc::Adc));

    let temp_sensor = components::temperature_stm::TemperatureSTMComponent::new(
        adc_mux,
        stm32f429zi::adc::Channel::Channel18,
        2.5,
        0.76,
    )
    .finalize(components::temperature_stm_adc_component_static!(
        stm32f429zi::adc::Adc
    ));
    let grant_cap = create_capability!(capabilities::MemoryAllocationCapability);
    let grant_temperature =
        board_kernel.create_grant(capsules_extra::temperature::DRIVER_NUM, &grant_cap);

    let temp = static_init!(
        capsules_extra::temperature::TemperatureSensor<'static>,
        capsules_extra::temperature::TemperatureSensor::new(temp_sensor, grant_temperature)
    );
    kernel::hil::sensors::TemperatureDriver::set_client(temp_sensor, temp);

    let adc_channel_0 =
        components::adc::AdcComponent::new(adc_mux, stm32f429zi::adc::Channel::Channel10)
            .finalize(components::adc_component_static!(stm32f429zi::adc::Adc));

    let adc_syscall =
        components::adc::AdcVirtualComponent::new(board_kernel, capsules_core::adc::DRIVER_NUM)
            .finalize(components::adc_syscall_component_helper!(
                adc_channel_0,
            ));

    let process_printer = components::process_printer::ProcessPrinterTextComponent::new()
        .finalize(components::process_printer_text_component_static!());
    PROCESS_PRINTER = Some(process_printer);

    // RNG
    let rng = components::rng::RngComponent::new(
        board_kernel,
        capsules_core::rng::DRIVER_NUM,
        &peripherals.trng,
    )
    .finalize(components::rng_component_static!());

    // PROCESS CONSOLE
    let process_console = components::process_console::ProcessConsoleComponent::new(
        board_kernel,
        uart_mux,
        mux_alarm,
        process_printer,
        None,
    )
    .finalize(components::process_console_component_static!(
        stm32f429zi::tim2::Tim2
    ));
    let _ = process_console.start();

    let ipc_comm =
        driver_setup::ipc::IPCComponent::new(board_kernel, drivers::communication::DRIVER_NUM)
            .finalize(driver_setup::ipc_component_static!());

    let can = driver_setup::can::CanComponent::new(
        board_kernel,
        capsules_extra::can::DRIVER_NUM,
        &peripherals.can1,
    )
    .finalize(components::can_component_static!(
        stm32f429zi::can::Can<'static>
    ));

    // I2C SSD1306 screen
    let i2c_mux = static_init!(
        MuxI2C<'static>,
        MuxI2C::new(&peripherals.stm32f4.i2c1, None, dynamic_deferred_caller)
    );
    peripherals.stm32f4.i2c1.set_master_client(i2c_mux);

    let bus = components::bus::I2CMasterBusComponent::new(
        i2c_mux,
        capsules_extra::ssd1306::SLAVE_ADDRESS_WRITE,
    )
    .finalize(components::i2c_master_bus_component_static!(2));

    let ssd1306_screen = components::ssd1306::SSD1306Component::new(bus, dynamic_deferred_caller)
        .finalize(components::ssd1306_component_static!(
            capsules_extra::bus::I2CMasterBus<'static, capsules_core::virtualizers::virtual_i2c::I2CDevice<'static>>,
        ));

    let _ = ssd1306_screen.init();

    let display = components::graphic_display::GraphicDisplayComponent::new(
        board_kernel,
        capsules_extra::graphic_display::DRIVER_NUM,
        ssd1306_screen,
        Some(ssd1306_screen),
    )
    .finalize(components::graphic_display_component_static!(1025));

    let mux_pwm = components::pwm::PwmMuxComponent::new(&base_peripherals.tim1).finalize(
        components::pwm_mux_component_static!(stm32f4xx::tim1::Tim1_8),
    );

    let pwm = components::pwm::PwmVirtualComponent::new(board_kernel, capsules_extra::pwm::DRIVER_NUM)
        .finalize(components::pwm_syscall_component_helper!(
            components::pwm::PwmPinComponent::new(
                &mux_pwm,
                stm32f4xx::tim1::TimOutputs::OutputCh2,
                //gpio_ports.get_pin(stm32f429zi: gpio::PinId::PE11).unwrap()
            )
            .finalize(components::pwm_pin_user_component_static!(
                stm32f4xx::tim1::Tim1_8
            ))
        ));

    let measurement = components::measurement::MeasurementComponent::new(&base_peripherals.tim3)
        .finalize(components::measurement_component_static!(stm32f4xx::tim3::Tim2_5));

    let scheduler = components::sched::round_robin::RoundRobinComponent::new(&PROCESSES)
        .finalize(components::round_robin_component_static!(NUM_PROCS));

    let nucleo_f429zi = NucleoF429ZI {
        console,
        ipc: kernel::ipc::IPC::new(
            board_kernel,
            kernel::ipc::DRIVER_NUM,
            &memory_allocation_capability,
        ),
        adc: adc_syscall,
        led,
        temperature: temp,
        button,
        alarm,
        gpio,
        rng,

        scheduler,
        systick: cortexm4::systick::SysTick::new_with_calibration(
            rcc.get_nominal_frequency_(&rcc::PeripheralClockType::AHB1(rcc::HCLK1::SysTick)),
        ),

        display,
        ipc_comm,
        pwm,
        measurement,
        can,
    };

    // // Optional kernel tests
    // //
    // // See comment in `boards/imix/src/main.rs`
    // virtual_uart_rx_test::run_virtual_uart_receive(mux_uart);
    debug!("Initialization complete. Entering main loop");

    // These symbols are defined in the linker script.
    extern "C" {
        /// Beginning of the ROM region containing app images.
        static _sapps: u8;
        /// End of the ROM region containing app images.
        static _eapps: u8;
        /// Beginning of the RAM region for app memory.
        static mut _sappmem: u8;
        /// End of the RAM region for app memory.
        static _eappmem: u8;
    }

    kernel::process::load_processes(
        board_kernel,
        chip,
        core::slice::from_raw_parts(
            &_sapps as *const u8,
            &_eapps as *const u8 as usize - &_sapps as *const u8 as usize,
        ),
        core::slice::from_raw_parts_mut(
            &mut _sappmem as *mut u8,
            &_eappmem as *const u8 as usize - &_sappmem as *const u8 as usize,
        ),
        &mut PROCESSES,
        &FAULT_RESPONSE,
        &process_management_capability,
    )
    .unwrap_or_else(|err| {
        debug!("Error loading processes!");
        debug!("{:?}", err);
    });

    //Uncomment to run multi alarm test
    /*components::test::multi_alarm_test::MultiAlarmTestComponent::new(mux_alarm)
    .finalize(components::multi_alarm_test_component_buf!(stm32f429zi::tim2::Tim2))
    .run();*/

    board_kernel.kernel_loop(
        &nucleo_f429zi,
        chip,
        Some(&nucleo_f429zi.ipc),
        &main_loop_capability,
    );
}