use crate::constants::NORMAL;
use crate::constants::PARSE_ERROR;
use std::fmt::Write;
extern crate log;

use log::{Metadata, Record};
use std::sync::{Arc, Mutex};

pub(crate) struct BufferLogger {
    buffer: Mutex<String>,
}

impl log::Log for BufferLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut buffer = self.buffer.lock().unwrap();
            if buffer.len() > 0 {
                writeln!(&mut *buffer, "").unwrap();
            }
            write!(
                &mut *buffer,
                "{PARSE_ERROR}{}{NORMAL}: {}",
                record.level(),
                record.args()
            )
            .unwrap();
        }
    }

    fn flush(&self) {}
}

impl BufferLogger {
    pub(crate) fn get_logs(&self) -> String {
        let buffer = self.buffer.lock().unwrap();
        buffer.clone()
    }
}

pub(crate) fn init_logger() -> Result<Arc<BufferLogger>, log::SetLoggerError> {
    let logger = Arc::new(BufferLogger {
        buffer: Mutex::new(String::new()),
    });

    log::set_max_level(log::LevelFilter::Trace);
    log::set_boxed_logger(Box::new(logger.clone()))?;

    Ok(logger)
}
