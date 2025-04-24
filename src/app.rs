use std::sync::Mutex;

use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use log::info;

use crate::{button::ButtonEvent, display::{draw_rect, draw_text, flush_display, AppDisplay}};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InitMenuDisplayOptions {
    Scan,
    Dump,
    Size,
    Exit,
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
                    *state = InitMenuDisplayOptions::Size;
                },
                InitMenuDisplayOptions::Size => {
                    *state = InitMenuDisplayOptions::Exit;
                },
                InitMenuDisplayOptions::Exit => {
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
            draw_text(display, 5, 5, "-Scan", true)?;
            draw_text(display, 5, 15, "Dump", true)?;
            draw_text(display, 5, 25, "Size", true)?;
            draw_text(display, 5, 35, "Exit", true)?;
        },
        InitMenuDisplayOptions::Dump => {
            draw_text(display, 5, 5, "Scan", true)?;
            draw_text(display, 5, 15, "-Dump", true)?;
            draw_text(display, 5, 25, "Size", true)?;
            draw_text(display, 5, 35, "Exit", true)?;
        },
        InitMenuDisplayOptions::Size => {
            draw_text(display, 5, 5, "Scan", true)?;
            draw_text(display, 5, 15, "Dump", true)?;
            draw_text(display, 5, 25, "-Size", true)?;
            draw_text(display, 5, 35, "Exit", true)?;
        },
        InitMenuDisplayOptions::Exit => {
            draw_text(display, 5, 5, "Scan", true)?;
            draw_text(display, 5, 15, "Dump", true)?;
            draw_text(display, 5, 25, "Size", true)?;
            draw_text(display, 5, 35, "-Exit", true)?;
        },
    }
    flush_display(display)?;

    FreeRtos::delay_ms(1000);

    Ok(())
}
