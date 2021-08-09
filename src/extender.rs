use futures::{Future, Stream};
use pin_project_lite::pin_project;

use core::pin::Pin;
use core::task::{Context, Poll};

pin_project! {
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct StreamExtender<'c, A, B: ?Sized> {
        #[pin]
        stream: A,
        extendable: &'c mut B
    }
}

impl<'c, A: Stream, B> Future for StreamExtender<'c, A, B>
where
    A: Stream,
    <A as Stream>::Item: IntoIterator,
    B: Extend<<<A as Stream>::Item as IntoIterator>::Item>,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let this = self.as_mut().project();
            match this.stream.poll_next(context) {
                Poll::Ready(Some(item)) => {
                    this.extendable.extend(item);
                }
                Poll::Ready(None) => break Poll::Ready(()),
                Poll::Pending => break Poll::Pending,
            }
        }
    }
}

pub trait StreamExtendable<A>: Extend<A> {
    fn stream_extend<St>(&mut self, stream: St) -> StreamExtender<'_, St, Self> {
        StreamExtender {
            stream,
            extendable: self,
        }
    }
}

impl<T, A> StreamExtendable<A> for T where T: Extend<A> {}
