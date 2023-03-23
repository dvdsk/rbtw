use rustc_version::{version_meta, Channel};

fn main() {
    // Set cfg flags depending on release channel
    if let Ok(Channel::Nightly) = version_meta().map(|m| m.channel) {
        return;
    } else {
        panic!("\n*****************************\n \
               Need nightly to compile. \
               When installing from crates.io try using:\
               \n\t`cargo +nightly install rbtw`\
               \n******************************");
    }
}
