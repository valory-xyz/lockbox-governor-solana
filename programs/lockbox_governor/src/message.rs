use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use std::io;
use wormhole_io::{Readable, Writeable};

const PAYLOAD_TRANSFER: u8 = 0;
const PAYLOAD_TRANSFER_ALL: u8 = 1;
const PAYLOAD_TRANSFER_TOKEN_ACCOUNTS: u8 = 2;
const PAYLOAD_SET_UPGRADE_AUTHORITY: u8 = 3;
const PAYLOAD_UPGRADE_PROGRAM: u8 = 4;

#[derive(Clone)]
pub struct TransferMessage {
    pub source: [u8; 32],
    pub destination: [u8; 32],
    pub amount: u64
}

impl AnchorSerialize for TransferMessage {
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        PAYLOAD_TRANSFER.write(writer)?;
        self.source.write(writer)?;
        self.destination.write(writer)?;
        self.amount.write(writer)?;

        Ok(())
    }
}

impl AnchorDeserialize for TransferMessage {
    fn deserialize_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let selector = u8::read(reader)?;

        match selector {
            PAYLOAD_TRANSFER => Ok(TransferMessage {
                source: <[u8; 32]>::read(reader)?,
                destination: <[u8; 32]>::read(reader)?,
                amount: u64::read(reader)?,
            }),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid payload ID",
            )),
        }
    }
}

#[derive(Clone)]
pub struct TransferAllMessage {
    pub source_sol: [u8; 32],
    pub source_olas: [u8; 32],
    pub destination_sol: [u8; 32],
    pub destination_olas: [u8; 32]
}

impl AnchorSerialize for TransferAllMessage {
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        PAYLOAD_TRANSFER_ALL.write(writer)?;
        self.source_sol.write(writer)?;
        self.source_olas.write(writer)?;
        self.destination_sol.write(writer)?;
        self.destination_olas.write(writer)?;

        Ok(())
    }
}

impl AnchorDeserialize for TransferAllMessage {
    fn deserialize_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let selector = u8::read(reader)?;

        match selector {
            PAYLOAD_TRANSFER_ALL => Ok(TransferAllMessage {
                source_sol: <[u8; 32]>::read(reader)?,
                source_olas: <[u8; 32]>::read(reader)?,
                destination_sol: <[u8; 32]>::read(reader)?,
                destination_olas: <[u8; 32]>::read(reader)?
            }),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid payload ID",
            )),
        }
    }
}

#[derive(Clone)]
pub struct TransferTokenAccountsMessage {
    pub source_sol: [u8; 32],
    pub source_olas: [u8; 32],
    pub destination: [u8; 32]
}

impl AnchorSerialize for TransferTokenAccountsMessage {
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        PAYLOAD_TRANSFER_TOKEN_ACCOUNTS.write(writer)?;
        self.source_sol.write(writer)?;
        self.source_olas.write(writer)?;
        self.destination.write(writer)?;

        Ok(())
    }
}

impl AnchorDeserialize for TransferTokenAccountsMessage {
    fn deserialize_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let selector = u8::read(reader)?;

        match selector {
            PAYLOAD_TRANSFER_TOKEN_ACCOUNTS => Ok(TransferTokenAccountsMessage {
                source_sol: <[u8; 32]>::read(reader)?,
                source_olas: <[u8; 32]>::read(reader)?,
                destination: <[u8; 32]>::read(reader)?
            }),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid payload ID",
            )),
        }
    }
}

#[derive(Clone)]
pub struct SetUpgradeAuthorityMessage {
    pub program_id_bytes: [u8; 32],
    pub upgrade_authority: [u8; 32]
}

impl AnchorSerialize for SetUpgradeAuthorityMessage {
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        PAYLOAD_SET_UPGRADE_AUTHORITY.write(writer)?;
        self.program_id_bytes.write(writer)?;
        self.upgrade_authority.write(writer)?;

        Ok(())
    }
}

impl AnchorDeserialize for SetUpgradeAuthorityMessage {
    fn deserialize_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let selector = u8::read(reader)?;

        match selector {
            PAYLOAD_SET_UPGRADE_AUTHORITY => Ok(SetUpgradeAuthorityMessage {
                program_id_bytes: <[u8; 32]>::read(reader)?,
                upgrade_authority: <[u8; 32]>::read(reader)?
            }),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid payload ID",
            )),
        }
    }
}

#[derive(Clone)]
pub struct UpgradeProgramMessage {
    pub program_id_bytes: [u8; 32],
    pub buffer_account_bytes: [u8; 32],
    pub spill_account_bytes: [u8; 32]
}

impl AnchorSerialize for UpgradeProgramMessage {
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        PAYLOAD_UPGRADE_PROGRAM.write(writer)?;
        self.program_id_bytes.write(writer)?;
        self.buffer_account_bytes.write(writer)?;
        self.spill_account_bytes.write(writer)?;

        Ok(())
    }
}

impl AnchorDeserialize for UpgradeProgramMessage {
    fn deserialize_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let selector = u8::read(reader)?;

        match selector {
            PAYLOAD_UPGRADE_PROGRAM => Ok(UpgradeProgramMessage {
                program_id_bytes: <[u8; 32]>::read(reader)?,
                buffer_account_bytes: <[u8; 32]>::read(reader)?,
                spill_account_bytes: <[u8; 32]>::read(reader)?
            }),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid payload ID",
            )),
        }
    }
}
