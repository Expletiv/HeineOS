use crate::library::input;

pub fn keyboard_demo() {
    println!("Keyboard Demo:");
    println!("Press keys on your keyboard. Press 'q' to exit the demo.");

    loop {
        let char = input::read_char();
        println!("{:?}", char);

        if 'q' == char {
            break;
        }
    }

    println!("Exiting keyboard demo.");
}