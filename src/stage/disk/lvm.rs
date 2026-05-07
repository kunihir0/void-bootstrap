use crate::types::VolumeManager;
use crate::ui::Ui;
use crate::util::command;
use anyhow::Result;

const VG_NAME: &str = VolumeManager::VG_NAME;
const LV_ROOT: &str = VolumeManager::LV_ROOT;

/// Set up LVM on the given partition, returning the root LV device path.
///
/// Creates a Physical Volume, a Volume Group (`vg_void`), and a single
/// Logical Volume (`lv_root`) spanning 100 % of the VG.
pub(crate) fn setup(ui: &Ui, partition: &str) -> Result<String> {
    ui.status(&format!("Creating Physical Volume on {partition}..."));
    command::run("pvcreate", &["-f", partition])?;

    ui.status(&format!("Creating Volume Group '{VG_NAME}'..."));
    command::run("vgcreate", &[VG_NAME, partition])?;

    ui.status(&format!(
        "Creating Logical Volume '{LV_ROOT}' (100% of VG)..."
    ));
    command::run("lvcreate", &["-l", "100%FREE", "-n", LV_ROOT, VG_NAME])?;

    let lv_path = format!("/dev/{VG_NAME}/{LV_ROOT}");
    ui.success(&format!("LVM ready — root device: {lv_path}"));

    Ok(lv_path)
}
