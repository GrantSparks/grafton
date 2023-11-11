mod util;
mod model;

use util::config::Config;

fn main() {
    let config = Config::load("./config");
    println!("Hello, world! {:?}", config);
}
