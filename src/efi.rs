use efibootnext::Adapter;

fn windows_num(adapter: &mut Adapter) -> u16 {
    let mut windows_entries = adapter
        .load_options()
        .map(Result::unwrap)
        .filter(|o| o.description.contains("Windows"))
        .map(|o| o.number);

    let num = windows_entries.next().unwrap();

    assert!(
        windows_entries.next().is_none(),
        "multiple windows installs/boot loaders do not know which to boot too"
    );
    num
}

pub fn set_next_boot() {
    let mut adapter = Adapter::default();
    let windows = windows_num(&mut adapter);
    adapter.set_boot_next(windows).unwrap();
}
