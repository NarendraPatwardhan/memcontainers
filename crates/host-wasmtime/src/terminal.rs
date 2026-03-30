use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io;

pub struct Terminal;

impl Terminal {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        Ok(Terminal)
    }

    pub fn read_key(&self) -> io::Result<Option<KeyEvent>> {
        if event::poll(std::time::Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                return Ok(Some(key));
            }
        }
        Ok(None)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}
