#![allow(warnings)]
include!("../out/_.rs");

pub use prost::DecodeError as ProtobufDecodeError;
pub use prost::Message as Protobuf;

pub trait CmdID {
    const CMD_ID: u16;

    fn get_cmd_id(&self) -> u16 {
        Self::CMD_ID
    }
}

/// Response CmdID and message name (without CsReq/ScRsp) for a request CmdID, when
/// StarRail.proto defines both the request and its response.
pub fn response_for(request_id: u16) -> Option<(u16, &'static str)> {
    REQUEST_RESPONSE_PAIRS
        .binary_search_by_key(&request_id, |(request, _, _)| *request)
        .ok()
        .map(|i| (REQUEST_RESPONSE_PAIRS[i].1, REQUEST_RESPONSE_PAIRS[i].2))
}
