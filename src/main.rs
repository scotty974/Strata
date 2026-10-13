#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::error::Error;
use slint::{PhysicalSize, WindowSize};

mod utils;
slint::include_modules!();

fn main()-> Result<(), Box<dyn Error>>{

    let ui = Home::new()?;
    let window = ui.window();
   

    let window_size = WindowSize::Physical(PhysicalSize::new(600, 800));
    window.set_size(window_size);



    ui.run()?;

    Ok(())

}

