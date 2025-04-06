use std::sync::Mutex;

use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use log::info;

use crate::{button::ButtonEvent, display::{draw_rect, draw_text, flush_display, AppDisplay}};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InitMenuDisplayOptions {
    Scan,
    Dump,
}
pub static INIT_MENU_DISPLAY_STATE: Mutex<InitMenuDisplayOptions> = Mutex::new(InitMenuDisplayOptions::Scan);

pub fn update_initial_menu_state(button_event: &ButtonEvent) -> Result<()> {
    info!("{:?}", button_event);
    let mut state = INIT_MENU_DISPLAY_STATE.lock().map_err(|e| { anyhow::anyhow!("Mutex poisoned: {:?}", e)})?;

    match button_event {
        ButtonEvent::None => {},
        ButtonEvent::ShortPress => {
            match *state {
                InitMenuDisplayOptions::Scan => {
                    *state = InitMenuDisplayOptions::Dump;
                },
                InitMenuDisplayOptions::Dump => {
                    *state = InitMenuDisplayOptions::Scan;
                },
            }
        },
        ButtonEvent::LongPress => {},
    }
    Ok(())
}

pub fn render_initial_menu(display: &mut AppDisplay) -> Result<()> {
    let state = INIT_MENU_DISPLAY_STATE.lock().map_err(|e| { anyhow::anyhow!("Mutex poisoned: {:?}", e)})?;

    draw_rect(display, 0, 0, 128, 64, true)?;
    match *state {
        InitMenuDisplayOptions::Scan => {
            draw_text(display, 10, 10, "- Scan", true)?;
            draw_text(display, 10, 25, "Dump", true)?;
        },
        InitMenuDisplayOptions::Dump => {
            draw_text(display, 10, 10, "Scan", true)?;
            draw_text(display, 10, 25, "- Dump", true)?;
        },
    }
    flush_display(display)?;

    FreeRtos::delay_ms(1000);

    Ok(())
}
