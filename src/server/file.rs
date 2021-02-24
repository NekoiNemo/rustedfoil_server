use std::fs::Metadata;
use std::io;
use std::path::Path;
use std::pin::Pin;

use bb8_postgres::tokio_postgres::types::private::BytesMut;
use bytes::Bytes;
use futures::{ready, Stream};
use futures_util::task::Poll;
use headers::HeaderMapExt;
use tokio::fs::File;
use tokio_util::io::poll_read_buf;
use warp::http::Response;
use warp::hyper::Body;

pub async fn file_reply(path: &Path) -> Response<Body> {
    let file = File::open(path).await.expect("Failed to open file");
    let metadata = file.metadata().await.expect("Failed to get file metadata");

    let body = Body::wrap_stream(file_stream(file, &metadata));

    let mut response = Response::new(body);
    response.headers_mut().typed_insert(headers::ContentLength(metadata.len()));
    response.headers_mut().typed_insert(headers::ContentType::octet_stream());

    response
}

fn file_stream(
    mut file: File,
    metadata: &Metadata,
) -> impl Stream<Item=Result<Bytes, io::Error>> + Send {
    let buf_size = optimal_buf_size(&metadata);
    let mut buf = BytesMut::new();
    let mut len = metadata.len();

    futures_util::stream::poll_fn(move |cx| {
        if len == 0 {
            return Poll::Ready(None);
        }
        reserve_at_least(&mut buf, buf_size);

        let n = match ready!(poll_read_buf(Pin::new(&mut file), cx, &mut buf)) {
            Ok(n) => n as u64,
            Err(err) => {
                tracing::debug!("file read error: {}", err);
                return Poll::Ready(Some(Err(err)));
            }
        };

        if n == 0 {
            tracing::debug!("file read found EOF before expected length");
            return Poll::Ready(None);
        }

        let mut chunk = buf.split().freeze();
        if n > len {
            chunk = chunk.split_to(len as usize);
            len = 0;
        } else {
            len -= n;
        }

        Poll::Ready(Some(Ok(chunk)))
    })
}

fn reserve_at_least(buf: &mut BytesMut, cap: usize) {
    if buf.capacity() - buf.len() < cap {
        buf.reserve(cap);
    }
}

const DEFAULT_READ_BUF_SIZE: usize = 8_192;

fn optimal_buf_size(metadata: &Metadata) -> usize {
    std::cmp::min(DEFAULT_READ_BUF_SIZE as u64, metadata.len()) as usize
}
