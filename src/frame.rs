use std::iter;
use std::io;
use std::io::Read;
use std::error::Error;

use byteorder::{ReadBytesExt, BigEndian};

const PAYLOAD_LEN_U16: u8 = 126;
const PAYLOAD_LEN_U64: u8 = 127;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpCode {
    TextFrame       = 1,  // 0001
    BinaryFrame     = 2,  // 0010
    ConnectionClose = 8,  // 1000
    Ping            = 9,  // 1001
    Pong            = 10, // 1010
}

impl OpCode {
    fn from(op: u8) -> Option<OpCode> {
        match op {
            1   => Some(OpCode::TextFrame),
            2   => Some(OpCode::BinaryFrame),
            8   => Some(OpCode::ConnectionClose),
            9   => Some(OpCode::TextFrame),
            0xA => Some(OpCode::Ping),
            _   => None,
        }
    }
}

#[derive(Debug)]
pub struct WebSocketFrameHeader {
    fin: bool,
    rsv1: bool,
    rsv2: bool,
    rsv3: bool,
    masked: bool,
    opcode: OpCode,
    payload_length: u8,
}

#[derive(Debug)]
pub struct WebSocketFrame {
    header: WebSocketFrameHeader,
    mask: Option<[u8; 4]>,
    pub payload: Vec<u8>,
}

impl WebSocketFrame {

    pub fn read(input: &mut Read) -> io::Result<WebSocketFrame> {
        let buf = try!(input.read_u16::<BigEndian>());
        let header = Self::parse_header(buf).unwrap();
        let len = try!(Self::read_length(header.payload_length, input));
        let mask_key = if header.masked {
            let mask = try!(Self::read_mask(input));
            Some(mask)
        } else {
            None
        };

        let mut payload = try!(Self::read_payload(len, input));

        if let Some(mask) = mask_key {
            Self::apply_mask(mask, &mut payload);
        }

        Ok(WebSocketFrame {
            header: header,
            payload: payload,
            mask: mask_key,
        })
    }

    pub fn get_opcode(&self) -> OpCode {
        self.header.opcode.clone()
    }

    fn parse_header(buf: u16) -> Result<WebSocketFrameHeader, String> {
        let opcode_num = ((buf >> 8) as u8) & 0x0F;
        let opcode = OpCode::from(opcode_num);

        if let Some(op) = opcode {
            Ok(WebSocketFrameHeader {
                fin:  (buf >> 8) & 0x80 == 0x80,
                rsv1: (buf >> 8) & 0x40 == 0x40,
                rsv2: (buf >> 8) & 0x20 == 0x20,
                rsv3: (buf >> 8) & 0x10 == 0x10,
                opcode: op,
                masked: buf & 0x80 == 0x80,
                payload_length: (buf as u8) & 0x7F,
                })
        } else {
            Err(format!("Invalid opcode: {}", opcode_num))
        }
    }

    fn apply_mask(mask: [u8; 4], bytes: &mut Vec<u8>) {
        for (idx, c) in bytes.iter_mut().enumerate() {
            *c = *c ^ mask[idx % 4];
        }
    }

    fn read_mask(input: &mut Read) -> io::Result<[u8; 4]> {
        let mut buf = [0; 4];
        try!(input.read(&mut buf));
        Ok(buf)
    }

    fn read_payload(payload_len: usize, input: &mut Read) -> io::Result<Vec<u8>> {
        let mut payload: Vec<u8> = Vec::with_capacity(payload_len);
        payload.extend(iter::repeat(0).take(payload_len)); //
        try!(input.read(&mut payload));
        Ok(payload)
    }

    fn read_length(payload_len: u8, input: &mut Read) -> io::Result<usize> {
        match payload_len {
            PAYLOAD_LEN_U64 => input.read_u64::<BigEndian>().map(|v| v as usize).map_err(From::from),
            PAYLOAD_LEN_U16 => input.read_u16::<BigEndian>().map(|v| v as usize).map_err(From::from),
            _ => Ok(payload_len as usize),
        }
    }
}

#[test]
fn name() {
    println!("{:b}", 0x80);
    println!("{:b}", 0x40);
    println!("{:b}", 0x20);
    println!("{:b}", 0x10);
    println!("{:b}", 0x7F);
}