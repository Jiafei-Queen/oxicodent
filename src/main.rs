mod config_manager;

use crate::config_manager::*;

fn main() {
    print_logo();
    let config = Config::load_or_init();
}

fn print_logo() {
    println!("\n  .oooooo.                o8o                            .o8                            .   ");
    println!(" d8P'  `Y8b               `\"'                           \"888                          .o8   ");
    println!("888      888 oooo    ooo oooo   .ooooo.   .ooooo.   .oooo888   .ooooo.  ooo. .oo.   .o888oo ");
    println!("888      888  `88b..8P'  `888  d88' `\"Y8 d88' `88b d88' `888  d88' `88b `888P\"Y88b    888   ");
    println!("888      888    Y888'     888  888       888   888 888   888  888ooo888  888   888    888   ");
    println!("`88b    d88'  .o8\"'88b    888  888   .o8 888   888 888   888  888    .o  888   888    888 . ");
    println!(" `Y8bood8P'  o88'   888o o888o `Y8bod8P' `Y8bod8P' `Y8bod88P\" `Y8bod8P' o888o o888o   \"888\" ");
    println!("\t:: Oxicodent — A Light Coding Agent ::\t(v{})\n", env!("CARGO_PKG_VERSION"))
}