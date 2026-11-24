//! The month shown by the calendar.

use chrono::{Datelike, Months, NaiveDate};

/// First day of the month of `date`.
pub fn month_start(date: NaiveDate) -> NaiveDate {
    date.with_day(1).unwrap_or(date)
}

/// The same day `n` months later (or earlier, for negative `n`), clamped to the month's length.
pub fn add_months(date: NaiveDate, n: i32) -> NaiveDate {
    let shifted = if n >= 0 {
        date.checked_add_months(Months::new(n.unsigned_abs()))
    } else {
        date.checked_sub_months(Months::new(n.unsigned_abs()))
    };
    shifted.unwrap_or(date)
}

/// The weeks of the month of `date`, Monday first: six rows of seven cells, `None` outside the
/// month. Six rows always, so the calendar does not change height between months.
pub fn month_grid(date: NaiveDate) -> [[Option<NaiveDate>; 7]; 6] {
    let first = month_start(date);
    let lead = first.weekday().num_days_from_monday();
    let mut grid = [[None; 7]; 6];
    let mut day = first;
    let mut cell = lead as usize;
    while day.month() == first.month() {
        if let Some(row) = grid.get_mut(cell / 7)
            && let Some(slot) = row.get_mut(cell % 7)
        {
            *slot = Some(day);
        }
        cell += 1;
        match day.succ_opt() {
            Some(next) => day = next,
            None => break,
        }
    }
    grid
}
