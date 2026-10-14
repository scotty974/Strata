#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::error::Error;

use slint::{PhysicalSize, WindowSize};

mod bindings;
mod core;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    let app = Home::new()?;
    app.window()
        .set_size(WindowSize::Physical(PhysicalSize::new(1200, 800)));

    bindings::setup(&app);

    app.run()?;
    Ok(())
}
