//! GUI 控件层：页面只组合参数，样式集中在此维护。

mod button;
mod form;
mod layout;
mod modal;
mod nav;
mod scroll;

pub use button::button;
pub use form::{PathBrowse, PathPickMode, path_control_row, path_field, search_field, text_field};
pub use layout::{
    card, detail_field, detail_path_field, form_page, right_aligned, settings_content_frame,
    split_row,
};
pub use modal::{
    ModalOptions, ModalSection, ModalSize, fill_ui_width, modal_scroll_vertical, show_modal,
};
pub use nav::settings_nav;
pub use scroll::vertical_when_overflow;
