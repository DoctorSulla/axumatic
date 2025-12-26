use axum::{
    extract::Request,
    response::{IntoResponse, Response},
};
use futures_util::future::BoxFuture;
use http::HeaderValue;
use std::{
    sync::Arc,
    task::{Context, Poll},
};
use tower::{Layer, Service};
use tracing::{Level, event};

use crate::{AppState, auth::validate_cookie};

#[derive(Clone)]
pub struct ValidateSessionLayer {
    pub state: Arc<AppState>,
}

impl ValidateSessionLayer {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

impl<S> Layer<S> for ValidateSessionLayer {
    type Service = ValidateSession<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ValidateSession {
            inner,
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
pub struct ValidateSession<S> {
    pub inner: S,
    pub state: Arc<AppState>,
}

impl<S> Service<Request> for ValidateSession<S>
where
    S: Service<Request, Response = Response> + Send + 'static + Clone,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    // `BoxFuture` is a type alias for `Pin<Box<dyn Future + Send + 'a>>`
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request) -> Self::Future {
        let mut inner = self.inner.clone();
        let state = self.state.clone();

        Box::pin(async move {
            let response: Response = match validate_cookie(request.headers(), state).await {
                Ok(email) => {
                    request
                        .headers_mut()
                        .insert("email", HeaderValue::from_str(email.0.as_str()).unwrap());

                    let future = inner.call(request);
                    future.await?
                }
                _ => {
                    event!(
                        Level::WARN,
                        "Attempt to access protected route without valid session"
                    );
                    http::StatusCode::UNAUTHORIZED.into_response()
                }
            };
            Ok(response)
        })
    }
}
