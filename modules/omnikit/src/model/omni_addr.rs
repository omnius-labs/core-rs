use std::{
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    str::FromStr,
};

use nom::{branch::*, bytes::complete::*, character::complete::*, combinator::*, multi::*, sequence::*, IResult, Parser};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OmniAddr {
    inner: String,
}

impl OmniAddr {
    pub fn new<S: AsRef<str> + ?Sized>(value: &S) -> OmniAddr {
        OmniAddr {
            inner: value.as_ref().to_string(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.inner
    }

    pub fn create_i2p<S: AsRef<str> + ?Sized>(value: &S) -> OmniAddr {
        Self::new(format!("i2p({})", value.as_ref()).as_str())
    }

    pub fn create_tcp(ip: IpAddr, port: u16) -> OmniAddr {
        match ip {
            IpAddr::V4(ip) => Self::new(format!("tcp(ip4({}),{})", ip, port).as_str()),
            IpAddr::V6(ip) => Self::new(format!("tcp(ip6({}),{})", ip, port).as_str()),
        }
    }

    pub fn create_tcp_dns<S: AsRef<str> + ?Sized>(value: &S, port: u16) -> OmniAddr {
        Self::new(format!("tcp(dns({}),{})", value.as_ref(), port).as_str())
    }

    pub fn parse_tcp_ip(&self) -> anyhow::Result<SocketAddr> {
        let (_, element) = StringParser::function_element_parser()(&self.inner).map_err(|e| e.to_owned())?;
        let addr = ElementParser::parse_tcp_ip(&element)?;
        Ok(addr)
    }

    pub fn parse_tcp_host(&self) -> anyhow::Result<(String, u16)> {
        let (_, element) = StringParser::function_element_parser()(&self.inner).map_err(|e| e.to_owned())?;
        let (ip, port) = ElementParser::parse_tcp_host(&element)?;
        Ok((ip, port))
    }
}

impl FromStr for OmniAddr {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(OmniAddr::new(s))
    }
}

impl fmt::Display for OmniAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[allow(unused)]
struct ElementParser;

#[allow(unused)]
impl ElementParser {
    pub fn parse_i2p(element: &Element) -> anyhow::Result<&str> {
        match element {
            Element::Function(f) => {
                if f.name == "i2p" {
                    if let [Element::Constant(v)] = f.args.as_slice() {
                        return Ok(v);
                    }
                }
                Err(anyhow::anyhow!("Invalid i2p element"))
            }
            _ => Err(anyhow::anyhow!("Invalid element")),
        }
    }

    pub fn parse_tcp_ip(element: &Element) -> anyhow::Result<SocketAddr> {
        match element {
            Element::Function(f) => {
                if f.name == "tcp" {
                    if let [Element::Function(f2), Element::Constant(v)] = f.args.as_slice() {
                        if let Ok(ip) = Self::parse_ip4(&Element::Function(f2.clone())) {
                            let port = v.parse::<u16>()?;
                            return Ok(SocketAddr::new(ip, port));
                        }

                        if let Ok(ip) = Self::parse_ip6(&Element::Function(f2.clone())) {
                            let port = v.parse::<u16>()?;
                            return Ok(SocketAddr::new(ip, port));
                        }
                    };
                }
                Err(anyhow::anyhow!("Invalid tcp element"))
            }
            _ => Err(anyhow::anyhow!("Invalid element")),
        }
    }

    pub fn parse_tcp_host(element: &Element) -> anyhow::Result<(String, u16)> {
        match element {
            Element::Function(f) => {
                if f.name == "tcp" {
                    if let [Element::Function(f2), Element::Constant(v)] = f.args.as_slice() {
                        if let Ok(ip) = Self::parse_ip4(&Element::Function(f2.clone())) {
                            let port = v.parse::<u16>()?;
                            return Ok((ip.to_string(), port));
                        }

                        if let Ok(ip) = Self::parse_ip6(&Element::Function(f2.clone())) {
                            let port = v.parse::<u16>()?;
                            return Ok((ip.to_string(), port));
                        }

                        if let Ok(host) = Self::parse_dns(&Element::Function(f2.clone())) {
                            let port = v.parse::<u16>()?;
                            return Ok((host, port));
                        }
                    };
                }
                Err(anyhow::anyhow!("Invalid tcp element"))
            }
            _ => Err(anyhow::anyhow!("Invalid element")),
        }
    }

