//! DiaryCraft interface (egui): calendar, the page of the day, search.
#![forbid(unsafe_code)]

mod app;
mod folders;
mod i18n;
mod markdown;
mod views;

pub use app::DiaryCraftApp;

#[cfg(test)]
mod tests;
