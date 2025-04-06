use esp_idf_hal::{
    delay::FreeRtos, i2c::I2cDriver
};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, Ssd1306};
use anyhow::Result;

use crate::button::ButtonEvent;

// Constants to match Arduino code
pub const DISPLAY_ADDRESS: u8 = 0x3C;
pub const DISPLAY_I2C_FREQ: u32 = 10_000; // 10 kHz

pub type AppDisplay = Ssd1306<I2CInterface<I2cDriver<'static>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>;

// Rest of the display implementation remains the same
pub fn clear_display(display: &mut AppDisplay) -> Result<()> {
    display.clear(BinaryColor::Off).map_err(|e| anyhow::anyhow!("There was an error clearing the display: {:?}", e))?;
    Ok(())
}

pub fn draw_rect(display: &mut AppDisplay, x: i32, y: i32, width: i32, height: i32, color: bool) -> Result<()> {
    let top_left = Point::new(x, y);
    let rect_color = if color { BinaryColor::On } else { BinaryColor::Off };
    let rect_style = PrimitiveStyle::with_stroke(rect_color, 1);
    
    Rectangle::new(top_left, Size::new(width as u32, height as u32))
        .into_styled(rect_style)
        .draw(display)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        
    Ok(())
}

pub fn fill_rect(display: &mut AppDisplay, x: i32, y: i32, width: i32, height: i32, color: bool) -> Result<()> {
    let top_left = Point::new(x, y);
    let rect_color = if color { BinaryColor::On } else { BinaryColor::Off };
    let rect_style = PrimitiveStyle::with_fill(rect_color);
    
    Rectangle::new(top_left, Size::new(width as u32, height as u32))
        .into_styled(rect_style)
        .draw(display)
        .map_err(|e| anyhow::anyhow!("Failed to draw rectangle: {:?}", e))?;
        
    Ok(())
}

pub fn draw_text(display: &mut AppDisplay, x: i32, y: i32, text: &str, color: bool) -> Result<()> {
    let text_color = if color { BinaryColor::On } else { BinaryColor::Off };
    
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(text_color)
        .build();
        
    Text::with_baseline(
        text,
        Point::new(x, y),
        text_style,
        Baseline::Top,
    )
    .draw(display)
    .map_err(|e| anyhow::anyhow!("Failed to draw text: {:?}", e))?;
    
    Ok(())
}

pub fn flush_display(display: &mut AppDisplay) -> Result<()> {
    display.flush().map_err(|e| anyhow::anyhow!("Failed to flush display: {:?}", e))?;
    FreeRtos::delay_ms(10);
    Ok(())
}

pub fn draw_start_up(display: &mut AppDisplay) -> Result<()> {
    clear_display(display)?;
    flush_display(display)?;
    FreeRtos::delay_ms(1000);
    Ok(())
}

pub fn draw_status_update(display: &mut AppDisplay, durration: &u64, total_count: &usize, button_event: &ButtonEvent) -> Result<()> {
    // Update display with current status
    clear_display(display)?;
    draw_text(display, 10, 10, &format!("Time left: {}s", durration), true)?;
    draw_text(display, 10, 30, &format!("MACs found: {}", total_count), true)?;
    flush_display(display)?;

    Ok(())
}

pub fn draw_final_count(display: &mut AppDisplay, count: &usize) -> Result<()> {
    // Show final count on display
    clear_display(display)?;
    draw_text(display, 10, 10, &format!("Found {} MACs", count), true)?;
    flush_display(display)?;
    FreeRtos::delay_ms(5000); // Show the result for 5 seconds

    Ok(())
}
