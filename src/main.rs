use pholidota::Engine;

fn main() {
    let main_engine = Engine::new(); // create the Engine instance
    if let Err(e) = main_engine { // if there are any errors on the Engine init
        eprintln!("{}", e); // print the traceback
        return; // close the program early
    }

    let mut main_engine = main_engine.unwrap(); // safe unwrap
    if let Err(e) = main_engine.run() {         // run the engine main function
        eprintln!("{}", e); // print the traceback in case of errors
    }
} // program end
