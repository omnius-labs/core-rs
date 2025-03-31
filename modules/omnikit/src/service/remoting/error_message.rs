use omnius_core_rocketpack::{Result as RocketPackResult, RocketMessage, RocketMessageReader, RocketMessageWriter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmniRemotingDefaultErrorMessage {
    pub typ: String,
    pub message: String,
    pub stack_trace: String,
}

impl RocketMessage for OmniRemotingDefaultErrorMessage {
    fn pack(writer: &mut RocketMessageWriter, value: &Self, _depth: u32) -> RocketPackResult<()> {
        writer.put_str(&value.typ);
        writer.put_str(&value.message);
        writer.put_str(&value.stack_trace);

        Ok(())
    }

    fn unpack(reader: &mut RocketMessageReader, _depth: u32) -> RocketPackResult<Self>
    where
        Self: Sized,
    {
        let typ = reader.get_string(1024)?;
        let message = reader.get_string(1024)?;
        let stack_trace = reader.get_string(1024)?;

        Ok(Self { typ, message, stack_trace })
    }
}

impl std::fmt::Display for OmniRemotingDefaultErrorMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "{}: {}", self.typ, self.message)?;
        write!(f, "{}", self.stack_trace)?;
        Ok(())
    }
}
