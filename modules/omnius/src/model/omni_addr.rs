use std::fmt;

use nom::bytes::complete::{is_not, tag};
use nom::character::complete::{char, multispace0};
use nom::sequence::delimited;
use nom::IResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OmniAddr {
    inner: String,
}

impl OmniAddr {
    pub fn new<S: AsRef<str> + ?Sized>(value: &S) -> OmniAddr {
        OmniAddr {
            inner: value.as_ref().to_string(),
        }
    }

    pub fn parse_tcp(&self) -> anyhow::Result<String> {
        let (_, addr) = Self::parse_tcp_sub(&self.inner).map_err(|e| e.to_owned())?;
        Ok(addr.to_string())
    }

    fn parse_tcp_sub(v: &str) -> IResult<&str, &str> {
        let (v, _) = tag("tcp")(v)?;
        let (v, addr) = delimited(char('('), delimited(multispace0, is_not(")"), multispace0), char(')'))(v)?;
        Ok((v, addr))
    }
}

impl fmt::Display for OmniAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl From<String> for OmniAddr {
    fn from(value: String) -> Self {
        Self::new(&value)
    }
}

#[cfg(test)]
mod tests {
    use crate::model::OmniAddr;

    #[tokio::test]
    #[ignore]
    async fn add_port_mapping_test() {
        let addr = OmniAddr::new("tcp(127.0.0.1:8000)");
        println!("{:?}", addr.parse_tcp());
    }
}
