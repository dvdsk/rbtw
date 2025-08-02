use efibootnext::Adapter;

use crate::showln;

pub fn boot_num(adapter: &mut Adapter, boot_target: &str) -> Option<u16> {
    let mut efi_entries = adapter
        .load_options()
        .unwrap()
        .map(Result::unwrap)
        .filter(|o| {
            o.description
                .to_lowercase()
                .contains(&boot_target.to_lowercase())
        })
        .map(|o| o.number);

    let num = efi_entries.next()?;

    if efi_entries.next().is_some() {
        showln!("multiple {boot_target} efi boot loaders, starting first, exiting");
    }
    Some(num)
}
