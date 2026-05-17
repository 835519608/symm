mod dashboard;
mod detail;
mod footer;
mod sidebar;
mod top_bar;

pub use dashboard::show_dashboard;
pub use detail::show_detail;
pub use footer::show_footer;
pub use sidebar::show_sidebar;
pub use top_bar::{TopBarAction, show_top_bar};
