mod app;
mod auth;
mod model;
mod util;

use app::start;
use util::Config;

fn main() {
    match Config::load("./config") {
        Ok(config) => match start(config) {
            Ok(_) => println!("Application started successfully"),
            Err(e) => eprintln!("Failed to start application: {}", e),
        },
        Err(e) => eprintln!("Failed to load config: {}", e),
    }
}
