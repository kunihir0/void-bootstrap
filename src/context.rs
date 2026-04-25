use crate::types::FsType;

#[derive(Debug)]
pub(crate) struct InstallContext {
    pub root_part: String,
    pub efi_part: String,
    pub fs_type: FsType,
}
