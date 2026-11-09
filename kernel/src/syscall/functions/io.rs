use crate::device::{
    key::KeyEvent,
    keyboard::keyboard_buffer,
    terminal::{self, terminal},
};

pub extern "C" fn sys_print(buffer: *const u8, len: usize) {
    let bytes = unsafe { core::slice::from_raw_parts(buffer, len) };
    let msg = core::str::from_utf8(bytes);

    let Ok(msg) = msg else {
        return;
    };

    let mut terminal = terminal().lock();
    terminal::print(&mut terminal, format_args!("{}", msg));
}

pub extern "C" fn sys_get_key_event(event: *mut KeyEvent) -> u64 {
    let evt = keyboard_buffer().pop_key_event();

    let Some(evt) = evt else {
        return 0;
    };

    unsafe {
        core::ptr::write(event, evt);
    };

    1
}
