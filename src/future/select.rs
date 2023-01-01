use super::assert_future;
use core::pin::Pin;
use futures_core::future::{Future, FusedFuture};
use futures_core::task::{Context, Poll};
use crate::future::{Either, FutureExt};

/// Future for the [`select()`] function.
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[derive(Debug)]
pub struct Select<A, B> {
    inner: Option<(A, B)>,
}

impl<A: Unpin, B: Unpin> Unpin for Select<A, B> {}

/// Waits for either one of two differently-typed futures to complete.
///
/// This function will return a new future which awaits for either one of both
/// futures to complete. The returned future will finish with both the value
/// resolved and a future representing the completion of the other work.
///
/// Note that this function consumes the receiving futures and returns a
/// wrapped version of them.
///
/// Also note that if both this and the second future have the same
/// output type you can use the `Either::factor_first` method to
/// conveniently extract out the value at the end.
///
/// # Examples
///
/// A simple example
///
/// ```
/// # futures::executor::block_on(async {
/// use futures::{
///     pin_mut,
///     future::Either,
///     future::self,
/// };
/// 
/// // These two futures have different types even though their outputs have the same type.
/// let future1 = async {
///     future::pending::<()>().await; // will never finish
///     1
/// };
/// let future2 = async { 
///     future::ready(2).await
/// };
///
/// // 'select' requires Future + Unpin bounds
/// pin_mut!(future1);
/// pin_mut!(future2);
///
/// let value = match future::select(future1, future2).await {
///     Either::Left((value1, _)) => value1,  // `value1` is resolved from `future1`
///                                           // `_` represents `future2`
///     Either::Right((value2, _)) => value2, // `value2` is resolved from `future2`
///                                           // `_` represents `future1`
/// };
///
/// assert!(value == 2);
/// # });
/// ```
///
/// A more complex example
///
/// ```
/// use futures::future::{self, Either, Future, FutureExt};
///
/// // A poor-man's join implemented on top of select
///
/// fn join<A, B>(a: A, b: B) -> impl Future<Output=(A::Output, B::Output)>
///     where A: Future + Unpin,
///           B: Future + Unpin,
/// {
///     future::select(a, b).then(|either| {
///         match either {
///             Either::Left((x, b)) => b.map(move |y| (x, y)).left_future(),
///             Either::Right((y, a)) => a.map(move |x| (x, y)).right_future(),
///         }
///     })
/// }
/// ```
pub fn select<A, B>(future1: A, future2: B) -> Select<A, B>
    where A: Future + Unpin, B: Future + Unpin
{
    assert_future::<Either<(A::Output, B), (B::Output, A)>, _>(Select { inner: Some((future1, future2)) })
}

impl<A, B> Future for Select<A, B>
where
    A: Future + Unpin,
    B: Future + Unpin,
{
    type Output = Either<(A::Output, B), (B::Output, A)>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (mut a, mut b) = self.inner.take().expect("cannot poll Select twice");
        match a.poll_unpin(cx) {
            Poll::Ready(x) => Poll::Ready(Either::Left((x, b))),
            Poll::Pending => match b.poll_unpin(cx) {
                Poll::Ready(x) => Poll::Ready(Either::Right((x, a))),
                Poll::Pending => {
                    self.inner = Some((a, b));
                    Poll::Pending
                }
            }
        }
    }
}

impl<A, B> FusedFuture for Select<A, B>
where
    A: Future + Unpin,
    B: Future + Unpin,
{
    fn is_terminated(&self) -> bool {
        self.inner.is_none()
    }
}
