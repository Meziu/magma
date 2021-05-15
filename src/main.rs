use std::process;

use sdlfun::engine::Engine;

fn main() {
    let main_engine = Engine::new();
    if let Err(e) = main_engine {
        eprintln!("Engine init panic: {}", e);
        process::exit(1);
    }

    let mut main_engine = main_engine.unwrap(); // safe unwrap
    if let Err(e) = main_engine.run() {         // run the engine main function
        eprintln!("Engine runtime panic: {}", e);
        process::exit(1);
    }
}
