use alloc::string::String;
use crate::device::terminal::framebuffer;
use crate::filesystem::tarfs;
use crate::library::bitmap::Bitmap;

pub fn print_file() {
    let fs = tarfs::filesystem();
    let handle = fs.open("hello_world.txt").unwrap();

    let mut buffer = [0; 512];
    let _ = fs.read(handle, &mut buffer).unwrap();

    println!("{}", String::from_utf8_lossy(&buffer));

    fs.close(handle);
}

pub fn print_bitmap() {
    let bitmap = Bitmap::read_from_file("heine.bmp");

    match bitmap {
        Ok(Some(bitmap)) => {
            println!("Bitmap width: {}", bitmap.width());
            println!("Bitmap height: {}", bitmap.height());

            framebuffer().lock().draw_bitmap(&bitmap, 128, 128);
        },
        Ok(None) => println!("Invalid or unsupported BMP file"),
        Err(_) => println!("Error reading BMP file"),
    }
}