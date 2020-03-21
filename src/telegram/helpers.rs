use crate::telegram::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn build_inline_keyboard_markup(
    buttons: Vec<InlineKeyboardButton>,
    buttons_per_row: usize,
) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = vec![];
    let mut row: Vec<InlineKeyboardButton> = vec![];
    let mut buttons_iterator = buttons.into_iter();

    while let Some(button) = buttons_iterator.next() {
        row.push(button);
        if row.len() == buttons_per_row {
            rows.push(row.clone());
            row = vec![];
        }
    }

    if row.len() > 0 {
        rows.push(row);
    }

    InlineKeyboardMarkup {
        inline_keyboard: rows,
    }
}
