use tap::pipe::Pipe;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio_serde::formats::SymmetricalBincode;
use tokio_serde::SymmetricallyFramed;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

pub type NetReader<Message, Transport> = SymmetricallyFramed<FramedRead<Transport, LengthDelimitedCodec>, Message, SymmetricalBincode<Message>>;
pub type NetWriter<Message, Transport> = SymmetricallyFramed<FramedWrite<Transport, LengthDelimitedCodec>, Message, SymmetricalBincode<Message>>;

pub fn net_reader<Message, T: AsyncRead>(read_stream: T) -> NetReader<Message, T> {
    let length_delimited =
        FramedRead::new(read_stream, LengthDelimitedCodec::new());
    tokio_serde::SymmetricallyFramed::new(length_delimited, SymmetricalBincode::<Message>::default())
}

pub fn net_writer<Message, T: AsyncWrite>(write_stream: T) -> NetWriter<Message, T> {
    let length_delimited = FramedWrite::new(write_stream, LengthDelimitedCodec::new());
    SymmetricallyFramed::new(length_delimited, SymmetricalBincode::<Message>::default())
}


pub fn into_framed_split<ReaderMessage, WriteMessage>(a: TcpStream) -> (NetReader<ReaderMessage, OwnedReadHalf>, NetWriter<WriteMessage, OwnedWriteHalf>) {
    a.into_split().pipe(|(rs, ws)| (net_reader(rs), net_writer(ws)))
}
