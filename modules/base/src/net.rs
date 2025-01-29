use std::net::{Ipv4Addr, Ipv6Addr};

pub trait Reachable {
    fn is_reachable(&self) -> bool;
}

impl Reachable for Ipv4Addr {
    fn is_reachable(&self) -> bool {
        !(self.is_private() || self.is_loopback() || self.is_link_local() || self.is_broadcast() || self.is_documentation() || self.is_unspecified())
    }
}

impl Reachable for Ipv6Addr {
    fn is_reachable(&self) -> bool {
        !(self.is_unspecified() || self.is_loopback() || self.is_unique_local() || self.is_unicast_link_local())
    }
}
