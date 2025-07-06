use tokio::net::TcpStream;
use tokio_serde::formats::SymmetricalJson;
use tokio_serde::{Framed as SerdeFramed};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use crate::{Message, StateMachine};

pub type JsonFrame<S> = SerdeFramed<
    Framed<TcpStream, LengthDelimitedCodec>,
    Message<S>,
    Message<S>,
    SymmetricalJson<Message<S>>,
>;

pub fn framed_stream<S: StateMachine>(stream: TcpStream) -> JsonFrame<S> {
    let length_codec = LengthDelimitedCodec::builder()
        .length_field_type::<u32>()
        .new_codec();

    let framed = Framed::new(stream, length_codec);
    SerdeFramed::new(framed, SymmetricalJson::<Message<S>>::default())
}
