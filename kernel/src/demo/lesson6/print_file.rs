use alloc::string::String;
use crate::filesystem::tarfs;

pub fn print_file() {
    let fs = tarfs::filesystem();
    let handle = fs.open("hello_world.txt").unwrap();

    let mut buffer = [0; 512];
    let _ = fs.read(handle, &mut buffer).unwrap();

    println!("{}", String::from_utf8_lossy(&buffer));

    fs.close(handle);
}