    pub fn parse_ip4(element: &Element) -> anyhow::Result<IpAddr> {
        match element {
            Element::Function(f) => {
                if f.name == "ip4" {
                    if let [Element::Constant(v)] = f.args.as_slice() {
                        match v.parse::<Ipv4Addr>() {
                            Ok(addr) => {
                                return Ok(IpAddr::V4(addr));
                            }
                            Err(e) => return Err(anyhow::anyhow!("Failed to parse ip4 element: {}", e)),
                        }
                    }
                }
                Err(anyhow::anyhow!("Invalid ip4 element"))
            }
            _ => Err(anyhow::anyhow!("Invalid element")),
        }
    }

    pub fn parse_ip6(element: &Element) -> anyhow::Result<IpAddr> {
        match element {
            Element::Function(f) => {
                if f.name == "ip6" {
                    if let [Element::Constant(v)] = f.args.as_slice() {
                        match v.parse::<Ipv6Addr>() {
                            Ok(addr) => {
                                return Ok(IpAddr::V6(addr));
                            }
                            Err(e) => return Err(anyhow::anyhow!("Failed to parse ip6 element: {}", e)),
                        }
                    }
                }
                Err(anyhow::anyhow!("Invalid ip6 element"))
            }
            _ => Err(anyhow::anyhow!("Invalid element")),
        }
    }

    pub fn parse_dns(element: &Element) -> anyhow::Result<String> {
        match element {
            Element::Function(f) => {
                if f.name == "dns" {
                    if let [Element::Constant(v)] = f.args.as_slice() {
                        return Ok(v.clone());
                    }
                }
                Err(anyhow::anyhow!("Invalid dns element"))
            }
            _ => Err(anyhow::anyhow!("Invalid element")),
        }
    }
}

#[allow(unused)]
struct StringParser;

#[allow(unused)]
impl StringParser {
    pub fn string_literal_parser<'a>() -> impl FnMut(&'a str) -> IResult<&'a str, String> {
        move |input: &'a str| {
            let (input, parsed) = delimited(multispace0, many1(is_not(",()")), multispace0)(input)?;
            let result: String = parsed.into_iter().collect();
            Ok((input, result))
        }
    }

    pub fn quoted_string_literal_parser<'a>() -> impl FnMut(&'a str) -> IResult<&'a str, String> {
        move |input: &'a str| {
            let (input, _) = char('"')(input)?;
            let (input, fragments) =
                many0(map(preceded(char('\\'), anychar), |c| format!("\\{}", c)).or(map(is_not("\\\""), |s: &str| s.to_string())))(input)?;
            let (input, _) = char('"')(input)?;
            let result: String = fragments.concat().replace("\\\"", "\"");
            Ok((input, result))
        }
    }

    pub fn constant_element_parser<'a>() -> impl FnMut(&'a str) -> IResult<&'a str, Element> {
        move |input: &'a str| {
            let (input, text) = delimited(
                multispace0,
                alt((Self::quoted_string_literal_parser(), Self::string_literal_parser())),
                multispace0,
            )(input)?;
            let (input, _) = opt(delimited(multispace0, char(','), multispace0))(input)?;
            let result = Element::Constant(text);
            Ok((input, result))
        }
    }

    pub fn function_element_parser<'a>() -> impl FnMut(&'a str) -> IResult<&'a str, Element> {
        move |input: &'a str| {
            let (input, name) = delimited(multispace0, Self::string_literal_parser(), multispace0)(input)?;
            let (input, _) = delimited(multispace0, char('('), multispace0)(input)?;
            let (input, args) = many0(delimited(
                multispace0,
                alt((Self::function_element_parser(), Self::constant_element_parser())),
                multispace0,
            ))(input)?;
            let (input, _) = delimited(multispace0, char(')'), multispace0)(input)?;
            let (input, _) = opt(delimited(multispace0, char(','), multispace0))(input)?;
            let result = Element::Function(FunctionElement { name, args });
            Ok((input, result))
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum Element {
    Function(FunctionElement),
    Constant(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionElement {
    name: String,
    args: Vec<Element>,
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    #[tokio::test]
    async fn string_parser_test() -> TestResult {
        let (_, res) = StringParser::string_literal_parser()("abc, def, ghi")?;
        assert_eq!(res, "abc");

        let (_, res) = StringParser::quoted_string_literal_parser()("\"a,bc\\\", def\", ghi")?;
        assert_eq!(res, "a,bc\", def");

        let (_, res) = StringParser::constant_element_parser()("\"a,bc\\\", def\", ghi")?;
        if let Element::Constant(v) = res {
            assert_eq!(v, "a,bc\", def");
        }

        let (_, res) = StringParser::function_element_parser()("ghi(a,b)")?;
        if let Element::Function(f) = res {
            assert_eq!(f.name, "ghi");
            assert_eq!(f.args, vec![Element::Constant("a".to_string()), Element::Constant("b".to_string())]);
        }

        Ok(())
    }
}