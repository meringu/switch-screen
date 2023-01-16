use clap::Parser;
use windows::{Win32::{Devices::Display::SetDisplayConfig, Graphics::Gdi}};

#[repr(u32)]
#[derive(Parser, Debug)]
#[command(about)]
enum Topology {
    INTERNAL = Gdi::SDC_TOPOLOGY_INTERNAL,
    EXTERNAL = Gdi::SDC_TOPOLOGY_EXTERNAL,
    CLONE = Gdi::SDC_TOPOLOGY_CLONE,
    EXTEND = Gdi::SDC_TOPOLOGY_EXTEND,
    SUPPLIED = Gdi::SDC_TOPOLOGY_SUPPLIED,
}

fn main() -> Result<(), &'static str> {
    let topology = Topology::parse();

    let res = unsafe {
        SetDisplayConfig(None, None, Gdi::SDC_APPLY | topology as u32)
    };

    if res != 0 {
        return Err("An error occcured swtiching displays");
    }

    Ok(())
}
