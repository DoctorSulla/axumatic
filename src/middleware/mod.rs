use axum::{body::Body, extract::Request, response::Response};
use futures_util::future::BoxFuture;
use http::{header, HeaderValue};
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

use crate::AppState;

#[derive(Clone)]
struct MyLayer {
    state: Arc<AppState>,
}

impl<S> Layer<S> for MyLayer {
    type Service = ValidateSession<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ValidateSession {
            inner,
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
struct ValidateSession<S> {
    inner: S,
    state: Arc<AppState>,
}

impl<S> Service<Request> for ValidateSession<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    // `BoxFuture` is a type alias for `Pin<Box<dyn Future + Send + 'a>>`
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let future = self.inner.call(request);
        Box::pin(async move {
            let mut response: Response = future.await?;
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str("application/json").unwrap(),
            );

            Ok(response)
        })
    }
}
