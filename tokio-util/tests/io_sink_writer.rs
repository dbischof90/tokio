#![warn(rust_2018_idioms)]

use bytes::Bytes;
use futures_util::SinkExt;
use std::io::{self, Cursor, Error, ErrorKind};
use tokio::io::AsyncWriteExt;
use tokio_util::codec::{Encoder, FramedWrite, LinesCodec};
use tokio_util::io::{CopyToBytes, SinkWriter};
use tokio_util::sync::PollSender;

#[tokio::test]
async fn test_copied_sink_writer() -> Result<(), Error> {
    // Construct a channel pair to send data across and wrap a pollable sink.
    // Note that the sink must mimic a writable object, e.g. have `std::io::Error`
    // as its error type.
    // As `PollSender` requires an owned copy of the buffer, we wrap it additionally
    // with a `CopyToBytes` helper.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Bytes>(1);
    let mut writer = SinkWriter::new(CopyToBytes::new(
        PollSender::new(tx).sink_map_err(|_| io::Error::from(ErrorKind::BrokenPipe)),
    ));

    // Write data to our interface...
    let data: [u8; 4] = [1, 2, 3, 4];
    let _ = writer.write(&data).await;

    // ... and receive it.
    assert_eq!(data.to_vec(), rx.recv().await.unwrap());

    Ok(())
}

/// A trivial encoder.
struct SliceEncoder;

impl SliceEncoder {
    fn new() -> Self {
        Self {}
    }
}

impl<'a> Encoder<&'a [u8]> for SliceEncoder {
    type Error = Error;

    fn encode(&mut self, item: &'a [u8], dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        // We pretend there is something important going on here.
        dst.extend_from_slice(item);
        Ok(())
    }
}

#[tokio::test]
async fn test_direct_sink_writer() -> Result<(), Error> {
    // We define a framed writer which accepts bytes slices
    // and 'reverse' this construction immediately.
    let framed_byte_lc = FramedWrite::new(Vec::new(), SliceEncoder::new());
    let mut writer = SinkWriter::new(framed_byte_lc);

    // Write multiple slices to the sink...
    writer.write(&[1, 2, 3]).await;
    writer.write(&[4, 5, 6]).await;

    // ... and compare it with the buffer.
    assert_eq!(
        writer.into_inner().write_buffer().to_vec().as_slice(),
        &[1, 2, 3, 4, 5, 6]
    );

    Ok(())
}
