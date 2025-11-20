#[derive(Clone)]
pub enum FSM {
    Main,
    Menu,
    IpSelect(usize),
    PortSelect,
}
