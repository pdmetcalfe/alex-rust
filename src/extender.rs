use futures::{future::FusedFuture, ready, stream::FusedStream, Future, Stream};
use pin_project_lite::pin_project;

use core::pin::Pin;
use core::task::{Context, Poll};

pin_project! {
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct StreamExtender<'dst, Dst: ?Sized, St> {
        #[pin]
        stream: St,
        extendable: &'dst mut Dst
    }
}

impl<'dst, Dst, St> Future for StreamExtender<'dst, Dst, St>
where
    St: Stream,
    <St as Stream>::Item: IntoIterator,
    Dst: Extend<<<St as Stream>::Item as IntoIterator>::Item>,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let this = self.as_mut().project();
            match ready!(this.stream.poll_next(context)) {
                Some(item) => {
                    this.extendable.extend(item);
                }
                None => break Poll::Ready(()),
            }
        }
    }
}

pub trait StreamExtendable<A>: Extend<A> {
    fn stream_extend<St>(&mut self, stream: St) -> StreamExtender<'_, Self, St> {
        StreamExtender {
            stream,
            extendable: self,
        }
    }
}

impl<T, A> StreamExtendable<A> for T where T: Extend<A> {}

impl<'dst, Dst, St> FusedFuture for StreamExtender<'dst, Dst, St>
where
    StreamExtender<'dst, Dst, St>: Future,
    St: FusedStream,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}
