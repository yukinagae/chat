use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use http_muncher::{Parser, ParserHandler};
use std::str;

#[derive(Debug)]
pub struct HttpParserHandler {
    pub current_key: Option<String>,
    pub headers: Rc<RefCell<HashMap<String, String>>>,
}

impl ParserHandler for HttpParserHandler {

    fn on_header_field(&mut self, parser: &mut Parser, s: &[u8]) -> bool {
        self.current_key = Some(str::from_utf8(s).unwrap().to_string());
        true
    }

    fn on_header_value(&mut self, parser: &mut Parser, s: &[u8]) -> bool {
        self.headers.borrow_mut().insert(
            self.current_key.clone().unwrap(),
            str::from_utf8(s).unwrap().to_string()
        );
        true
    }

    fn on_headers_complete(&mut self, parser: &mut Parser) -> bool {
        false
    }
}
