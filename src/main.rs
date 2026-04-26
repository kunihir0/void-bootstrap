pub(crate) mod context;
pub(crate) mod stage;
pub(crate) mod types;
pub(crate) mod ui;
pub(crate) mod util;
pub(crate) mod validation;

use ui::Ui;

fn main() {
    let ui = Ui::new();

    if let Err(e) = stage::run_pipeline(&ui) {
        ui.error(&format!("{e:#}"));
        std::process::exit(1);
    }
}
