use std::fmt;

use chrono::Local;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;

const FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";

const FILE_FORMAT: &str = "%Y%m%d-%H%M%S%3f";

#[derive(Clone, Copy, Default)]
pub struct LocalTime;

impl FormatTime for LocalTime {
    fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
        write!(w, "{}", Local::now().format(FORMAT))
    }
}

pub fn now_stamp() -> String {
    Local::now().format(FORMAT).to_string()
}

pub fn now_file_stamp() -> String {
    Local::now().format(FILE_FORMAT).to_string()
}
