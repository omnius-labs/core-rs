use crate::prelude::*;

/// 固定 schema の入力を先に浅い CBOR へ制限し、未知 field の再帰 skip を防ぐ。
pub(super) struct HandshakePayload<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> HandshakePayload<'a> {
    pub fn validate(bytes: &'a [u8]) -> Result<()> {
        let mut parser = Self { bytes, offset: 0 };
        parser.value(0)?;
        if parser.offset != bytes.len() {
            return Err(Self::invalid());
        }
        Ok(())
    }

    fn value(&mut self, depth: usize) -> Result<()> {
        if depth > 4 {
            return Err(Self::invalid());
        }
        let head = *self.bytes.get(self.offset).ok_or_else(Self::invalid)?;
        self.offset += 1;
        let length = self.argument(head & 31)?;
        match head >> 5 {
            0 => Ok(()),
            2 | 3 => {
                let length = usize::try_from(length).map_err(|_| Self::invalid())?;
                let end = self.offset.checked_add(length).filter(|end| *end <= self.bytes.len()).ok_or_else(Self::invalid)?;
                self.offset = end;
                Ok(())
            }
            5 if length <= 8 => {
                for _ in 0..length {
                    self.value(depth + 1)?;
                    self.value(depth + 1)?;
                }
                Ok(())
            }
            _ => Err(Self::invalid()),
        }
    }

    fn argument(&mut self, additional: u8) -> Result<u64> {
        if additional < 24 {
            return Ok(additional as u64);
        }
        let length = match additional {
            24 => 1,
            25 => 2,
            26 => 4,
            27 => 8,
            _ => return Err(Self::invalid()),
        };
        let end = self.offset.checked_add(length).filter(|end| *end <= self.bytes.len()).ok_or_else(Self::invalid)?;
        let mut value = 0u64;
        for byte in &self.bytes[self.offset..end] {
            value = value << 8 | *byte as u64;
        }
        self.offset = end;
        Ok(value)
    }

    fn invalid() -> Error {
        Error::new(ErrorKind::InvalidFormat).with_message("invalid V2 handshake payload structure")
    }
}
