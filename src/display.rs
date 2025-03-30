use esp_idf_hal::{
    i2c::{I2cDriver},
    gpio::{OutputPin, PinDriver},
    delay::FreeRtos,
};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::{BinaryColor},
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};
use anyhow::Result;
use log::info;

// Constants to match Arduino code
const DISPLAY_ADDRESS: u8 = 0x3C;
pub const DISPLAY_I2C_FREQ: u32 = 500_000; // 500 kHz

pub struct Display {
    display: Ssd1306<I2CInterface<I2cDriver<'static>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>,
}

impl Display {
    pub fn new<RST>(i2c: I2cDriver<'static>, mut reset_pin: PinDriver<'static, RST, esp_idf_hal::gpio::Output>) -> Result<Self> 
    where 
        RST: OutputPin,
    {
        info!("Performing display reset sequence");
        // Reset sequence (matching typical OLED reset procedure)
        reset_pin.set_high()?;
        FreeRtos::delay_ms(1);
        reset_pin.set_low()?;
        FreeRtos::delay_ms(10);
        reset_pin.set_high()?;
        FreeRtos::delay_ms(20);
        
        info!("Creating display interface with address 0x{:02X}", DISPLAY_ADDRESS);
        // Create display interface with the exact address from Arduino code
        let interface = I2CDisplayInterface::new_custom_address(i2c, DISPLAY_ADDRESS);
        
        // Create display driver
        let mut display = Ssd1306::new(
            interface,
            DisplaySize128x64,
            DisplayRotation::Rotate0,
        ).into_buffered_graphics_mode();
        
        // Initialize with better error handling
        info!("Initializing display");
        display.init()
            .map_err(|e| anyhow::anyhow!("Failed to initialize display: {:?}", e))?;
        
        info!("Display initialized successfully");
        Ok(Self { display })
    }
    
    // Rest of the display implementation remains the same
    pub fn clear(&mut self) -> Result<()> {
        self.display.clear(BinaryColor::Off);
        Ok(())
    }
    
    pub fn draw_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: bool) -> Result<()> {
        let top_left = Point::new(x, y);
        let rect_color = if color { BinaryColor::On } else { BinaryColor::Off };
        let rect_style = PrimitiveStyle::with_stroke(rect_color, 1);
        
        Rectangle::new(top_left, Size::new(width as u32, height as u32))
            .into_styled(rect_style)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            
        Ok(())
    }
    
    pub fn fill_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: bool) -> Result<()> {
        let top_left = Point::new(x, y);
        let rect_color = if color { BinaryColor::On } else { BinaryColor::Off };
        let rect_style = PrimitiveStyle::with_fill(rect_color);
        
        Rectangle::new(top_left, Size::new(width as u32, height as u32))
            .into_styled(rect_style)
            .draw(&mut self.display)
            .map_err(|e| anyhow::anyhow!("Failed to draw rectangle: {:?}", e))?;
            
        Ok(())
    }

    pub fn draw_text(&mut self, x: i32, y: i32, text: &str, color: bool) -> Result<()> {
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
        .draw(&mut self.display)
        .map_err(|e| anyhow::anyhow!("Failed to draw text: {:?}", e))?;
        
        Ok(())
    }
    
    pub fn flush(&mut self) -> Result<()> {
        self.display.flush()
            .map_err(|e| anyhow::anyhow!("Failed to flush display: {:?}", e))?;
        Ok(())
    }
}