use zmq::{Context, Socket};
use flate2::read::GzDecoder;

use std::io::{Cursor, BufReader};

/// Helper function to connect to multiple subscriptions with ØMQ
pub fn subscribe(endpoint: &str, subscriptions: &[&[u8]]) -> Result<Socket, zmq::Error> {
    let socket = Context::new().socket(zmq::SUB)?;
    socket.connect(endpoint)?;

    for subscription in subscriptions {
        socket.set_subscribe(subscription)?;
    }

    Ok(socket)
}

/// Helper function to receive a Gzipped XML message through a ØMQ socket
pub fn receive(socket: &Socket) -> Result<BufReader<GzDecoder<Cursor<Vec<u8>>>>, Box<dyn std::error::Error>> {
    Ok(BufReader::new(GzDecoder::new(Cursor::new(socket.recv_multipart(0)?.into_iter().nth(1).ok_or("Multipart message not fully received")?))))
}