// src/button.rs
use esp_idf_hal::{
    gpio::{AnyInputPin, Gpio0, Input, InterruptType, Level, PinDriver, Pull},
    prelude::*,
};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex,
    }
};
use esp_idf_svc::hal::delay::FreeRtos;

// We'll use GPIO0 as that's typically where the PRG button is connected
// on Heltec boards, but you might need to adjust this based on your board
const PRG_BUTTON_PIN: i32 = 0;

// Long press duration in milliseconds
const LONG_PRESS_DURATION_MS: u32 = 2000; // 1 second for long press

// Button states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonEvent {
    None,
    ShortPress,
    LongPress,
}

// Button state tracking
static BUTTON_PRESSED: AtomicBool = AtomicBool::new(false);
static BUTTON_RELEASED: AtomicBool = AtomicBool::new(false);
static PRESS_START_TIME: AtomicU32 = AtomicU32::new(0);
static PRESS_DURATION: AtomicU32 = AtomicU32::new(0);
static BUTTON_EVENT: Mutex<ButtonEvent> = Mutex::new(ButtonEvent::None);

type ButtonType = PinDriver<'static, Gpio0, Input>;

// Get current time in milliseconds
fn current_time_ms() -> u32 {
    let now = unsafe { esp_idf_svc::sys::esp_timer_get_time() } as u32;
    now / 1000 // Convert microseconds to milliseconds
}

// Button interrupt handler - simpler version that just sets flags
fn button_isr() {
    // When the interrupt fires, check the current time
    let current_time = current_time_ms();
    
    // If the button wasn't previously pressed, this is a new press
    if !BUTTON_PRESSED.load(Ordering::SeqCst) {
        BUTTON_PRESSED.store(true, Ordering::SeqCst);
        BUTTON_RELEASED.store(false, Ordering::SeqCst);
        PRESS_START_TIME.store(current_time, Ordering::SeqCst);
        log::debug!("Button pressed at {} ms", current_time);
    } else {
        // Button was already pressed, so this must be a release
        BUTTON_RELEASED.store(true, Ordering::SeqCst);
        
        // Calculate press duration
        let start_time = PRESS_START_TIME.load(Ordering::SeqCst);
        let duration = current_time.saturating_sub(start_time);
        PRESS_DURATION.store(duration, Ordering::SeqCst);
        
        log::debug!("Button released, duration: {} ms", duration);
        
        // Reset pressed flag
        BUTTON_PRESSED.store(false, Ordering::SeqCst);
        
        // Determine if it was a short or long press
        let mut event = BUTTON_EVENT.lock().unwrap();
        if duration >= LONG_PRESS_DURATION_MS {
            *event = ButtonEvent::LongPress;
        } else {
            *event = ButtonEvent::ShortPress;
        }
    }
}

// Function to check what type of button event occurred and reset the flag
pub fn check_button_event() -> ButtonEvent {
    let mut event = BUTTON_EVENT.lock().unwrap();
    let result = *event;
    if result != ButtonEvent::None {
        *event = ButtonEvent::None;
    }
    result
}

// Check if button is currently pressed (manual polling)
pub fn is_button_pressed(button: &ButtonType) -> bool {
    button.get_level() == Level::Low
}

// Update button state - call this in your main loop
pub fn update_button_state(button: &ButtonType) {
    // Check for button state changes manually (works with interrupt or polling)
    let is_pressed = is_button_pressed(button);
    let was_pressed = BUTTON_PRESSED.load(Ordering::SeqCst);
    let was_released = BUTTON_RELEASED.load(Ordering::SeqCst);
    
    // If state changed from not pressed to pressed
    if is_pressed && !was_pressed && !was_released {
        BUTTON_PRESSED.store(true, Ordering::SeqCst);
        BUTTON_RELEASED.store(false, Ordering::SeqCst);
        PRESS_START_TIME.store(current_time_ms(), Ordering::SeqCst);
        log::debug!("Button press detected (polling)");
    }
    
    // If state changed from pressed to not pressed
    if !is_pressed && was_pressed && !was_released {
        BUTTON_RELEASED.store(true, Ordering::SeqCst);
        
        // Calculate press duration
        let start_time = PRESS_START_TIME.load(Ordering::SeqCst);
        let current_time = current_time_ms();
        let duration = current_time.saturating_sub(start_time);
        PRESS_DURATION.store(duration, Ordering::SeqCst);
        
        log::debug!("Button release detected (polling), duration: {} ms", duration);
        
        // Reset pressed flag
        BUTTON_PRESSED.store(false, Ordering::SeqCst);
        
        // Determine if it was a short or long press
        let mut event = BUTTON_EVENT.lock().unwrap();
        if duration >= LONG_PRESS_DURATION_MS {
            *event = ButtonEvent::LongPress;
        } else {
            *event = ButtonEvent::ShortPress;
        }
    }
    
    // Long press detection while button is still held
    if is_pressed && was_pressed && !was_released {
        let start_time = PRESS_START_TIME.load(Ordering::SeqCst);
        let current_time = current_time_ms();
        let duration = current_time.saturating_sub(start_time);
        
        // If we've reached long press duration and haven't triggered it yet
        if duration >= LONG_PRESS_DURATION_MS {
            let mut event = BUTTON_EVENT.lock().unwrap();
            if *event == ButtonEvent::None {
                *event = ButtonEvent::LongPress;
                // Log the long press
                log::info!("Long press detected ({} ms)", duration);
                // Reset the pressed state so we don't trigger again
                BUTTON_RELEASED.store(true, Ordering::SeqCst);
                BUTTON_PRESSED.store(false, Ordering::SeqCst);
            }
        }
    }
    
    // Reset released flag after processing
    if was_released {
        BUTTON_RELEASED.store(false, Ordering::SeqCst);
    }
}

// Initialize the button with interrupt
pub fn init_button(gpio0: esp_idf_hal::gpio::Gpio0) -> anyhow::Result<ButtonType> {
    // Configure the pin as input with pull-up
    let mut button = PinDriver::input(gpio0)?;
    button.set_pull(Pull::Up)?;
    
    // Set up an interrupt for any edge (both press and release)
    button.set_interrupt_type(InterruptType::AnyEdge)?;
    
    // Subscribe to interrupts with simplified handler
    unsafe {
        button.subscribe(button_isr)?;
    }
    
    // Enable interrupts for this pin
    button.enable_interrupt()?;
    
    log::info!("PRG button initialized on GPIO{} with press/hold detection", PRG_BUTTON_PIN);
    
    Ok(button)
}