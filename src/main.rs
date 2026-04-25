use anyhow::Result;

pub(crate) mod context;
pub(crate) mod stage;
pub(crate) mod types;
pub(crate) mod ui;
pub(crate) mod util;
pub(crate) mod validation;

use ui::Ui;

fn main() -> Result<()> {
    let ui = Ui::new();
    stage::run_pipeline(&ui)
}
