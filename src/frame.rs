use std::{iter, io, u16};
use std::io::{Read, Write};
use std::error::Error;

use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};

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

impl WebSocketFrameHeader {

    fn new_header(len: usize, opcode: OpCode) -> Self {
        WebSocketFrameHeader {
            fin: true,
            rsv1: false, rsv2: false, rsv3: false,
            masked: false,
            opcode: opcode,
            payload_length: Self::determine_len(len),
        }
    }

    fn determine_len(len: usize) -> u8 {
        if len < (PAYLOAD_LEN_U16 as usize) {
            len as u8
        } else if len < (u16::MAX as usize) {
            PAYLOAD_LEN_U16
        } else {
            PAYLOAD_LEN_U64
        }
    }
}

#[derive(Debug)]
pub struct WebSocketFrame {
    header: WebSocketFrameHeader,
    mask: Option<[u8; 4]>,
    pub payload: Vec<u8>,
}

impl<'a> From<&'a str> for WebSocketFrame {
    fn from(payload: &str) -> WebSocketFrame {
        WebSocketFrame {
            header: WebSocketFrameHeader::new_header(payload.len(), OpCode::TextFrame),
            payload: Vec::from(payload),
            mask: None,
        }
    }
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

    // TODO: check it later
    fn serialize_header(hdr: &WebSocketFrameHeader) -> u16 {
        let b1 = ((hdr.fin as u8) << 7)
                  | ((hdr.rsv1 as u8) << 6)
                  | ((hdr.rsv2 as u8) << 5)
                  | ((hdr.rsv3 as u8) << 4)
                  | ((hdr.opcode as u8) & 0x0F);

        let b2 = ((hdr.masked as u8) << 7)
            | ((hdr.payload_length as u8) & 0x7F);

        ((b1 as u16) << 8) | (b2 as u16)
    }

    pub fn write(&self, output: &mut Write) -> io::Result<()> {
        let header = Self::serialize_header(&self.header);
        try!(output.write_u16::<BigEndian>(header));

        match self.header.payload_length {
            PAYLOAD_LEN_U16 => try!(output.write_u16::<BigEndian>(self.payload.len() as u16)),
            PAYLOAD_LEN_U64 => try!(output.write_u64::<BigEndian>(self.payload.len() as u64)),
            _ => {},
        }

        try!(output.write(&self.payload));
        Ok(())
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