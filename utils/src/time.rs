use time::{format_description::FormatItem, macros::format_description};

pub const DATE_FORMAT: &[FormatItem<'_>] = format_description!("[year]-[month]-[day]");
